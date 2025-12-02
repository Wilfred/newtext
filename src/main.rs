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
    #[arg(value_name = "OLD")]
    old: String,

    /// The text to replace with
    #[arg(value_name = "NEW")]
    new: String,

    /// Treat the find string as a regular expression pattern
    #[arg(short = 'p', long = "pattern")]
    pattern: bool,

    /// Case-insensitive matching with case-preserving replacement
    #[arg(short = 'i', long = "ignore-case")]
    ignore_case: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.old.is_empty() {
        eprintln!("Error: old string cannot be empty");
        std::process::exit(1);
    }

    // If using regex mode, compile the regex pattern
    let regex = if cli.pattern {
        let pattern = if cli.ignore_case {
            format!("(?i){}", cli.old)
        } else {
            cli.old.clone()
        };
        match Regex::new(&pattern) {
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
    let mut directories_traversed = 0;

    for result in WalkBuilder::new(&current_dir)
        .hidden(false) // Don't automatically skip hidden files/dirs
        .standard_filters(true) // Use standard VCS filters (ignores .git, etc)
        .build()
    {
        let entry = match result {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        // Track directories
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            directories_traversed += 1;
            continue;
        }

        // Only process files
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }

        let path = entry.path();

        match process_file(path, &cli.old, &cli.new, regex.as_ref(), cli.ignore_case) {
            Ok(true) => {
                files_modified += 1;
                files_processed += 1;
            }
            Ok(false) => {
                files_processed += 1;
            }
            Err(e) => {
                eprintln!("Warning: Could not process {}: {}", path.display(), e);
            }
        }

        // Print progress update (clear line and overwrite)
        eprint!(
            "\x1b[2K\rFiles: {}, Dirs: {}, Modified: {}",
            files_processed, directories_traversed, files_modified
        );
    }

    // Print newline after progress updates
    eprintln!();
}

/// Apply the case pattern from the matched text to the replacement text
fn apply_case_pattern(matched: &str, replacement: &str) -> String {
    // If the matched text has no letters, just return the replacement as-is
    if !matched.chars().any(|c| c.is_alphabetic()) {
        return replacement.to_string();
    }

    let matched_chars: Vec<char> = matched.chars().collect();

    // Determine case pattern of matched text
    let alphabetic_chars: Vec<char> = matched_chars
        .iter()
        .filter(|c| c.is_alphabetic())
        .copied()
        .collect();

    if alphabetic_chars.is_empty() {
        return replacement.to_string();
    }

    let all_upper = alphabetic_chars.iter().all(|c| c.is_uppercase());
    let all_lower = alphabetic_chars.iter().all(|c| c.is_lowercase());
    let first_upper = alphabetic_chars[0].is_uppercase()
        && alphabetic_chars[1..].iter().all(|c| c.is_lowercase());

    if all_upper {
        // All uppercase: BAR -> BAR
        replacement.to_uppercase()
    } else if all_lower {
        // All lowercase: bar -> bar
        replacement.to_lowercase()
    } else if first_upper {
        // Title case: Bar -> Bar
        let mut result = String::new();
        let mut first_letter = true;
        for c in replacement.chars() {
            if c.is_alphabetic() {
                if first_letter {
                    result.push_str(&c.to_uppercase().to_string());
                    first_letter = false;
                } else {
                    result.push_str(&c.to_lowercase().to_string());
                }
            } else {
                result.push(c);
            }
        }
        result
    } else {
        // Mixed case (including camelCase): try to preserve pattern character by character
        let mut result = String::new();
        let mut matched_alpha_iter = alphabetic_chars.iter();

        for c in replacement.chars() {
            if c.is_alphabetic() {
                if let Some(&matched_c) = matched_alpha_iter.next() {
                    if matched_c.is_uppercase() {
                        result.push_str(&c.to_uppercase().to_string());
                    } else {
                        result.push_str(&c.to_lowercase().to_string());
                    }
                } else {
                    // If we run out of matched characters, keep the original case
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }
        result
    }
}

fn process_file(
    path: &Path,
    old: &str,
    new: &str,
    regex: Option<&Regex>,
    ignore_case: bool,
) -> io::Result<bool> {
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
        // Regex mode (ignore_case is already handled in regex compilation)
        if !re.is_match(&content) {
            return Ok(false);
        }
        re.replace_all(&content, new).to_string()
    } else if ignore_case {
        // Literal mode with case-insensitive matching and case-preserving replacement
        // Use regex for safe case-insensitive matching
        let pattern = format!("(?i){}", regex::escape(old));
        let re = match Regex::new(&pattern) {
            Ok(r) => r,
            Err(_) => return Ok(false),
        };

        if !re.is_match(&content) {
            return Ok(false);
        }

        // Replace all matches with case-preserved versions
        let result = re.replace_all(&content, |caps: &regex::Captures| {
            let matched = caps.get(0).unwrap().as_str();
            apply_case_pattern(matched, new)
        });

        result.to_string()
    } else {
        // Literal mode (case-sensitive)
        if !content.contains(old) {
            return Ok(false);
        }
        content.replace(old, new)
    };

    // Only write if content actually changed
    if new_content == content {
        return Ok(false);
    }

    // Write back to the file
    fs::write(path, new_content)?;

    Ok(true)
}
