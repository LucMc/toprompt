use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    // Get command line arguments (skip the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    
    if args.is_empty() {
        eprintln!("Usage: {} <file1|dir1> [file2|dir2] ...", env::args().next().unwrap());
        std::process::exit(1);
    }
    
    let mut formatted_content = String::new();
    let mut successful_files = 0;
    let mut file_index = 0;
    
    // Process each argument (can be file or directory)
    for arg in args.iter() {
        match process_path(arg, &mut formatted_content, &mut file_index, &mut successful_files) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("Error processing '{}': {}", arg, e);
            }
        }
    }
    
    if successful_files == 0 {
        eprintln!("No files were successfully processed.");
        std::process::exit(1);
    }
    
    // Copy to clipboard
    match copy_to_clipboard(&formatted_content) {
        Ok(_) => {
            println!("\nSuccessfully copied {} file(s) to clipboard!", successful_files);
            println!("\n--- Clipboard Contents ---\n");
            println!("{}", formatted_content);
        }
        Err(e) => {
            eprintln!("Failed to copy to clipboard: {}", e);
            println!("\n--- Output (not copied) ---\n");
            println!("{}", formatted_content);
        }
    }
}

fn process_path(
    path: &str,
    formatted_content: &mut String,
    file_index: &mut usize,
    successful_files: &mut usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = Path::new(path);
    
    if path.is_file() {
        // Process single file
        match process_file(path.to_str().unwrap()) {
            Ok(content) => {
                if *file_index > 0 {
                    formatted_content.push_str("\n\n");
                }
                formatted_content.push_str(&content);
                *successful_files += 1;
                *file_index += 1;
            }
            Err(e) => return Err(e),
        }
    } else if path.is_dir() {
        // Process directory recursively
        process_directory(path, formatted_content, file_index, successful_files)?;
    } else {
        return Err(format!("'{}' is neither a file nor a directory", path.display()).into());
    }
    
    Ok(())
}

fn process_directory(
    dir: &Path,
    formatted_content: &mut String,
    file_index: &mut usize,
    successful_files: &mut usize,
) -> Result<(), Box<dyn std::error::Error>> {
    // Count items in directory
    let entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .collect();
    
    let item_count = entries.len();
    
    // Check if confirmation needed
    if item_count > 10 {
        println!("\nWarning: Directory '{}' contains {} items.", dir.display(), item_count);
        print!("Do you want to process all files in this directory? (y/n): ");
        io::stdout().flush()?;
        
        let mut response = String::new();
        io::stdin().read_line(&mut response)?;
        
        if !response.trim().to_lowercase().starts_with('y') {
            println!("Skipping directory '{}'", dir.display());
            return Ok(());
        }
    }
    
    // Process all entries
    for entry in entries {
        let path = entry.path();
        
        if path.is_file() {
            // Process file
            match process_file(path.to_str().unwrap()) {
                Ok(content) => {
                    if *file_index > 0 {
                        formatted_content.push_str("\n\n");
                    }
                    formatted_content.push_str(&content);
                    *successful_files += 1;
                    *file_index += 1;
                }
                Err(e) => {
                    eprintln!("Error processing '{}': {}", path.display(), e);
                }
            }
        } else if path.is_dir() {
            // Recursively process subdirectory
            process_directory(&path, formatted_content, file_index, successful_files)?;
        }
    }
    
    Ok(())
}

fn process_file(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Read file contents
    let contents = fs::read_to_string(filename)?;
    
    // Get the language hint from file extension
    let language = get_language_from_extension(filename);
    
    // Format with markdown
    let formatted = format!(
        "# {}\n```{}\n{}\n```",
        filename,
        language,
        contents.trim_end() // Remove trailing whitespace
    );
    
    Ok(formatted)
}

fn get_language_from_extension(filename: &str) -> &str {
    let path = Path::new(filename);
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => "rust",
        Some("py") => "python",
        Some("js") => "javascript",
        Some("ts") => "typescript",
        Some("jsx") => "jsx",
        Some("tsx") => "tsx",
        Some("java") => "java",
        Some("c") => "c",
        Some("cpp") | Some("cc") | Some("cxx") => "cpp",
        Some("cs") => "csharp",
        Some("go") => "go",
        Some("rb") => "ruby",
        Some("php") => "php",
        Some("swift") => "swift",
        Some("kt") => "kotlin",
        Some("r") => "r",
        Some("m") => "matlab",
        Some("sql") => "sql",
        Some("sh") | Some("bash") => "bash",
        Some("yaml") | Some("yml") => "yaml",
        Some("json") => "json",
        Some("xml") => "xml",
        Some("html") | Some("htm") => "html",
        Some("css") => "css",
        Some("scss") => "scss",
        Some("md") => "markdown",
        Some("tex") => "latex",
        Some("vim") => "vim",
        Some("lua") => "lua",
        Some("dart") => "dart",
        Some("scala") => "scala",
        Some("jl") => "julia",
        Some("hs") => "haskell",
        Some("clj") => "clojure",
        Some("ex") | Some("exs") => "elixir",
        Some("erl") => "erlang",
        Some("ml") => "ocaml",
        Some("fs") | Some("fsx") => "fsharp",
        Some("pl") => "perl",
        Some("ps1") => "powershell",
        Some("toml") => "toml",
        Some("ini") => "ini",
        Some("cfg") => "cfg",
        Some("dockerfile") | Some("Dockerfile") => "dockerfile",
        Some("makefile") | Some("Makefile") => "makefile",
        _ => "", // No language hint for unknown extensions
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Try xclip first (most common)
    if let Ok(mut child) = Command::new("xclip")
        .arg("-selection")
        .arg("clipboard")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.flush()?;
        }
        child.wait()?;
        return Ok(());
    }
    
    // Try xsel as fallback
    if let Ok(mut child) = Command::new("xsel")
        .arg("--clipboard")
        .arg("--input")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.flush()?;
        }
        child.wait()?;
        return Ok(());
    }
    
    // Try wl-copy for Wayland
    if let Ok(mut child) = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(text.as_bytes())?;
            stdin.flush()?;
        }
        child.wait()?;
        return Ok(());
    }
    
    Err("No clipboard tool found. Please install xclip, xsel, or wl-clipboard".into())
}
