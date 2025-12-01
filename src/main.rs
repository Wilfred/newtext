use std::env;
use std::fs;
use std::io;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        eprintln!("Usage: newtext <find> <replace>");
        eprintln!("Replaces all occurrences of <find> with <replace> in all files in the current directory");
        std::process::exit(1);
    }

    let find = &args[1];
    let replace = &args[2];

    if find.is_empty() {
        eprintln!("Error: find string cannot be empty");
        std::process::exit(1);
    }

    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {}", e);
            std::process::exit(1);
        }
    };

    let mut files_processed = 0;
    let mut files_modified = 0;

    for entry in WalkDir::new(&current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();

        // Skip .git directory and other hidden directories
        if path.components().any(|c| {
            c.as_os_str().to_string_lossy().starts_with('.')
        }) {
            continue;
        }

        match process_file(path, find, replace) {
            Ok(true) => {
                files_modified += 1;
                files_processed += 1;
                println!("Modified: {}", path.display());
            }
            Ok(false) => {
                files_processed += 1;
            }
            Err(e) => {
                eprintln!("Warning: Could not process {}: {}", path.display(), e);
            }
        }
    }

    println!("\nProcessed {} files, modified {} files", files_processed, files_modified);
}

fn process_file(path: &Path, find: &str, replace: &str) -> io::Result<bool> {
    // Try to read the file as text
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            // If we can't read it as text, it's probably binary, skip it
            return Ok(false);
        }
    };

    // Check if the file contains the search string
    if !content.contains(find) {
        return Ok(false);
    }

    // Replace all occurrences
    let new_content = content.replace(find, replace);

    // Write back to the file
    fs::write(path, new_content)?;

    Ok(true)
}
