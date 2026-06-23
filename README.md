# fnmatch-rs

A Rust port of Python's [`fnmatch`](https://docs.python.org/3/library/fnmatch.html) —
shell-style filename matching. The matching behaviour is verified against Python's
`fnmatch.fnmatchcase` across thousands of `(pattern, name)` pairs.

| Pattern | Matches |
|---|---|
| `*` | everything |
| `?` | any single character |
| `[seq]` | any character in `seq` |
| `[!seq]` | any character not in `seq` |

## Usage

```rust
use fnmatch_rs::{fnmatchcase, filter, translate};

assert!(fnmatchcase("report.txt", "*.txt"));
assert!(!fnmatchcase("report.csv", "*.txt"));
assert!(fnmatchcase("file5.log", "file[0-9].log"));
assert!(fnmatchcase("x", "[!0-9]"));

// Keep matching names.
assert_eq!(filter(&["a.txt", "b.rs", "c.txt"], "*.txt"), vec!["a.txt", "c.txt"]);

// Get the underlying (anchored) regular expression.
let _re: String = translate("*.txt");
```

## Installation

```sh
cargo add fnmatch-rs
```

```toml
[dependencies]
fnmatch-rs = "0.1"
```

Requires a Rust toolchain with 2024-edition support (Rust 1.85 or newer).

## How it works

Like `fnmatch`, this works by translating the glob into a regular expression. The simple parts
(`*` → `.*`, `?` → `.`) are easy; the real work is the `[...]` character classes — negation,
the `]`-at-start rule, ranges, empty-range removal, and escaping regex metacharacters. Python
wraps `*` runs in atomic groups to tame its backtracking engine; Rust's `regex` crate is a
linear-time automaton, so plain `.*` gives identical results.

Matching is **case-sensitive** (Python's `fnmatchcase`). Python's plain `fnmatch`, which
case-normalizes via the platform's `os.path.normcase`, is intentionally not ported — apply
case folding yourself if you need it.

## Verification

Matching is checked by differential testing against Python's `fnmatch.fnmatchcase` across thousands
of `(pattern, name)` pairs. The crate is additionally cross-checked against the vocabulary in
CPython's own upstream
[`test_fnmatch.py`](https://github.com/python/cpython/blob/v3.13.13/Lib/test/test_fnmatch.py):
35,696 `(pattern, name)` pairs agree with CPython 3.13.13.

## License

Licensed under the [MIT License](LICENSE-MIT).
