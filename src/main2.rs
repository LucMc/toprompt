use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

struct Config {
    use_gitignore: bool,
    verbose: bool,
    recursive: bool,
    paths: Vec<String>,
}

fn print_usage_and_exit() {
    eprintln!(
        "Usage: {} [-r] [-i] [-v] <file1|dir1> [file2|dir2] ...",
        env::args().next().unwrap_or_else(|| "toprompt".to_string())
    );
    eprintln!("  -r     Process directories recursively");
    eprintln!("  -i     Use .gitignore files to exclude files/directories");
    eprintln!("  -v     Verbose output (show ignored files)");
    eprintln!("\n## Advanced options examples:");
    eprintln!("  {} *.py # wildcards/ regex for specific files (shell expanded)", env::args().next().unwrap_or_else(|| "toprompt".to_string()));
    eprintln!("  {} . # Copy all files in current/specified folder (non-recursive)", env::args().next().unwrap_or_else(|| "toprompt".to_string()));
    eprintln!("  {} -r . # Copy all files in current/specified folder and subfolders recursively", env::args().next().unwrap_or_else(|| "toprompt".to_string()));
    eprintln!("  {} -i . # Use .gitignore to not copy exclude specified files from copying (non-recursive for dir)", env::args().next().unwrap_or_else(|| "toprompt".to_string()));
    eprintln!("  {} -ri . # Use .gitignore and recurse through subfolders", env::args().next().unwrap_or_else(|| "toprompt".to_string()));
    std::process::exit(1);
}

fn main() {
    let config = parse_args();

    if config.paths.is_empty() {
        print_usage_and_exit();
    }

    let mut formatted_content = String::new();
    let mut successful_files = 0;
    let mut file_index = 0;

    for path_str in config.paths.iter() {
        match process_path(path_str, &mut formatted_content, &mut file_index, &mut successful_files, &config) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Error processing path '{}': {}", path_str, e);
            }
        }
    }

    if successful_files == 0 {
        eprintln!("No files were successfully processed.");
        return;
    }

    match copy_to_clipboard(&formatted_content) {
        Ok(_) => {
            println!("\nSuccessfully copied {} file(s) to clipboard!", successful_files);
            if config.use_gitignore {
                println!("(.gitignore rules were applied)");
            }
            if config.recursive {
                println!("(Processed directories recursively)");
            } else if config.paths.iter().any(|p| Path::new(p).is_dir()) {
                 println!("(Processed directories non-recursively)");
            }
            println!("\n--- Clipboard Contents Preview (first 500 chars) ---\n");
            let preview = if formatted_content.len() > 500 {
                &formatted_content[..500]
            } else {
                &formatted_content
            };
            println!("{}...", preview);
        }
        Err(e) => {
            eprintln!("Failed to copy to clipboard: {}", e);
            println!("\n--- Output (not copied) ---\n");
            println!("{}", formatted_content);
        }
    }
}

fn parse_args() -> Config {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut config = Config {
        use_gitignore: false,
        verbose: false,
        recursive: false,
        paths: Vec::new(),
    };

    for arg in args {
        if arg.starts_with('-') && arg.len() > 1 {
            for flag_char in arg.chars().skip(1) {
                match flag_char {
                    'i' => config.use_gitignore = true,
                    'v' => config.verbose = true,
                    'r' => config.recursive = true,
                    _ => {
                        eprintln!("Unknown flag character: '{}' in argument '{}'", flag_char, arg);
                        print_usage_and_exit();
                    }
                }
            }
        } else if arg == "-" {
             eprintln!("Reading from stdin via '-' is not supported.");
             print_usage_and_exit();
        }
        else {
            config.paths.push(arg);
        }
    }
    config
}

fn process_path(
    path_str: &str,
    formatted_content: &mut String,
    file_index: &mut usize,
    successful_files: &mut usize,
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path_str);

    if !path.exists() {
        return Err(format!("Path '{}' does not exist or is not accessible.", path.display()).into());
    }

    if path.is_file() {
        let filename_str = match path.to_str() {
            Some(s) => s,
            None => {
                // Log error and skip if path is not valid UTF-8
                eprintln!("Warning: Skipping non-UTF8 file path: {}", path.display());
                return Ok(()); // Successfully skipped
            }
        };
        match process_file(filename_str) {
            Ok(content) => {
                if *file_index > 0 {
                    formatted_content.push_str("\n\n");
                }
                formatted_content.push_str(&content);
                *successful_files += 1;
                *file_index += 1;
            }
            Err(e) => return Err(Box::new(e)), // Propagate error
        }
    } else if path.is_dir() {
        let gitignore = if config.use_gitignore {
            let mut gitignore = GitIgnore::with_defaults(path);
            let loaded = load_gitignore(path, config);
            gitignore.merge(loaded);
            gitignore
        } else {
            GitIgnore::empty()
        };
        process_directory(path, path, formatted_content, file_index, successful_files, config, &gitignore)?;
    } else {
        return Err(format!("'{}' is neither a file nor a directory", path.display()).into());
    }
    Ok(())
}

