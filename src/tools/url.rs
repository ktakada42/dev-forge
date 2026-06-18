pub fn encode(input: &str) -> String {
    urlencoding::encode(input).into_owned()
}

pub fn decode(input: &str) -> Result<String, String> {
    urlencoding::decode(input.trim())
        .map(|s| s.into_owned())
        .map_err(|e| format!("Decode error: {}", e))
}

/// Auto-detect encode or decode. Returns (operation, result).
///
/// Decode is chosen when the input contains at least one valid
/// percent-encoded sequence (%XX where XX is two hex digits).
pub fn auto(input: &str) -> (&'static str, Result<String, String>) {
    if should_decode(input) {
        ("decode", decode(input))
    } else {
        ("encode", Ok(encode(input)))
    }
}

fn should_decode(input: &str) -> bool {
    let bytes = input.as_bytes();
    for i in 0..bytes.len().saturating_sub(2) {
        if bytes[i] == b'%'
            && bytes[i + 1].is_ascii_hexdigit()
            && bytes[i + 2].is_ascii_hexdigit()
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_space() {
        assert_eq!(encode("hello world"), "hello%20world");
    }

    #[test]
    fn encode_special_chars() {
        assert_eq!(encode("a=1&b=2"), "a%3D1%26b%3D2");
    }

    #[test]
    fn encode_already_percent_encoded() {
        // '%' itself is encoded
        assert_eq!(encode("hello%20world"), "hello%2520world");
    }

    #[test]
    fn decode_valid() {
        assert_eq!(decode("hello%20world").unwrap(), "hello world");
    }

    #[test]
    fn decode_trims_whitespace() {
        assert_eq!(decode("  hello%20world  ").unwrap(), "hello world");
    }

    #[test]
    fn decode_invalid_utf8_sequence_returns_error() {
        // %FF%FE decodes to bytes [0xFF, 0xFE] which are invalid UTF-8
        let e = decode("%FF%FE").unwrap_err();
        assert!(e.contains("Decode error"));
    }

    #[test]
    fn auto_decodes_percent_encoded_string() {
        let (op, result) = auto("hello%20world");
        assert_eq!(op, "decode");
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn auto_encodes_plain_text() {
        let (op, result) = auto("hello world");
        assert_eq!(op, "encode");
        assert_eq!(result.unwrap(), "hello%20world");
    }

    #[test]
    fn auto_encodes_bare_percent_without_hex() {
        // "100%" has % not followed by two hex digits — treat as plain text
        let (op, _) = auto("100%");
        assert_eq!(op, "encode");
    }

    #[test]
    fn auto_encodes_no_special_chars() {
        let (op, _) = auto("hello");
        assert_eq!(op, "encode");
    }

    #[test]
    fn auto_decodes_mixed_encoded_string() {
        let (op, result) = auto("a%3D1%26b%3D2");
        assert_eq!(op, "decode");
        assert_eq!(result.unwrap(), "a=1&b=2");
    }
}
