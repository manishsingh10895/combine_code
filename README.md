# combine_code

`combine_code` is a command-line tool that merges source files into a single output file, with support for:

- Extension filtering (`--exts`)
- Respecting `.gitignore` rules
- Optional recursive traversal
- Ignoring specific directory names (`--ignore-dirs`)
- Excluding files with glob patterns (`--exclude-glob`)
- Including hidden files/directories (`--include-hidden`)
- Non-UTF8 handling modes (`--encoding-policy skip|lossy|strict`)
- Dry-run mode to preview matched files (`--dry-run`)
- Writing merged output to stdout (`--stdout`)

## Installation

```bash
cargo install combine_code
```

## Usage

```bash
combine_code [PATH] --exts rs,py [OPTIONS]
```

### Examples

Merge Rust files in the current directory:

```bash
combine_code . --exts rs
```

Merge recursively and skip `target` and `node_modules`:

```bash
combine_code . --exts rs,js --recursive --ignore-dirs target,node_modules --output merged.txt
```

Exclude generated files by glob pattern:

```bash
combine_code . --exts rs --exclude-glob "*generated*"
```

Include hidden files:

```bash
combine_code . --exts rs --include-hidden
```

Preview files that would be merged (no output file written):

```bash
combine_code . --exts rs --dry-run
```

Write merged output to stdout:

```bash
combine_code . --exts rs --stdout
```

Control non-UTF8 handling:

```bash
combine_code . --exts rs --encoding-policy skip
combine_code . --exts rs --encoding-policy lossy
combine_code . --exts rs --encoding-policy strict
```

## Options

- `--exts <LIST>`: Comma-separated file extensions to include.
- `--recursive`: Traverse subdirectories.
- `--ignore-dirs <LIST>`: Comma-separated directory names to skip.
- `--exclude-glob <LIST>`: Comma-separated glob patterns to exclude (example: `*generated*`).
- `--include-hidden`: Include dotfiles and hidden directories.
- `--encoding-policy <skip|lossy|strict>`:
  - `skip`: Skip non-UTF8 files.
  - `lossy`: Include non-UTF8 files using replacement characters.
  - `strict`: Fail immediately if any non-UTF8 file is encountered.
- `--dry-run`: Print matched files and exit without writing merged output.
- `--stdout`: Write merged content to stdout instead of `--output`.
- `--output <FILE>`: Output filename (default: `merged.txt`).

## Output format

The generated output includes a file header before each merged file and a summary section at the end with total files and lines combined.

## License

Licensed under the MIT License. See `LICENSE`.
