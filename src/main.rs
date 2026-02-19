use anyhow::{Context, Result};
use clap::Parser;
use ignore::WalkBuilder;
use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tracing::{Level, info, warn};
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Merges code files while respecting .gitignore"
)]
struct Args {
    /// Directory to search
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Comma-separated extensions: py,rs,js
    #[arg(short, long, value_delimiter = ',')]
    exts: Vec<String>,

    /// Search subdirectories (default is false)
    #[arg(short, long)]
    recursive: bool,

    /// Custom directories to ignore (e.g., "node_modules,target")
    #[arg(short, long, value_delimiter = ',')]
    ignore_dirs: Vec<String>,

    /// Name of the resulting file
    #[arg(short, long, default_value = "merged.txt")]
    output: String,
}

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).context("Logging init failed")?;

    let args = Args::parse();

    if args.exts.is_empty() {
        warn!("Please provide extensions. Example: --exts py,rs");
        return Ok(());
    }

    let mut output_file =
        File::create(&args.output).with_context(|| format!("Failed to create {}", args.output))?;

    let mut builder = WalkBuilder::new(&args.path);
    builder.standard_filters(true).hidden(true);

    // If recursive is NOT requested, limit depth to 1 (current dir only)
    if !args.recursive {
        builder.max_depth(Some(1));
    }

    let mut total_files = 0;
    let mut total_lines = 0;

    info!("Walking directory: {:?}", args.path);

    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if should_process(path, &args) {
                    match merge_file(path, &mut output_file) {
                        Ok(lines) => {
                            info!("Merged: {:?}", path);
                            total_files += 1;
                            total_lines += lines;
                        }
                        Err(e) => warn!("Skipped {:?}: {}", path, e),
                    }
                }
            }
            Err(err) => warn!("Error walking path: {}", err),
        }
    }

    // Write a small summary at the end of the file
    writeln!(output_file, "\n# --- SUMMARY ---")?;
    writeln!(output_file, "# Total Files Merged: {}", total_files)?;
    writeln!(output_file, "# Total Lines Combined: {}", total_lines)?;

    info!("Success! {} files merged into {}", total_files, args.output);
    Ok(())
}

fn should_process(path: &Path, args: &Args) -> bool {
    if path.is_dir() {
        return false;
    }

    // Avoid merging the output file into itself
    if path.file_name().and_then(|n| n.to_str()) == Some(&args.output) {
        return false;
    }

    // Check custom ignore list against path components
    if !args.ignore_dirs.is_empty() {
        if path.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map_or(false, |s| args.ignore_dirs.contains(&s.to_string()))
        }) {
            return false;
        }
    }

    // Match extension
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| args.exts.iter().any(|wanted| wanted == ext))
        .unwrap_or(false)
}

fn merge_file(path: &Path, output: &mut impl Write) -> Result<usize> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut content = String::new();

    reader
        .read_to_string(&mut content)
        .context("Non-UTF8 file")?;
    let line_count = content.lines().count();

    writeln!(output, "\n# --- FILE: {:?} ---\n", path)?;
    output.write_all(content.as_bytes())?;
    writeln!(output, "\n")?;

    Ok(line_count)
}
