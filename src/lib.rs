//! A Rust port of Python's [`fnmatch`](https://docs.python.org/3/library/fnmatch.html) —
//! shell-style filename matching (`*`, `?`, `[seq]`, `[!seq]`).
//!
//! Port target: Python's standard library `fnmatch.py`. The matching behaviour is verified
//! against Python's `fnmatch.fnmatchcase` over a large corpus of `(pattern, name)` pairs.
//!
//! Like `fnmatch`, this works by translating the glob to a regular expression. Python wraps
//! runs of `*` in atomic groups to tame its backtracking engine; Rust's `regex` crate is a
//! linear-time automaton, so plain `.*` gives identical results with simpler output.

use regex::Regex;

/// Translate a shell glob pattern into an anchored regular expression (Rust `regex` syntax).
///
/// Port of `fnmatch.translate`, emitting a Rust-compatible regex instead of Python's. The
/// resulting expression matches a whole string (it is anchored with `\A … \z` and uses the
/// `s` flag so `.` matches newlines, just like the reference).
pub fn translate(pat: &str) -> String {
    let chars: Vec<char> = pat.chars().collect();
    let n = chars.len();
    let mut body = String::new();
    let mut last_was_star = false;
    let mut i = 0;

    while i < n {
        let c = chars[i];
        i += 1;

        if c == '*' {
            // Compress consecutive `*` into a single `.*`.
            if !last_was_star {
                body.push_str(".*");
            }
            last_was_star = true;
            continue;
        }
        last_was_star = false;

        match c {
            '?' => body.push('.'),
            '[' => i = translate_class(&chars, i, n, &mut body),
            _ => body.push_str(&regex::escape(&c.to_string())),
        }
    }

    format!("(?s)\\A{body}\\z")
}

/// Translate a `[...]` character class starting at `i` (the index just past `[`). Appends the
/// translation to `body` and returns the index to continue scanning from. Port of the `[`
/// branch of `fnmatch._translate`.
fn translate_class(chars: &[char], i: usize, n: usize, body: &mut String) -> usize {
    // Find the closing `]`, allowing a leading `!` and/or a literal `]` right after `[`.
    let mut j = i;
    if j < n && chars[j] == '!' {
        j += 1;
    }
    if j < n && chars[j] == ']' {
        j += 1;
    }
    while j < n && chars[j] != ']' {
        j += 1;
    }

    // Unterminated `[` is a literal bracket; keep scanning from `i`.
    if j >= n {
        body.push_str("\\[");
        return i;
    }

    let stuff = build_class_stuff(chars, i, j);
    let next = j + 1;

    if stuff.is_empty() {
        body.push_str("[^\\s\\S]"); // empty class: never matches (Rust has no `(?!)` lookahead)
    } else if stuff == "!" {
        body.push('.'); // negated empty class: matches any character
    } else {
        let mut s = stuff;
        let first = s.chars().next().unwrap();
        if first == '!' {
            s = format!("^{}", &s['!'.len_utf8()..]);
        } else if first == '^' || first == '[' {
            s = format!("\\{s}");
        }
        body.push('[');
        body.push_str(&s);
        body.push(']');
    }
    next
}

