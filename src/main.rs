use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use regex::Regex;

struct Config {
    use_gitignore: bool,
    verbose: bool,
    recursive: bool,
    regex_pattern: Option<String>,
    paths: Vec<String>,
}

fn print_usage() {
    eprintln!(
        "Usage: {} [-i] [-v] [-r] [-R <pattern>] <file1|dir1> [file2|dir2] ...",
        env::args().next().unwrap_or_else(|| "toprompt".to_string())
    );
    eprintln!("  -i             Use .gitignore files to exclude files/directories");
    eprintln!("  -v             Verbose output (show ignored files, detailed success messages, and preview)");
    eprintln!("  -r             Recursively process subdirectories");
    eprintln!("  -R <pattern>   Recursively process subdirectories, matching files against regex pattern (applied to relative paths)");
    eprintln!("\nExample combined flags: -ri, -rv, -iv, -riv (and permutations)");
    eprintln!("\nExamples:");
    eprintln!("  toprompt file.txt             # Copy specific file (prints 'file.txt')");
    eprintln!("  toprompt -v file.txt          # Verbose copy of file.txt");
    eprintln!("  toprompt .                    # Copy all files in current folder (prints filenames)");
    eprintln!("  toprompt -R \"^src/.*\\.rs$\" . # Copy all .rs files in src/ and its subdirs (prints matching filenames)");
}

fn main() {
    let config = parse_args();

    if config.paths.is_empty() {
        print_usage();
        std::process::exit(1);
    }

    let compiled_regex = match &config.regex_pattern {
        Some(pattern_str) => match Regex::new(pattern_str) {
            Ok(re) => Some(re),
            Err(e) => {
                eprintln!("Error: Invalid regex pattern '{}': {}", pattern_str, e);
                print_usage();
                std::process::exit(1);
            }
        },
        None => None,
    };

    let mut formatted_content = String::new();
    let mut successful_files = 0;
    let mut file_index = 0;
    let mut copied_file_names: Vec<String> = Vec::new(); // To store names of copied files

    for path_str in config.paths.iter() {
        match process_path(
            path_str,
            &mut formatted_content,
            &mut file_index,
            &mut successful_files,
            &config,
            &compiled_regex,
            &mut copied_file_names, // Pass the a mutable reference
        ) {
            Ok(_) => {}
            Err(e) => {
                if config.verbose { // Only print processing errors if verbose, or they are critical like path not found.
                    eprintln!("Error processing '{}': {}", path_str, e);
                }
            }
        }
    }

    if successful_files == 0 {
        eprintln!("No files were successfully processed.");
        if config.regex_pattern.is_some() && !config.paths.is_empty() {
            eprintln!("Check your regex pattern and paths. Regex is applied to paths relative to the input directory arguments.");
        }
        std::process::exit(1);
    }

    match copy_to_clipboard(&formatted_content) {
        Ok(_) => { // Successfully copied to clipboard
            if config.verbose {
                println!(
                    "\nSuccessfully copied {} file(s) to clipboard!",
                    successful_files
                );
                if config.use_gitignore { println!("(.gitignore rules were applied)"); }
                if config.recursive { println!("(Recursive mode was active)"); }
                if config.regex_pattern.is_some() {
                    println!("(Regex filter '{}' was applied)", config.regex_pattern.as_ref().unwrap());
                }
                println!("\nCopied files:");
                for name in &copied_file_names {
                    println!("{}", name);
                }
                println!(
                    "\n--- Clipboard Contents Preview (first 500 chars) ---\n"
                );
                let preview = if formatted_content.len() > 500 {
                    &formatted_content[..500]
                } else {
                    &formatted_content
                };
                println!("{}...", preview);
            } else { // Not verbose, successfully copied
                println!(":: Copied {} files ::", successful_files);
                // Iterate over the first 10 names, or fewer if the list is shorter.
                for name in copied_file_names.iter().take(10) {
                    println!("{}", name);
                }

                // If there were more than 10 files in total, print "..."
                if copied_file_names.len() > 10 {
                    println!("...");
                }
            }
        }
        Err(e) => { // Failed to copy to clipboard
            eprintln!("Failed to copy to clipboard: {}", e);
            // Always inform about processed files, then show content for manual copy
            println!("\nFiles processed (but not copied to clipboard):");
            for name in &copied_file_names {
                println!("{}", name);
            }
            println!("\n--- Output (not copied to clipboard) ---\n");
            println!("{}", formatted_content);
        }
    }
}

