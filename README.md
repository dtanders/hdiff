# hdiff

Diff HTML files (or plain text) by their visible text content.

When a file is detected as HTML, its visible text is extracted before comparison — stripping tags, scripts, and styles. Plain-text files are compared as-is, so you can freely mix HTML and plain-text operands.

## Usage

```
hdiff [OPTIONS] <FILE_A> <FILE_B>
```

### Options

| Flag | Description |
|------|-------------|
| `-U <N>`, `--unified <N>` | Context lines around each change (default: 3) |
| `-q`, `--brief` | Print only whether the files differ, no diff output |
| `-a`, `--text` | Treat both files as plain text (skip HTML detection) |
| `--html-a` | Force FILE_A to be treated as HTML |
| `--html-b` | Force FILE_B to be treated as HTML |
| `--no-color` | Disable ANSI colour output |

### Exit status

| Code | Meaning |
|------|---------|
| `0` | Files are identical |
| `1` | Files differ |
| `2` | Error (e.g. file not found) |

## Examples

```sh
# Diff two HTML pages, ignoring markup/style/script changes
hdiff old.html new.html

# Diff an HTML file against a plain-text transcript
hdiff page.html transcript.txt

# Suppress diff output — just check whether they differ
hdiff -q before.html after.html

# No colours, 0 context lines
hdiff --no-color -U 0 a.html b.html
```

## Build

```sh
cargo build --release
# Binary: target/release/hdiff
```

## Test

```sh
cargo test

# Also run the optional network test (fetches a live Wikipedia page):
cargo test --features net_tests
```
