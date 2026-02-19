use anyhow::{Context, Result, anyhow};
use clap::{Parser, ValueEnum};
use globset::{Glob, GlobSet, GlobSetBuilder};
use ignore::WalkBuilder;
use std::fs::File;
use std::io::{self, BufWriter, Write};
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

    /// Exclude files/directories matching one or more glob patterns
    #[arg(long, value_delimiter = ',')]
    exclude_glob: Vec<String>,

    /// Include hidden files and directories (dot-prefixed)
    #[arg(long)]
    include_hidden: bool,

    /// Behavior when encountering non-UTF8 files
    #[arg(long, value_enum, default_value_t = EncodingPolicy::Skip)]
    encoding_policy: EncodingPolicy,

    /// Name of the resulting file
    #[arg(short, long, default_value = "merged.txt")]
    output: String,

    /// Print merged output to stdout instead of a file
    #[arg(long)]
    stdout: bool,

    /// Print files that would be merged without writing output
    #[arg(long)]
    dry_run: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum EncodingPolicy {
    Skip,
    Lossy,
    Strict,
}

/// CLI entrypoint.
///
/// Parses arguments, gathers candidate files, optionally prints a dry-run list,
/// then merges matching files into either stdout or an output file.
fn main() -> Result<()> {
    // Initialize global logging once for the CLI process.
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).context("Logging init failed")?;

    let args = Args::parse();

    if args.exts.is_empty() {
        warn!("Please provide extensions. Example: --exts py,rs");
        return Ok(());
    }

    // Resolve the full candidate list first so we can sort it for deterministic output.
    let files = collect_files(&args)?;
    info!("Found {} matching files", files.len());

    if args.dry_run {
        // Dry-run reports exactly what would be merged and exits without side effects.
        for path in &files {
            println!("{}", path.display());
        }
        info!("Dry run complete: no output was written");
        return Ok(());
    }

    // Route output to either stdout or a file with buffering for fewer syscalls.
    let mut output: Box<dyn Write> = if args.stdout {
        Box::new(BufWriter::new(io::stdout()))
    } else {
        let file = File::create(&args.output)
            .with_context(|| format!("Failed to create {}", args.output))?;
        Box::new(BufWriter::new(file))
    };

    let mut total_files = 0;
    let mut total_lines = 0;

    info!("Walking directory: {}", args.path.display());

    for path in files {
        match merge_file(&path, &mut output, args.encoding_policy) {
            Ok(lines) => {
                info!("Merged: {}", path.display());
                total_files += 1;
                total_lines += lines;
            }
            Err(e) => {
                if args.encoding_policy == EncodingPolicy::Strict {
                    return Err(e)
                        .with_context(|| format!("Strict mode failed on {}", path.display()));
                }
                warn!("Skipped {}: {}", path.display(), e);
            }
        }
    }

    // Write a small summary at the end of the file
    writeln!(output, "\n# --- SUMMARY ---")?;
    writeln!(output, "# Total Files Merged: {}", total_files)?;
    writeln!(output, "# Total Lines Combined: {}", total_lines)?;
    output.flush()?;

    if args.stdout {
        info!("Success! {} files merged to stdout", total_files);
    } else {
        info!("Success! {} files merged into {}", total_files, args.output);
    }
    Ok(())
}

/// Walks the requested directory and returns all files that match `should_process`.
///
/// Returned paths are sorted lexicographically to keep merge output deterministic.
fn collect_files(args: &Args) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(&args.path);
    builder.standard_filters(true).hidden(!args.include_hidden);

    // If recursive is NOT requested, limit depth to 1 (current dir only)
    if !args.recursive {
        builder.max_depth(Some(1));
    }

    let exclude_set = build_exclude_set(&args.exclude_glob)?;
    let mut files = Vec::new();
    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                // Reuse the central predicate to keep filtering consistent.
                if should_process(path, args, exclude_set.as_ref()) {
                    files.push(path.to_path_buf());
                }
            }
            Err(err) => warn!("Error walking path: {}", err),
        }
    }

    // Stable output independent of filesystem traversal order.
    files.sort_unstable_by(|a, b| a.to_string_lossy().cmp(&b.to_string_lossy()));
    Ok(files)
}

/// Returns `true` if a path should be merged for the current CLI arguments.
///
/// Excludes directories, excluded custom directory components, non-matching
/// extensions, and the output file itself when writing to disk.
fn should_process(path: &Path, args: &Args, exclude_set: Option<&GlobSet>) -> bool {
    if path.is_dir() {
        return false;
    }

    // Avoid recursively re-merging the generated output file when writing to disk.
    if !args.stdout && path.file_name().and_then(|n| n.to_str()) == Some(&args.output) {
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

    if let Some(set) = exclude_set {
        let rel = path.strip_prefix(&args.path).unwrap_or(path);
        if set.is_match(rel) || set.is_match(path) {
            return false;
        }
    }

    // Match extension
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| args.exts.iter().any(|wanted| wanted == ext))
        .unwrap_or(false)
}

/// Builds a `GlobSet` from the provided glob patterns, or returns `None` if no patterns were given.
/// Patterns are matched against both the full path and the path relative to the search root.
///
/// Args:
/// - `globs`: A slice of glob pattern strings to compile into a `GlobSet
/// 
/// Returns:
/// - `Ok(Some(GlobSet))` if patterns were provided and compiled successfully.
/// - `Ok(None)` if no patterns were provided, indicating no glob-based exclusions.
fn build_exclude_set(globs: &[String]) -> Result<Option<GlobSet>> {
    if globs.is_empty() {
        return Ok(None);
    }

    let mut builder = GlobSetBuilder::new();
    for pattern in globs {
        let glob = Glob::new(pattern)
            .with_context(|| format!("Invalid --exclude-glob pattern: {}", pattern))?;
        builder.add(glob);
    }

    Ok(Some(
        builder
            .build()
            .context("Failed to build exclude glob set")?,
    ))
}

/// Streams one file into the merged output and returns its line count.
///
/// Decoding behavior for non-UTF8 bytes is controlled by `policy`.
fn merge_file(path: &Path, output: &mut impl Write, policy: EncodingPolicy) -> Result<usize> {
    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let text = match policy {
        EncodingPolicy::Strict => std::str::from_utf8(&bytes)
            .context("Non-UTF8 file encountered under strict policy")?
            .to_owned(),
        EncodingPolicy::Skip => match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_owned(),
            Err(_) => return Err(anyhow!("Non-UTF8 file encountered under skip policy")),
        },
        EncodingPolicy::Lossy => String::from_utf8_lossy(&bytes).into_owned(),
    };

    writeln!(output, "\n# --- FILE: {} ---\n", path.display())?;
    let line_count = text.lines().count();
    output.write_all(text.as_bytes())?;

    if !text.ends_with('\n') {
        writeln!(output)?;
    }
    writeln!(output, "\n")?;

    Ok(line_count)
}