fn parse_args() -> Config {
    let mut config = Config {
        use_gitignore: false,
        verbose: false,
        recursive: false,
        regex_pattern: None,
        paths: Vec::new(),
    };

    let mut iter = env::args().skip(1).peekable();
    while let Some(arg) = iter.next() {
        if arg == "-R" {
            if let Some(pattern) = iter.next() {
                if pattern.starts_with('-') && pattern.len() > 1 && pattern.chars().nth(1).map_or(false, |c| c.is_alphabetic() && c != 'R') {
                    eprintln!("Error: -R flag requires a regex pattern, but got '{}'. Did you forget to provide a pattern or quote it?", pattern);
                    print_usage();
                    std::process::exit(1);
                }
                config.regex_pattern = Some(pattern);
                config.recursive = true;
            } else {
                eprintln!("Error: -R flag requires a regex pattern.");
                print_usage();
                std::process::exit(1);
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            for char_code in arg.chars().skip(1) {
                match char_code {
                    'r' => config.recursive = true,
                    'i' => config.use_gitignore = true,
                    'v' => config.verbose = true,
                    _ => {
                        eprintln!("Unknown flag component in '{}': -{}", arg, char_code);
                        print_usage();
                        std::process::exit(1);
                    }
                }
            }
        } else if !arg.starts_with('-') {
            config.paths.push(arg);
        } else {
            eprintln!("Unknown or malformed argument: {}", arg);
            print_usage();
            std::process::exit(1);
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
    compiled_regex: &Option<Regex>,
    copied_file_names: &mut Vec<String>, // Added parameter
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path_str);
    let absolute_path = fs::canonicalize(path).map_err(|e| format!("Path error for '{}': {}. Ensure it exists and is accessible.", path_str, e))?;


    if absolute_path.is_file() {
        if let Some(rgx) = compiled_regex {
            let normalized_path_str_to_match = path_str.replace('\\', "/");
            if !rgx.is_match(&normalized_path_str_to_match) {
                if config.verbose {
                    println!(
                        "Skipping file (regex -R did not match path '{}'): {}",
                        normalized_path_str_to_match, path_str
                    );
                }
                return Ok(());
            }
        }

        match process_file(absolute_path.to_str().unwrap()) {
            Ok((file_content_segment, display_name_str)) => { // Expect tuple
                if *file_index > 0 {
                    formatted_content.push_str("\n\n");
                }
                formatted_content.push_str(&file_content_segment);
                *successful_files += 1;
                *file_index += 1;
                copied_file_names.push(display_name_str); // Collect display name
            }
            Err(e) => return Err(e),
        }
    } else if absolute_path.is_dir() {
        let gitignore = if config.use_gitignore {
            let mut gitignore = GitIgnore::with_defaults(&absolute_path);
            let loaded = load_gitignore(&absolute_path);
            gitignore.merge(loaded);
            gitignore
        } else {
            GitIgnore::empty()
        };
        process_directory(
            &absolute_path,
            &absolute_path,
            formatted_content,
            file_index,
            successful_files,
            config,
            &gitignore,
            compiled_regex,
            copied_file_names, // Pass it down
        )?;
    } else {
        return Err(format!(
            "'{}' (resolved to '{}') is neither a file nor a directory that can be processed",
            path_str, absolute_path.display()
        )
        .into());
    }

    Ok(())
}

fn process_directory(
    dir_to_process: &Path,
    cmd_arg_base_dir: &Path,
    formatted_content: &mut String,
    file_index: &mut usize,
    successful_files: &mut usize,
    config: &Config,
    parent_gitignore: &GitIgnore,
    compiled_regex: &Option<Regex>,
    copied_file_names: &mut Vec<String>, // Added parameter
) -> Result<(), Box<dyn std::error::Error>> {
    if config.use_gitignore {
        let dir_relative_to_cmd_arg_base = dir_to_process.strip_prefix(cmd_arg_base_dir).unwrap_or(dir_to_process);
        if parent_gitignore.should_ignore(&dir_relative_to_cmd_arg_base, true, cmd_arg_base_dir) {
            if config.verbose {
                println!("Ignoring directory (via .gitignore): {}", dir_to_process.display());
            }
            return Ok(());
        }
    }

    let mut current_gitignore = parent_gitignore.clone();
    if config.use_gitignore && dir_to_process.join(".gitignore").exists() {
        let new_gitignore = load_gitignore(dir_to_process);
        current_gitignore.merge(new_gitignore);
        if config.verbose {
            println!("Loaded .gitignore from: {}", dir_to_process.join(".gitignore").display());
        }
    }

    let mut entries: Vec<_> = fs::read_dir(dir_to_process)?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.path());

    let filtered_entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            if !config.use_gitignore {
                return true;
            }
            let entry_abs_path = entry.path();
            let path_relative_to_cmd_arg_base = entry_abs_path.strip_prefix(cmd_arg_base_dir).unwrap_or(&entry_abs_path);
            let should_ignore = current_gitignore.should_ignore(&path_relative_to_cmd_arg_base, entry_abs_path.is_dir(), cmd_arg_base_dir);
            if config.verbose && should_ignore {
                println!("Ignoring (via .gitignore): {}", path_relative_to_cmd_arg_base.display());
            }
            !should_ignore
        })
        .collect();

    if filtered_entries.len() > 10 && dir_to_process == cmd_arg_base_dir {
        if config.verbose { // Only show confirmation prompt if verbose
            println!(
                "\nWarning: Directory '{}' contains {} items (after .gitignore if used).",
                dir_to_process.display(),
                filtered_entries.len()
            );
            print!("Do you want to process all files in this directory level{}? (y/n): ",
                if config.recursive {" and its subdirectories (if applicable)"} else {""}
            );
            io::stdout().flush()?;
            let mut response = String::new();
            io::stdin().read_line(&mut response)?;
            if !response.trim().to_lowercase().starts_with('y') {
                println!("Skipping directory '{}'", dir_to_process.display());
                return Ok(());
            }
        }
    }

    for entry in filtered_entries {
        let entry_abs_path = entry.path();
        if entry_abs_path.is_file() {
            let mut process_this_file = true;
            if let Some(rgx) = compiled_regex {
                let path_relative_to_cmd_arg = entry_abs_path.strip_prefix(cmd_arg_base_dir).unwrap_or(&entry_abs_path);
                let path_to_match_str = path_relative_to_cmd_arg.to_string_lossy();
                let normalized_path_to_match = path_to_match_str.replace('\\', "/");

                if !rgx.is_match(&normalized_path_to_match) {
                    if config.verbose {
                        println!(
                            "Skipping file (regex -R did not match relative path '{}'): {}",
                            normalized_path_to_match, entry_abs_path.display()
                        );
                    }
                    process_this_file = false;
                }
            }

            if process_this_file {
                match process_file(entry_abs_path.to_str().unwrap()) {
                    Ok((file_content_segment, display_name_str)) => { // Expect tuple
                        if *file_index > 0 {
                            formatted_content.push_str("\n\n");
                        }
                        formatted_content.push_str(&file_content_segment);
                        *successful_files += 1;
                        *file_index += 1;
                        copied_file_names.push(display_name_str); // Collect display name
                    }
                    Err(e) => {
                        if config.verbose {
                           eprintln!("Error processing file '{}': {}", entry_abs_path.display(), e);
                        }
                    }
                }
            }
        } else if entry_abs_path.is_dir() {
            if config.recursive {
                process_directory(
                    &entry_abs_path,
                    cmd_arg_base_dir,
                    formatted_content,
                    file_index,
                    successful_files,
                    config,
                    &current_gitignore,
                    compiled_regex,
                    copied_file_names, // Pass it down
                )?;
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
struct GitIgnore {
    patterns: Vec<GitIgnorePattern>,
    effective_base_dir: PathBuf,
}

#[derive(Clone)]
struct GitIgnorePattern {
    pattern: String,
    raw_pattern: String,
    is_negation: bool,
    is_directory: bool,
    is_absolute: bool,
    contains_slash: bool,
    defined_in_dir: PathBuf,
}

impl GitIgnore {
    fn empty() -> Self {
        GitIgnore {
            patterns: Vec::new(),
            effective_base_dir: PathBuf::new(),
        }
    }

    fn with_defaults(operation_base_dir: &Path) -> Self {
        let mut patterns = Vec::new();
        patterns.push(GitIgnorePattern::new(".git/".to_string(), operation_base_dir));
        patterns.push(GitIgnorePattern::new(".gitignore".to_string(), operation_base_dir));
        GitIgnore {
            patterns,
            effective_base_dir: operation_base_dir.to_path_buf(),
        }
    }

    fn merge(&mut self, other: GitIgnore) {
        self.patterns.extend(other.patterns);
    }

    fn should_ignore(&self, path_to_check_relative_to_cmd_base: &Path, is_item_dir: bool, overall_cmd_arg_base_dir: &Path) -> bool {
        let mut ignored = false;
        for pattern_rule in &self.patterns {
            let abs_path_to_check = overall_cmd_arg_base_dir.join(path_to_check_relative_to_cmd_base);
            if let Ok(path_relative_to_pattern_def_dir) = abs_path_to_check.strip_prefix(&pattern_rule.defined_in_dir) {
                let path_str_to_match = path_relative_to_pattern_def_dir.to_string_lossy().replace('\\', "/");
                if pattern_rule.matches(&path_str_to_match, is_item_dir) {
                    ignored = !pattern_rule.is_negation;
                }
            } else if !pattern_rule.is_absolute && !pattern_rule.contains_slash {
                let path_str_to_match = path_to_check_relative_to_cmd_base.to_string_lossy().replace('\\', "/");
                if pattern_rule.matches_against_any_component(&path_str_to_match, is_item_dir) {
                     ignored = !pattern_rule.is_negation;
                }
            }
        }
        ignored
    }
}

impl GitIgnorePattern {
    fn new(raw_pattern_str: String, pattern_defined_in_dir_param: &Path) -> Self {
        let mut pattern = raw_pattern_str.trim().to_string();
        if pattern.is_empty() || pattern.starts_with('#') {
            return GitIgnorePattern {
                pattern: String::new(),
                raw_pattern: String::new(),
                is_negation: false,
                is_directory: false,
                is_absolute: false,
                contains_slash: false,
                defined_in_dir: pattern_defined_in_dir_param.to_path_buf(),
            };
        }
        let is_negation = pattern.starts_with('!');
        if is_negation { pattern = pattern[1..].to_string(); }
        let is_absolute = pattern.starts_with('/');
        if is_absolute { pattern = pattern[1..].to_string(); }
        let is_directory = pattern.ends_with('/');
        if is_directory { pattern = pattern[..pattern.len() - 1].to_string(); }
        let contains_slash = !is_absolute && pattern.contains('/');
        GitIgnorePattern {
            pattern, raw_pattern: raw_pattern_str, is_negation, is_directory, is_absolute, contains_slash,
            defined_in_dir: pattern_defined_in_dir_param.to_path_buf(),
        }
    }

    fn matches(&self, path_str_relative_to_def_dir: &str, is_item_dir: bool) -> bool {
        if self.pattern.is_empty() { return false; }
        if self.is_directory && !is_item_dir { return false; }
        if self.is_absolute || self.contains_slash {
            self.simple_glob_match(&self.pattern, path_str_relative_to_def_dir)
        } else {
            Path::new(path_str_relative_to_def_dir).file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |filename_str| self.simple_glob_match(&self.pattern, filename_str)) ||
            self.simple_glob_match(&self.pattern, path_str_relative_to_def_dir)
        }
    }

    fn matches_against_any_component(&self, path_str: &str, is_item_dir: bool) -> bool {
        if self.pattern.is_empty() { return false; }
        if self.is_directory && !is_item_dir { return false; }
        if Path::new(path_str).file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |name_part| self.simple_glob_match(&self.pattern, name_part)) {
            return true;
        }
        if !path_str.contains('/') && self.simple_glob_match(&self.pattern, path_str) {
            return true;
        }
        false
    }

    fn simple_glob_match(&self, pattern: &str, text: &str) -> bool {
        if pattern == "*" { return !text.contains('/'); }
        if pattern.is_empty() { return text.is_empty(); }
        if text.is_empty() { return pattern == "*" || pattern.is_empty(); }
        if !pattern.contains('*') && !pattern.contains('?') {
            return pattern == text;
        }
        let pattern_parts: Vec<&str> = pattern.split('*').collect();
        if pattern_parts.is_empty() { return true; }
        let mut text_idx = 0;
        for (i, part) in pattern_parts.iter().enumerate() {
            if part.is_empty() {
                if i == 0 && pattern_parts.len() == 1 { return !text.contains('/'); }
                continue;
            }
            if i == 0 && !pattern.starts_with('*') {
                if !text.starts_with(part) { return false; }
                text_idx = part.len();
            } else {
                if let Some(found_pos) = text[text_idx..].find(part) {
                    text_idx += found_pos + part.len();
                } else { return false; }
            }
        }
        if !pattern.ends_with('*') && text_idx != text.len() { return false; }
        true
    }
}

fn load_gitignore(dir_containing_gitignore: &Path) -> GitIgnore {
    let gitignore_path = dir_containing_gitignore.join(".gitignore");
    let mut patterns = Vec::new();
    if let Ok(contents) = fs::read_to_string(&gitignore_path) {
        for line in contents.lines() {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() || line_trimmed.starts_with('#') { continue; }
            patterns.push(GitIgnorePattern::new(line_trimmed.to_string(), dir_containing_gitignore));
        }
    }
    GitIgnore { patterns, effective_base_dir: dir_containing_gitignore.to_path_buf() }
}

// Returns (formatted_content_for_this_file, display_name_string)
fn process_file(filepath_str: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let contents = fs::read_to_string(filepath_str)?;
    let language = get_language_from_extension(filepath_str);
    let path_obj = Path::new(filepath_str);
    let display_name = env::current_dir()
        .ok()
        .and_then(|cwd| path_obj.strip_prefix(&cwd).ok())
        .unwrap_or(path_obj);

    let formatted_segment = format!(
        "# {}\n```{}\n{}\n```",
        display_name.display(),
        language,
        contents.trim_end()
    );
    Ok((formatted_segment, display_name.display().to_string()))
}

fn get_language_from_extension(filename: &str) -> &str {
    let path = Path::new(filename);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust", Some("py") => "python", Some("js") => "javascript", Some("ts") => "typescript",
        Some("jsx") => "jsx", Some("tsx") => "tsx", Some("java") => "java", Some("c") => "c",
        Some("cpp") | Some("cc") | Some("cxx") | Some("h") | Some("hpp") => "cpp",
        Some("cs") => "csharp", Some("go") => "go", Some("rb") => "ruby", Some("php") => "php",
        Some("swift") => "swift", Some("kt") => "kotlin", Some("r") => "r", Some("m") => "matlab",
        Some("mm") => "objective-c", Some("sql") => "sql", Some("sh") | Some("bash") | Some("zsh") => "bash",
        Some("yaml") | Some("yml") => "yaml", Some("json") => "json", Some("xml") => "xml",
        Some("html") | Some("htm") => "html", Some("css") => "css", Some("scss") | Some("sass") => "scss",
        Some("less") => "less", Some("md") | Some("markdown") => "markdown", Some("tex") => "latex",
        Some("vim") | Some("vimrc") => "vim", Some("lua") => "lua", Some("dart") => "dart",
        Some("scala") => "scala", Some("jl") => "julia", Some("hs") => "haskell",
        Some("clj") | Some("cljs") | Some("cljc") | Some("edn") => "clojure",
        Some("ex") | Some("exs") => "elixir", Some("erl") | Some("hrl") => "erlang",
        Some("ml") | Some("mli") => "ocaml", Some("fs") | Some("fsx") | Some("fsi") => "fsharp",
        Some("pl") | Some("pm") => "perl", Some("ps1") | Some("psm1") | Some("psd1") => "powershell",
        Some("toml") => "toml", Some("ini") => "ini", Some("cfg") => "cfg", Some("conf") => "plaintext",
        Some("log") => "log", Some("dockerfile") | Some("Dockerfile") => "dockerfile",
        Some("makefile") | Some("Makefile") | Some("mk") | Some("mak") => "makefile",
        Some("gd") => "gdscript", Some("gql") | Some("graphql") => "graphql",
        Some("hbs") | Some("handlebars") => "handlebars", Some("jinja") | Some("j2") => "jinja",
        Some("proto") => "protobuf", Some("sol") => "solidity", Some("tf") => "terraform",
        Some("v") => "vlang", Some("vue") => "vue", Some("svelte") => "svelte",
        _ => "",
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    if cfg!(target_os = "macos") {
        if let Ok(mut child) = Command::new("pbcopy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { stdin.write_all(text.as_bytes())?; stdin.flush()?; }
            if child.wait()?.success() { return Ok(()); }
        }
    } else if cfg!(target_os = "windows") {
        if let Ok(mut child) = Command::new("clip").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { stdin.write_all(text.as_bytes())?; stdin.flush()?; }
            if child.wait()?.success() { return Ok(()); }
        }
    } else {
        if let Ok(mut child) = Command::new("xclip").arg("-selection").arg("clipboard").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { stdin.write_all(text.as_bytes())?; stdin.flush()?; }
            if child.wait()?.success() { return Ok(()); }
        }
        if let Ok(mut child) = Command::new("xsel").arg("--clipboard").arg("--input").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { stdin.write_all(text.as_bytes())?; stdin.flush()?; }
            if child.wait()?.success() { return Ok(()); }
        }
        if let Ok(mut child) = Command::new("wl-copy").stdin(Stdio::piped()).spawn() {
            if let Some(mut stdin) = child.stdin.take() { stdin.write_all(text.as_bytes())?; stdin.flush()?; }
            if child.wait()?.success() { return Ok(()); }
        }
    }
    Err("No clipboard tool found or tool failed. Please install xclip/xsel (Linux X11), wl-clipboard (Wayland), pbcopy (macOS), or ensure clip.exe is in PATH (Windows).".into())
}
