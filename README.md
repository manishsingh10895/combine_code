# combine_code

`combine_code` is a command-line tool that merges source files into a single output file, with support for:

- Extension filtering (`--exts`)
- Respecting `.gitignore` rules
- Optional recursive traversal
- Ignoring specific directory names (`--ignore-dirs`)

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

## Output format

The generated output includes a file header before each merged file and a summary section at the end with total files and lines combined.

## License

Licensed under the MIT License. See `LICENSE`.
