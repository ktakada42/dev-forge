use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

pub fn decode(token: &str) -> Result<String, String> {
    let parts: Vec<&str> = token.trim().splitn(3, '.').collect();
    if parts.len() < 2 {
        return Err("Invalid JWT: expected at least 2 parts separated by '.'".to_string());
    }

    let header = decode_part(parts[0]).map_err(|e| format!("Header decode failed: {}", e))?;
    let payload = decode_part(parts[1]).map_err(|e| format!("Payload decode failed: {}", e))?;

    let header_json: serde_json::Value =
        serde_json::from_str(&header).map_err(|e| format!("Header JSON error: {}", e))?;
    let payload_json: serde_json::Value =
        serde_json::from_str(&payload).map_err(|e| format!("Payload JSON error: {}", e))?;

    Ok(format!(
        "Header\n{}\n\nPayload\n{}",
        serde_json::to_string_pretty(&header_json).unwrap(),
        serde_json::to_string_pretty(&payload_json).unwrap()
    ))
}

fn decode_part(part: &str) -> Result<String, String> {
    let bytes = URL_SAFE_NO_PAD.decode(part).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_JWT: &str =
        "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\
         .eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIn0\
         .SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";

    #[test]
    fn decode_valid_jwt_three_parts() {
        let r = decode(VALID_JWT).unwrap();
        assert!(r.contains("\"alg\": \"HS256\""));
        assert!(r.contains("\"sub\": \"1234567890\""));
    }

    #[test]
    fn decode_valid_jwt_two_parts_no_signature() {
        // Signature part is optional — two parts must succeed.
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\
                     .eyJzdWIiOiIxMjM0NTY3ODkwIn0";
        let r = decode(token).unwrap();
        assert!(r.contains("Header"));
        assert!(r.contains("Payload"));
    }

    #[test]
    fn decode_single_part_returns_error() {
        let e = decode("nodots").unwrap_err();
        assert!(e.contains("Invalid JWT"));
    }

    #[test]
    fn decode_invalid_base64url_in_header_returns_error() {
        let e = decode("!!!invalid.eyJzdWIiOiIxIn0.sig").unwrap_err();
        assert!(e.contains("Header decode failed"));
    }

    #[test]
    fn decode_non_json_header_returns_error() {
        // "aGVsbG8" decodes to "hello" — valid base64url, invalid JSON
        let e = decode("aGVsbG8.eyJzdWIiOiIxIn0.sig").unwrap_err();
        assert!(e.contains("Header JSON error"));
    }

    #[test]
    fn decode_non_json_payload_returns_error() {
        // Valid header, but payload decodes to "hello" (not JSON)
        let e = decode("eyJhbGciOiJIUzI1NiJ9.aGVsbG8.sig").unwrap_err();
        assert!(e.contains("Payload JSON error"));
    }

    #[test]
    fn decode_trims_whitespace() {
        let r = decode(&format!("  {}  ", VALID_JWT)).unwrap();
        assert!(r.contains("Header"));
    }

    // ── decode_part ──────────────────────────────────────────────────────────

    #[test]
    fn decode_part_valid() {
        // base64url of `{"alg":"HS256"}`
        assert!(decode_part("eyJhbGciOiJIUzI1NiJ9").is_ok());
    }

    #[test]
    fn decode_part_invalid_base64url_returns_error() {
        assert!(decode_part("!!!").is_err());
    }
}
