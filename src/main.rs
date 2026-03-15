mod diff;
mod html;

use std::fs;
use std::path::PathBuf;
use std::process;

use clap::Parser;

use diff::{DiffOptions, OutputFormat, diff_texts, exit_code};
use html::{extract_text, looks_like_html};

/// hdiff — diff HTML files (or plain text) by their plaintext content.
///
/// When a file is detected as HTML its visible text is extracted before
/// comparison.  Plain-text files are compared as-is, so you can mix HTML
/// and plain-text operands freely.
///
/// Exit status: 0 = identical, 1 = different, 2 = error.
#[derive(Parser)]
#[command(name = "hdiff", version, about, long_about = None)]
struct Cli {
    /// First file (HTML or plain text)
    file_a: PathBuf,

    /// Second file (HTML or plain text)
    file_b: PathBuf,

    /// Number of context lines to show around each change
    #[arg(short = 'U', long = "unified", default_value_t = 3, value_name = "N")]
    context: usize,

    /// Disable ANSI colour output
    #[arg(long = "no-color", alias = "no-colour")]
    no_color: bool,

    /// Treat FILE_A as HTML regardless of its content
    #[arg(long = "html-a")]
    force_html_a: bool,

    /// Treat FILE_B as HTML regardless of its content
    #[arg(long = "html-b")]
    force_html_b: bool,

    /// Treat both files as plain text (skip HTML detection)
    #[arg(long = "text", short = 'a')]
    force_text: bool,

    /// Print only whether the files differ (no diff output)
    #[arg(short = 'q', long = "brief")]
    brief: bool,
}

fn read_as_text(path: &PathBuf, force_html: bool, force_text: bool) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {}", path.display(), e))?;

    let is_html = if force_text {
        false
    } else if force_html {
        true
    } else {
        looks_like_html(&raw)
    };

    if is_html {
        Ok(extract_text(&raw))
    } else {
        Ok(raw)
    }
}

fn label(path: &PathBuf) -> String {
    path.display().to_string()
}

fn main() {
    let cli = Cli::parse();

    let text_a = match read_as_text(&cli.file_a, cli.force_html_a, cli.force_text) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("hdiff: {e}");
            process::exit(2);
        }
    };

    let text_b = match read_as_text(&cli.file_b, cli.force_html_b, cli.force_text) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("hdiff: {e}");
            process::exit(2);
        }
    };

    let opts = DiffOptions {
        format: OutputFormat::Unified,
        context_lines: cli.context,
        color: !cli.no_color,
        label_a: label(&cli.file_a),
        label_b: label(&cli.file_b),
    };

    let result = diff_texts(&text_a, &text_b, &opts);

    if cli.brief {
        if result.is_some() {
            println!(
                "Files {} and {} differ",
                cli.file_a.display(),
                cli.file_b.display()
            );
        }
    } else if let Some(ref diff) = result {
        print!("{diff}");
    }

    process::exit(exit_code(&result));
}
