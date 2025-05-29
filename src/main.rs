use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::io::Write;

fn main() {
    // Get command line arguments (skip the program name)
    let args: Vec<String> = env::args().skip(1).collect();
    
    if args.is_empty() {
        eprintln!("Usage: {} <file1> [file2] [file3] ...", env::args().next().unwrap());
        std::process::exit(1);
    }
    
    let mut formatted_content = String::new();
    let mut successful_files = 0;
    
    // Process each file
    for (index, filename) in args.iter().enumerate() {
        match process_file(filename) {
            Ok(content) => {
                // Add newline between files (but not before the first file)
                if index > 0 {
                    formatted_content.push_str("\n\n");
                }
                formatted_content.push_str(&content);
                successful_files += 1;
            }
            Err(e) => {
                eprintln!("Error processing '{}': {}", filename, e);
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
            println!("Successfully copied {} file(s) to clipboard!", successful_files);
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
