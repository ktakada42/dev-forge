use base64::{engine::general_purpose::STANDARD, Engine};

pub fn encode(input: &str) -> String {
    STANDARD.encode(input)
}

pub fn decode(input: &str) -> Result<String, String> {
    let bytes = STANDARD
        .decode(input.trim())
        .map_err(|e| format!("Decode error: {}", e))?;
    String::from_utf8(bytes).map_err(|e| format!("UTF-8 error: {}", e))
}

/// Auto-detect encode or decode. Returns (operation, result).
///
/// Decode is chosen when all of these hold:
///   1. Only valid base64 chars: [A-Za-z0-9+/=]
///   2. Length is a multiple of 4
///   3. Decodes without error
///   4. Decoded bytes are valid UTF-8 with no non-printable control chars
pub fn auto(input: &str) -> (&'static str, Result<String, String>) {
    if should_decode(input) {
        ("decode", decode(input))
    } else {
        ("encode", Ok(encode(input)))
    }
}

fn should_decode(input: &str) -> bool {
    if !input
        .chars()
        .all(|c| matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '+' | '/' | '='))
    {
        return false;
    }
    if input.len() % 4 != 0 {
        return false;
    }
    let bytes = match STANDARD.decode(input) {
        Ok(b) => b,
        Err(_) => return false,
    };
    match std::str::from_utf8(&bytes) {
        Ok(s) => s
            .chars()
            .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t')),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_ascii() {
        assert_eq!(encode("Hello"), "SGVsbG8=");
    }

    #[test]
    fn encode_empty() {
        assert_eq!(encode(""), "");
    }

    #[test]
    fn encode_unicode() {
        assert_eq!(encode("あ"), "44GC");
    }

    #[test]
    fn decode_valid() {
        assert_eq!(decode("SGVsbG8=").unwrap(), "Hello");
    }

    #[test]
    fn decode_trims_whitespace() {
        assert_eq!(decode("  SGVsbG8=  ").unwrap(), "Hello");
    }

    #[test]
    fn decode_invalid_base64_returns_error() {
        let e = decode("not!!valid").unwrap_err();
        assert!(e.contains("Decode error"));
    }

    #[test]
    fn decode_valid_base64_invalid_utf8_returns_error() {
        // [0xFF, 0xFE] encodes to "//4=" — valid base64, invalid UTF-8
        let e = decode("//4=").unwrap_err();
        assert!(e.contains("UTF-8 error"));
    }

    #[test]
    fn auto_decodes_valid_base64() {
        let (op, result) = auto("SGVsbG8=");
        assert_eq!(op, "decode");
        assert_eq!(result.unwrap(), "Hello");
    }

    #[test]
    fn auto_decodes_padded_base64() {
        let (op, result) = auto("dGVzdA==");
        assert_eq!(op, "decode");
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn auto_encodes_plain_text_with_space() {
        let (op, result) = auto("hello world");
        assert_eq!(op, "encode");
        assert_eq!(result.unwrap(), "aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn auto_encodes_when_non_multiple_of_4() {
        // "Hello" is 5 chars — not a multiple of 4
        let (op, _) = auto("Hello");
        assert_eq!(op, "encode");
    }

    #[test]
    fn auto_encodes_when_decode_yields_non_utf8() {
        // "test" passes charset and length checks but decodes to invalid UTF-8
        let (op, _) = auto("test");
        assert_eq!(op, "encode");
    }

    #[test]
    fn auto_encodes_japanese_text() {
        let (op, _) = auto("こんにちは");
        assert_eq!(op, "encode");
    }
}
