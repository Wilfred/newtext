use clap::Parser;
use ignore::WalkBuilder;
use regex::Regex;
use std::env;
use std::fs;
use std::io;
use std::path::Path;

/// A simple find and replace tool that processes all text files in the current directory
#[derive(Parser)]
#[command(name = "newtext")]
#[command(version)]
#[command(about = "Find and replace text in all files in the current directory", long_about = None)]
struct Cli {
    /// The text to search for
    #[arg(value_name = "FIND")]
    find: String,

    /// The text to replace with
    #[arg(value_name = "REPLACE")]
    replace: String,

    /// Treat the find string as a regular expression pattern
    #[arg(short = 'p', long = "pattern")]
    pattern: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.find.is_empty() {
        eprintln!("Error: find string cannot be empty");
        std::process::exit(1);
    }

    // If using regex mode, compile the regex pattern
    let regex = if cli.pattern {
        match Regex::new(&cli.find) {
            Ok(re) => Some(re),
            Err(e) => {
                eprintln!("Error: Invalid regex pattern: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            eprintln!("Error getting current directory: {}", e);
            std::process::exit(1);
        }
    };

    let mut files_processed = 0;
    let mut files_modified = 0;

    for result in WalkBuilder::new(&current_dir)
        .hidden(false) // Don't automatically skip hidden files/dirs
        .standard_filters(true) // Use standard VCS filters (ignores .git, etc)
        .build()
    {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        // Only process files
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        let path = entry.path();

        match process_file(path, &cli.find, &cli.replace, regex.as_ref()) {
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

    println!(
        "\nProcessed {} files, modified {} files",
        files_processed, files_modified
    );
}

fn process_file(path: &Path, find: &str, replace: &str, regex: Option<&Regex>) -> io::Result<bool> {
    // Try to read the file as text
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => {
            // If we can't read it as text, it's probably binary, skip it
            return Ok(false);
        }
    };

    // Perform replacement based on mode
    let new_content = if let Some(re) = regex {
        // Regex mode
        if !re.is_match(&content) {
            return Ok(false);
        }
        re.replace_all(&content, replace).to_string()
    } else {
        // Literal mode
        if !content.contains(find) {
            return Ok(false);
        }
        content.replace(find, replace)
    };

    // Only write if content actually changed
    if new_content == content {
        return Ok(false);
    }

    // Write back to the file
    fs::write(path, new_content)?;

    Ok(true)
}
