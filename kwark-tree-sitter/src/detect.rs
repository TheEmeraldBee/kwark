/// Returns the interpreter name from a shebang line, or `None` if it isn't one
///
/// Strips any leading path down to the file name, and resolves `env` to the
/// interpreter it invokes (`#!/usr/bin/env python3` -> `python3`)
pub fn parse_shebang(line: &str) -> Option<&str> {
    let rest = line.strip_prefix("#!")?;

    let mut parts = rest.split_whitespace();
    let mut interpreter = file_name(parts.next()?);

    if interpreter == "env" {
        interpreter = file_name(parts.next()?);
    }

    if interpreter.is_empty() {
        return None;
    }

    Some(interpreter)
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_interpreter_path() {
        assert_eq!(parse_shebang("#!/bin/sh"), Some("sh"));
    }

    #[test]
    fn env_interpreter() {
        assert_eq!(parse_shebang("#!/usr/bin/env python3"), Some("python3"));
    }

    #[test]
    fn not_a_shebang() {
        assert_eq!(parse_shebang("fn main() {}"), None);
    }

    #[test]
    fn empty_shebang() {
        assert_eq!(parse_shebang("#!"), None);
    }
}