fn process_directory(
    dir: &Path,
    base_dir: &Path,
    formatted_content: &mut String,
    file_index: &mut usize,
    successful_files: &mut usize,
    config: &Config,
    parent_gitignore: &GitIgnore,
) -> Result<(), Box<dyn std::error::Error>> {
    if config.use_gitignore {
        let relative_path_to_base = dir.strip_prefix(base_dir).unwrap_or(dir);
        if !relative_path_to_base.as_os_str().is_empty() && relative_path_to_base.components().next().is_some() {
            if parent_gitignore.should_ignore(&relative_path_to_base, true) {
                if config.verbose {
                    println!("Ignoring directory (due to parent rules): {}", dir.display());
                }
                return Ok(());
            }
        }
    }

    let mut current_gitignore = parent_gitignore.clone();
    if config.use_gitignore {
        let local_gitignore_path = dir.join(".gitignore");
        if local_gitignore_path.exists() {
            let new_gitignore = load_gitignore(dir, config); // Pass dir as the location of .gitignore
            current_gitignore.merge(new_gitignore);
            if config.verbose {
                println!("Loaded .gitignore from: {}", dir.join(".gitignore").display());
            }
        }
    }

    let mut entries: Vec<_> = match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(e) => {
            eprintln!("Warning: Could not read directory {}: {}", dir.display(), e);
            return Ok(()); // Continue if a directory cannot be read
        }
    };

    entries.sort_by_key(|e| e.path());

    let filtered_entries: Vec<_> = entries.into_iter()
        .filter(|entry| {
            if !config.use_gitignore {
                return true;
            }
            let path = entry.path();
            let relative_path_to_base = path.strip_prefix(base_dir).unwrap_or(&path);
            let should_ignore = current_gitignore.should_ignore(&relative_path_to_base, path.is_dir());

            if config.verbose && should_ignore {
                println!("Ignoring: {}", relative_path_to_base.display());
            }
            !should_ignore
        })
        .collect();

    for entry in filtered_entries {
        let path = entry.path();
        if path.is_file() {
            let filename_str = match path.to_str() {
                Some(s) => s,
                None => {
                    eprintln!("Warning: Skipping non-UTF8 file path: {}", path.display());
                    continue; // Skip this file and continue with the next
                }
            };
            match process_file(filename_str) {
                Ok(content) => {
                    if *file_index > 0 {
                        formatted_content.push_str("\n\n");
                    }
                    formatted_content.push_str(&content);
                    *successful_files += 1;
                    *file_index += 1;
                }
                Err(e) => {
                    eprintln!("Error processing file '{}': {}", path.display(), e);
                }
            }
        } else if path.is_dir() {
            if config.recursive {
                if config.verbose {
                    println!("Recursively processing directory: {}", path.display());
                }
                process_directory(&path, base_dir, formatted_content, file_index, successful_files, config, &current_gitignore)?;
            } else {
                if config.verbose {
                    println!("Skipping subdirectory (non-recursive mode): {}", path.display());
                }
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct GitIgnore {
    patterns: Vec<GitIgnorePattern>,
}

#[derive(Clone)]
struct GitIgnorePattern {
    pattern: String,
    // original_pattern: String, // Kept for potential future use, not strictly needed now
    is_negation: bool,
    is_directory: bool,
    is_absolute: bool,
   // _gitignore_location_dir: PathBuf, // Store the location of the .gitignore file for this pattern
}


impl GitIgnore {
    fn empty() -> Self {
        GitIgnore { patterns: Vec::new() }
    }

    fn with_defaults(_operation_base_dir: &Path) -> Self {
        let mut patterns = Vec::new();
        // For simplicity, GitIgnorePattern::new now takes the pattern string directly
        // The context (like _gitignore_location_dir) would be needed for full gitignore spec.
        patterns.push(GitIgnorePattern::new(".git/".to_string()));
        patterns.push(GitIgnorePattern::new(".gitignore".to_string()));
        GitIgnore { patterns }
    }

    fn merge(&mut self, other: GitIgnore) {
        self.patterns.extend(other.patterns);
    }

    fn should_ignore(&self, path_to_check: &Path, is_dir: bool) -> bool {
        let path_str = path_to_check.to_string_lossy().replace('\\', "/");
        let mut ignored = false;
        for pattern_obj in &self.patterns {
            if pattern_obj.matches(&path_str, is_dir, path_to_check) {
                ignored = !pattern_obj.is_negation;
            }
        }
        ignored
    }
}

impl GitIgnorePattern {
    fn new(pattern_line: String) -> Self {
        // let original_pattern = pattern_line.clone(); // Keep original if needed for debugging
        let mut p = pattern_line.trim().to_string();

        let is_negation = p.starts_with('!');
        if is_negation {
            p = p[1..].to_string();
        }
        
        let is_absolute = p.starts_with('/');
        if is_absolute {
            p = p[1..].to_string();
        }
        
        let is_directory = p.ends_with('/');
        if is_directory {
            p = p[..p.len()-1].to_string();
        }

        GitIgnorePattern {
            pattern: p,
            // original_pattern,
            is_negation,
            is_directory,
            is_absolute,
            // _gitignore_location_dir: gitignore_location_dir.to_path_buf(),
        }
    }

    fn matches(&self, path_to_check_str: &str, entry_is_dir: bool, _path_to_check_obj: &Path) -> bool {
        if self.is_directory && !entry_is_dir {
            return false;
        }

        let pattern_to_match = &self.pattern;

        if self.is_absolute {
            return self.simple_glob_match(pattern_to_match, path_to_check_str);
        }
        
        if pattern_to_match.contains('/') {
            // Simplified: if pattern has '/', it must match end of path_to_check_str or full path_to_check_str
            // This is still not fully git-compliant for nested gitignores but better than just filename.
            if self.simple_glob_match(pattern_to_match, path_to_check_str) {
                return true;
            }
            // Check if path_to_check_str ends with the pattern, and the last component matches.
            // This is a heuristic. e.g. pattern "foo/bar" and path "some/foo/bar"
            if path_to_check_str.ends_with(pattern_to_match) {
                 // Ensure it's not just a suffix of a component, e.g. pattern "dir/file" path "otherdir/file"
                 // This requires more careful checking if not using a full glob.
                 // For now, let simple_glob_match handle the anchoring if it can.
                 return self.simple_glob_match(pattern_to_match,path_to_check_str); // Re-check with glob which might handle this better
            }
        } else {
            // No "/" in pattern: matches filename in any directory.
            let filename = Path::new(path_to_check_str)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if self.simple_glob_match(pattern_to_match, filename) {
                return true;
            }
            // A pattern like "foo" (no slash) can also match a directory name "foo" anywhere in the path
            // if the entry itself is a directory.
            if entry_is_dir && path_to_check_str.split('/').any(|part| self.simple_glob_match(pattern_to_match, part)) {
                return true;
            }
        }
        // Fallback for patterns without '/' to match any component
        if !pattern_to_match.contains('/') {
             return path_to_check_str.split('/').any(|part| self.simple_glob_match(pattern_to_match, part));
        }
        false
    }
    
    // Rewritten simple_glob_match for basic '*' and '?' handling
    fn simple_glob_match(&self, pattern: &str, text: &str) -> bool {
        let mut pat_chars = pattern.chars().peekable();
        let mut text_chars = text.chars().peekable();

        loop {
            match (pat_chars.peek(), text_chars.peek()) {
                (Some(&'*'), _) => {
                    pat_chars.next(); // Consume the '*'
                    if pat_chars.peek().is_none() {
                        return true; // '*' at end of pattern matches rest of text
                    }
                    // Try to match the rest of the pattern with all suffixes of the text
                    let mut temp_text_chars = text_chars.clone(); // Clone to backtrack
                    loop {
                        // Pass a slice of the pattern and text to a recursive call or iterative equivalent
                        // For simplicity here, we'll use a loop and advance text_chars for '*'
                        // Create temporary iterators for the recursive-like match
                        let sub_pattern: String = pat_chars.clone().collect();
                        let sub_text: String = temp_text_chars.clone().collect();

                        if self.simple_glob_match(&sub_pattern, &sub_text) {
                            return true;
                        }
                        if temp_text_chars.next().is_none() {
                            break; // No more text to skip for '*'
                        }
                    }
                    return false; // '*' couldn't match
                }
                (Some(&'?'), None) => return false, // '?' needs a character
                (Some(&'?'), Some(_)) => {
                    pat_chars.next();
                    text_chars.next();
                }
                (Some(&p_char), Some(&t_char)) if p_char == t_char => {
                    pat_chars.next();
                    text_chars.next();
                }
                (Some(_), Some(_)) => return false, // Mismatch
                (None, None) => return true,        // Both pattern and text exhausted
                (None, Some(_)) => return false,    // Pattern exhausted, text remains
                (Some(_), None) => {                // Text exhausted, pattern remains
                    // If remaining pattern is just '*', it's a match
                    if pat_chars.clone().all(|c| c == '*') {
                        return true;
                    }
                    return false;
                }
            }
        }
    }
}

fn load_gitignore(dir_containing_gitignore: &Path, config: &Config) -> GitIgnore {
    let gitignore_path = dir_containing_gitignore.join(".gitignore");
    if !gitignore_path.exists() {
        return GitIgnore::empty();
    }

    let mut patterns = Vec::new();
    match fs::read_to_string(&gitignore_path) {
        Ok(contents) => {
            for line in contents.lines() {
                let line_trimmed = line.trim();
                if line_trimmed.is_empty() || line_trimmed.starts_with('#') {
                    continue;
                }
                patterns.push(GitIgnorePattern::new(line_trimmed.to_string()));
            }
        }
        Err(e) => {
            // Always print a warning if .gitignore can't be read
            eprintln!(
                "Warning: Could not read .gitignore file at '{}': {}",
                gitignore_path.display(),
                e
            );
            if config.verbose { // Provide more context if verbose
                eprintln!("Ignoring rules from this file might not be applied.");
            }
        }
    }
    GitIgnore { patterns }
}


fn process_file(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    if filename.is_empty() {
        return Err("Empty filename provided to process_file".into());
    }
    let contents = fs::read_to_string(filename)?;
    let language = get_language_from_extension(filename);
    let formatted = format!(
        "# {}\n```{}\n{}\n```",
        filename,
        language,
        contents.trim_end()
    );
    Ok(formatted)
}

fn get_language_from_extension(filename: &str) -> &str {
    let path = Path::new(filename);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust", Some("py") => "python", Some("js") => "javascript", Some("ts") => "typescript",
        Some("jsx") => "jsx", Some("tsx") => "tsx", Some("java") => "java", Some("c") => "c", Some("h") => "c",
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("hh") => "cpp", Some("cs") => "csharp",
        Some("go") => "go", Some("rb") => "ruby", Some("php") => "php", Some("swift") => "swift",
        Some("kt") | Some("kts") => "kotlin", Some("r") => "r", Some("m") => "matlab", Some("mm") => "objectivec",
        Some("sql") => "sql", Some("sh") | Some("bash") | Some("zsh") => "bash", Some("yaml") | Some("yml") => "yaml",
        Some("json") => "json", Some("xml") => "xml", Some("html") | Some("htm") => "html", Some("css") => "css",
        Some("scss") | Some("sass") => "scss", Some("less") => "less", Some("md") | Some("markdown") => "markdown",
        Some("tex") => "latex", Some("vim") | Some("vimrc") => "vim", Some("lua") => "lua", Some("dart") => "dart",
        Some("scala") => "scala", Some("jl") => "julia", Some("hs") => "haskell",
        Some("clj") | Some("cljs") | Some("cljc") | Some("edn") => "clojure", Some("ex") | Some("exs") => "elixir",
        Some("erl") | Some("hrl") => "erlang", Some("ml") | Some("mli") => "ocaml",
        Some("fs") | Some("fsi") | Some("fsx") | Some("fsscript") => "fsharp", Some("pl") | Some("pm") => "perl",
        Some("ps1") | Some("psm1") | Some("psd1") => "powershell", Some("toml") => "toml", Some("ini") => "ini",
        Some("cfg") => "ini", Some("conf") => "ini", Some("dockerfile") | Some("Dockerfile") => "dockerfile",
        Some("makefile") | Some("Makefile") | Some("mk") | Some("mak") => "makefile", Some("gradle") => "groovy",
        Some("tf") | Some("tfvars") => "terraform", Some("hcl") => "hcl", Some("http") => "http",
        Some("gd") => "gdscript", _ => "",
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "macos") {
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() && stdin.flush().is_ok() && child.wait()?.success() {
                    return Ok(());
                }
            }
        }
    } else if cfg!(target_os = "windows") {
        if let Ok(mut child) = Command::new("clip").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() && stdin.flush().is_ok() && child.wait()?.success() {
                    return Ok(());
                }
            }
        }
    } else { // Assume X11/Wayland environments for Linux/BSD etc.
        if let Ok(mut child) = Command::new("xclip").arg("-selection").arg("clipboard").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() && stdin.flush().is_ok() && child.wait()?.success() {
                    return Ok(());
                }
            }
        } else if let Ok(mut child) = Command::new("xsel").arg("--clipboard").arg("--input").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() && stdin.flush().is_ok() && child.wait()?.success() {
                    return Ok(());
                }
            }
        } else if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() {
                if stdin.write_all(text.as_bytes()).is_ok() && stdin.flush().is_ok() && child.wait()?.success() {
                    return Ok(());
                }
            }
        }
    }
    Err("No clipboard tool (pbcopy, clip, xclip, xsel, wl-copy) found or it failed.".into())
}
