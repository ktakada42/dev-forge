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
}
