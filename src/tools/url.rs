pub fn encode(input: &str) -> String {
    urlencoding::encode(input).into_owned()
}

pub fn decode(input: &str) -> Result<String, String> {
    urlencoding::decode(input.trim())
        .map(|s| s.into_owned())
        .map_err(|e| format!("Decode error: {}", e))
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
}