/// Build the inner text of a character class (between the brackets), handling ranges and
/// escaping. Port of the `stuff = ...` logic in `fnmatch._translate`.
fn build_class_stuff(chars: &[char], i_start: usize, j: usize) -> String {
    let content: String = chars[i_start..j].iter().collect();

    let stuff = if !content.contains('-') {
        content.replace('\\', "\\\\")
    } else {
        // Split the content into chunks around ranges, drop empty ranges, then rejoin.
        let mut chunks: Vec<String> = Vec::new();
        let mut i = i_start;
        let mut k = if chars[i_start] == '!' {
            i_start + 2
        } else {
            i_start + 1
        };
        loop {
            let found = (k..j).find(|&idx| chars[idx] == '-');
            match found {
                None => break,
                Some(pos) => {
                    chunks.push(chars[i..pos].iter().collect());
                    i = pos + 1;
                    k = pos + 3;
                }
            }
        }
        let tail: String = chars[i..j].iter().collect();
        if !tail.is_empty() {
            chunks.push(tail);
        } else if let Some(last) = chunks.last_mut() {
            last.push('-');
        }

        // Remove empty ranges (e.g. `z-a`), which are invalid in a regex.
        let mut idx = chunks.len();
        while idx > 1 {
            idx -= 1;
            let prev_last = chunks[idx - 1].chars().last();
            let cur_first = chunks[idx].chars().next();
            if let (Some(pl), Some(cf)) = (prev_last, cur_first)
                && pl > cf
            {
                let prev_trimmed: String = {
                    let count = chunks[idx - 1].chars().count();
                    chunks[idx - 1].chars().take(count - 1).collect()
                };
                let cur_rest: String = chunks[idx].chars().skip(1).collect();
                chunks[idx - 1] = prev_trimmed + &cur_rest;
                chunks.remove(idx);
            }
        }

        chunks
            .iter()
            .map(|s| s.replace('\\', "\\\\").replace('-', "\\-"))
            .collect::<Vec<_>>()
            .join("-")
    };

    // Escape characters that are regex set operators (`&`, `~`, `|`).
    let mut result = String::new();
    for ch in stuff.chars() {
        if matches!(ch, '&' | '~' | '|') {
            result.push('\\');
        }
        result.push(ch);
    }
    result
}

/// Whether `name` matches the glob `pat`, case-sensitively. Port of `fnmatch.fnmatchcase`.
pub fn fnmatchcase(name: &str, pat: &str) -> bool {
    Regex::new(&translate(pat)).is_ok_and(|re| re.is_match(name))
}

/// Keep the names that match the glob `pat`. Port of `fnmatch.filter` (case-sensitive).
pub fn filter<'a>(names: &[&'a str], pat: &str) -> Vec<&'a str> {
    match Regex::new(&translate(pat)) {
        Ok(re) => names.iter().copied().filter(|n| re.is_match(n)).collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn star_and_question() {
        assert!(fnmatchcase("foo.txt", "*.txt"));
        assert!(!fnmatchcase("foo.py", "*.txt"));
        assert!(fnmatchcase("abcXYZdef", "abc*def"));
        assert!(fnmatchcase("a", "?"));
        assert!(!fnmatchcase("ab", "?"));
        assert!(fnmatchcase("axc", "a?c"));
        assert!(fnmatchcase("anything", "*"));
        assert!(fnmatchcase("", "*"));
    }

    #[test]
    fn character_classes() {
        assert!(fnmatchcase("a", "[abc]"));
        assert!(!fnmatchcase("d", "[abc]"));
        assert!(fnmatchcase("m", "[a-z]"));
        assert!(!fnmatchcase("M", "[a-z]")); // case-sensitive
        assert!(fnmatchcase("file5.txt", "file[0-9].txt"));
        assert!(!fnmatchcase("fileX.txt", "file[0-9].txt"));
    }

    #[test]
    fn negated_classes() {
        assert!(fnmatchcase("d", "[!abc]"));
        assert!(!fnmatchcase("a", "[!abc]"));
        assert!(fnmatchcase("x", "[!0-9]"));
        assert!(!fnmatchcase("5", "[!0-9]"));
    }

    #[test]
    fn literals_and_specials() {
        // regex metacharacters in the pattern are literal
        assert!(fnmatchcase("a.b", "a.b"));
        assert!(!fnmatchcase("axb", "a.b")); // the '.' is literal, not "any char"
        assert!(fnmatchcase("a+b", "a+b"));
        assert!(fnmatchcase("(x)", "(x)"));
    }

    #[test]
    fn filter_keeps_matches() {
        assert_eq!(
            filter(&["a.txt", "b.py", "c.txt"], "*.txt"),
            vec!["a.txt", "c.txt"]
        );
    }
}
