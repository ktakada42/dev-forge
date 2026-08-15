//! Backslash escapes for text typed at the prompt.
//!
//! The prompt has no shell in front of it. At a shell there is `printf 'a\tb'`
//! and `$'a\tb'` to turn an escape into the character it names; inside the REPL
//! the only ways in were the Tab key, Shift-Enter and the clipboard, and none
//! of those reach a carriage return or a NUL. So the text a tool is asked to
//! encode is read as escaped text, the way a string literal is.
//!
//! The cost is that a backslash now means something. `C:\new` is a newline
//! where it used to be an `n`, and text that wants a real backslash has to
//! write `\\`. That is the trade every language with string literals makes,
//! and what makes it survivable here is that an escape nobody defined is an
//! error rather than a guess: `C:\Users` says so instead of quietly encoding
//! something else. The `forge` subcommands leave input exactly as it arrives,
//! so a pipe is still byte for byte.

/// Reads `input` as escaped text.
///
/// The set is the one string literals use, plus the shell's `\e`.
pub fn unescape(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(escape) = chars.next() else {
            return Err("Escape error: the line ends with a lone `\\`".to_string());
        };
        match escape {
            'n' => out.push('\n'),
            't' => out.push('\t'),
            'r' => out.push('\r'),
            '0' => out.push('\0'),
            'a' => out.push('\u{7}'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'v' => out.push('\u{b}'),
            // `\e` is not a string-literal escape, but every shell has it and
            // an escape character is hard to type any other way.
            'e' => out.push('\u{1b}'),
            '\\' => out.push('\\'),
            'x' => out.push(hex_byte(&mut chars)?),
            'u' => out.push(unicode(&mut chars)?),
            other => {
                return Err(format!(
                    "Escape error: `\\{other}` is not an escape (write `\\\\` for a backslash)"
                ))
            }
        }
    }
    Ok(out)
}

/// `\xNN`, ASCII only — the same limit string literals put on it.
///
/// Anything above 0x7F is half of a character rather than a character, and
/// what dev-forge carries from here on is text.
fn hex_byte(chars: &mut std::str::Chars) -> Result<char, String> {
    let digits: String = chars.take(2).collect();
    if digits.chars().count() != 2 {
        return Err("Escape error: `\\x` needs two hex digits, as in `\\x1b`".to_string());
    }
    let value = u8::from_str_radix(&digits, 16)
        .map_err(|_| format!("Escape error: `\\x{digits}` is not two hex digits"))?;
    if value > 0x7f {
        return Err(format!(
            "Escape error: `\\x{digits}` is above \\x7f; write it as `\\u{{{digits}}}`"
        ));
    }
    Ok(value as char)
}

/// `\u{...}`, one to six hex digits naming a character.
fn unicode(chars: &mut std::str::Chars) -> Result<char, String> {
    if chars.next() != Some('{') {
        return Err("Escape error: `\\u` needs braces, as in `\\u{1f600}`".to_string());
    }
    let mut digits = String::new();
    for c in chars.by_ref() {
        if c == '}' {
            break;
        }
        digits.push(c);
    }
    if digits.is_empty() || digits.len() > 6 {
        return Err("Escape error: `\\u{...}` needs one to six hex digits".to_string());
    }
    let value = u32::from_str_radix(&digits, 16)
        .map_err(|_| format!("Escape error: `\\u{{{digits}}}` is not hex digits"))?;
    char::from_u32(value)
        .ok_or_else(|| format!("Escape error: `\\u{{{digits}}}` is not a character"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_without_backslashes_is_unchanged() {
        assert_eq!(unescape("hello world").unwrap(), "hello world");
        assert_eq!(unescape("").unwrap(), "");
        assert_eq!(unescape("あ 漢字 🙂").unwrap(), "あ 漢字 🙂");
    }

    #[test]
    fn the_common_escapes_become_control_characters() {
        assert_eq!(unescape("a\\nb").unwrap(), "a\nb");
        assert_eq!(unescape("a\\tb").unwrap(), "a\tb");
        assert_eq!(unescape("a\\r\\nb").unwrap(), "a\r\nb");
        assert_eq!(unescape("a\\0b").unwrap(), "a\0b");
        assert_eq!(unescape("\\a\\b\\f\\v").unwrap(), "\u{7}\u{8}\u{c}\u{b}");
        assert_eq!(unescape("\\e[0m").unwrap(), "\u{1b}[0m");
    }

    #[test]
    fn a_doubled_backslash_is_a_backslash() {
        assert_eq!(unescape("C:\\\\new").unwrap(), "C:\\new");
        assert_eq!(unescape("\\\\\\\\").unwrap(), "\\\\");
        // The escape is consumed whole, so the `n` after it stays an `n`.
        assert_eq!(unescape("\\\\n").unwrap(), "\\n");
    }

    #[test]
    fn hex_escapes_name_ascii_characters() {
        assert_eq!(unescape("\\x1b").unwrap(), "\u{1b}");
        assert_eq!(unescape("\\x41BC").unwrap(), "ABC");
    }

    #[test]
    fn a_hex_escape_past_ascii_is_refused_with_the_way_to_write_it() {
        let e = unescape("\\xff").unwrap_err();
        assert!(e.contains("\\u{ff}"), "{e}");
    }

    #[test]
    fn unicode_escapes_name_any_character() {
        assert_eq!(unescape("\\u{3042}").unwrap(), "あ");
        assert_eq!(unescape("\\u{1f600}").unwrap(), "\u{1f600}");
        assert_eq!(unescape("a\\u{41}b").unwrap(), "aAb");
    }

    #[test]
    fn an_escape_nobody_defined_is_an_error_rather_than_a_guess() {
        // This is what stops `C:\Users\me` from being encoded as something
        // else without a word about it.
        let e = unescape("C:\\Users").unwrap_err();
        assert!(e.contains("\\U"), "{e}");
        assert!(e.contains("\\\\"), "{e}");
    }

    #[test]
    fn a_lone_trailing_backslash_is_an_error() {
        let e = unescape("end\\").unwrap_err();
        assert!(e.contains("lone"), "{e}");
    }

    #[test]
    fn malformed_hex_and_unicode_escapes_say_what_was_wanted() {
        assert!(unescape("\\x").unwrap_err().contains("two hex digits"));
        assert!(unescape("\\xzz").unwrap_err().contains("two hex digits"));
        assert!(unescape("\\u1234").unwrap_err().contains("braces"));
        assert!(unescape("\\u{}").unwrap_err().contains("one to six"));
        assert!(unescape("\\u{1234567}").unwrap_err().contains("one to six"));
        // A surrogate half is hex, and is not a character.
        assert!(unescape("\\u{d800}").unwrap_err().contains("not a character"));
    }
}
