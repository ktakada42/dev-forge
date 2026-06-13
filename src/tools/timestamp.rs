use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;

const MILLIS_THRESHOLD: i64 = 1_000_000_000_000;

pub fn convert(input: &str, tz: Option<&str>) -> Result<String, String> {
    let trimmed = input.trim();
    if let Ok(ts) = trimmed.parse::<i64>() {
        return timestamp_to_datetime(ts, tz);
    }
    datetime_to_timestamp(trimmed, tz)
}

// ─── internal ────────────────────────────────────────────────────────────────

fn timestamp_to_datetime(ts: i64, tz: Option<&str>) -> Result<String, String> {
    let (secs, millis) = if ts >= MILLIS_THRESHOLD {
        (ts / 1000, ts % 1000)
    } else {
        (ts, 0)
    };

    let utc_dt = DateTime::from_timestamp(secs, (millis * 1_000_000) as u32)
        .ok_or_else(|| "Invalid timestamp".to_string())?;

    let (date_str, tz_str) = match tz {
        None => fmt_with_local(&utc_dt),
        Some(name) => fmt_with_tz_name(&utc_dt, name)?,
    };

    Ok(if millis == 0 {
        format!("{}{}", date_str, tz_str)
    } else {
        format!("{}.{:03}{}", date_str, millis, tz_str)
    })
}

fn datetime_to_timestamp(input: &str, tz: Option<&str>) -> Result<String, String> {
    // RFC3339 carries its own timezone — ignore `tz` argument.
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Ok(dt.timestamp().to_string());
    }

    for fmt in &["%Y-%m-%d %H:%M:%S", "%Y/%m/%d %H:%M:%S"] {
        if let Ok(naive) = NaiveDateTime::parse_from_str(input, fmt) {
            let ts = match tz {
                None => naive_to_ts(naive, Local)?,
                Some(name) => {
                    if let Ok(named) = name.parse::<Tz>() {
                        naive_to_ts(naive, named)?
                    } else if let Some(fixed) = parse_fixed_offset(name) {
                        naive_to_ts(naive, fixed)?
                    } else {
                        return Err(tz_error(name));
                    }
                }
            };
            return Ok(ts.to_string());
        }
    }

    Err(format!("Cannot parse: {}", input))
}

// ─── timezone helpers ─────────────────────────────────────────────────────────

fn fmt_with_local(utc: &DateTime<Utc>) -> (String, String) {
    let dt = utc.with_timezone(&Local);
    (
        dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
        dt.format("%:z").to_string(),
    )
}

fn fmt_with_tz_name(utc: &DateTime<Utc>, name: &str) -> Result<(String, String), String> {
    if let Ok(tz) = name.parse::<Tz>() {
        let dt = utc.with_timezone(&tz);
        Ok((
            dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            dt.format("%:z").to_string(),
        ))
    } else if let Some(fixed) = parse_fixed_offset(name) {
        let dt = utc.with_timezone(&fixed);
        Ok((
            dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
            dt.format("%:z").to_string(),
        ))
    } else {
        Err(tz_error(name))
    }
}

fn naive_to_ts<T: TimeZone>(naive: NaiveDateTime, tz: T) -> Result<i64, String> {
    naive
        .and_local_timezone(tz)
        .single()
        .map(|dt| dt.timestamp())
        .ok_or_else(|| "Ambiguous datetime in specified timezone".to_string())
}

pub(crate) fn parse_fixed_offset(s: &str) -> Option<FixedOffset> {
    let s = s.trim();
    if s.len() < 6 {
        return None;
    }
    let (sign, rest) = match s.chars().next()? {
        '+' => (1i32, &s[1..]),
        '-' => (-1i32, &s[1..]),
        _ => return None,
    };
    let mut parts = rest.splitn(2, ':');
    let hours: i32 = parts.next()?.parse().ok()?;
    let mins: i32 = parts.next()?.parse().ok()?;
    FixedOffset::east_opt(sign * (hours * 3600 + mins * 60))
}

fn tz_error(name: &str) -> String {
    format!(
        "Unknown timezone: '{}'. Use IANA name (e.g. Asia/Tokyo, UTC) or UTC offset (e.g. +09:00)",
        name
    )
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── routing ──────────────────────────────────────────────────────────────

    #[test]
    fn numeric_input_routes_to_timestamp_to_datetime() {
        let r = convert("1749812345", Some("UTC")).unwrap();
        assert!(r.contains('T'), "should contain T separator");
    }

    #[test]
    fn nonnumeric_input_routes_to_datetime_to_timestamp() {
        let r = convert("2025-06-13T10:59:05+00:00", None).unwrap();
        assert_eq!(r, "1749812345");
    }

    // ── timestamp → datetime ─────────────────────────────────────────────────

    #[test]
    fn seconds_to_utc() {
        assert_eq!(
            convert("1749812345", Some("UTC")).unwrap(),
            "2025-06-13T10:59:05+00:00"
        );
    }

    #[test]
    fn millis_exact_to_utc() {
        // 1749812345000 ms == 1749812345 s, zero fractional part
        assert_eq!(
            convert("1749812345000", Some("UTC")).unwrap(),
            "2025-06-13T10:59:05+00:00"
        );
    }

    #[test]
    fn millis_fractional_to_utc() {
        assert_eq!(
            convert("1749812345678", Some("UTC")).unwrap(),
            "2025-06-13T10:59:05.678+00:00"
        );
    }

    #[test]
    fn millis_threshold_boundary() {
        // MILLIS_THRESHOLD itself is treated as ms → 1_000_000_000 s (2001-09-09)
        assert_eq!(
            convert("1000000000000", Some("UTC")).unwrap(),
            "2001-09-09T01:46:40+00:00"
        );
    }

    #[test]
    fn timestamp_with_iana_tz() {
        assert_eq!(
            convert("1749812345", Some("Asia/Tokyo")).unwrap(),
            "2025-06-13T19:59:05+09:00"
        );
    }

    #[test]
    fn timestamp_with_fixed_offset_positive() {
        assert_eq!(
            convert("1749812345", Some("+05:30")).unwrap(),
            "2025-06-13T16:29:05+05:30"
        );
    }

    #[test]
    fn timestamp_with_fixed_offset_negative() {
        assert_eq!(
            convert("1749812345", Some("-05:00")).unwrap(),
            "2025-06-13T05:59:05-05:00"
        );
    }

    #[test]
    fn timestamp_with_invalid_tz_returns_error() {
        let e = convert("1749812345", Some("Nowhere")).unwrap_err();
        assert!(e.contains("Unknown timezone"));
    }

    #[test]
    fn timestamp_local_tz_returns_valid_format() {
        // Local timezone is non-deterministic; just verify shape.
        let r = convert("1749812345", None).unwrap();
        assert_eq!(&r[10..11], "T");
        assert!(r.len() >= 25);
    }

    // ── datetime → timestamp ─────────────────────────────────────────────────

    #[test]
    fn rfc3339_to_timestamp() {
        assert_eq!(
            convert("2025-06-13T10:59:05+00:00", None).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn rfc3339_ignores_tz_argument() {
        // RFC3339 carries its own offset; --tz should not override it.
        assert_eq!(
            convert("2025-06-13T10:59:05+00:00", Some("Asia/Tokyo")).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn space_datetime_with_utc() {
        // 2025-06-13 10:59:05 UTC == 1749812345
        assert_eq!(
            convert("2025-06-13 10:59:05", Some("UTC")).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn slash_datetime_with_utc() {
        assert_eq!(
            convert("2025/06/13 10:59:05", Some("UTC")).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn space_datetime_with_iana_tz() {
        // 2025-06-13 19:59:05 Asia/Tokyo == UTC 10:59:05 == 1749812345
        assert_eq!(
            convert("2025-06-13 19:59:05", Some("Asia/Tokyo")).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn space_datetime_with_fixed_offset() {
        assert_eq!(
            convert("2025-06-13 19:59:05", Some("+09:00")).unwrap(),
            "1749812345"
        );
    }

    #[test]
    fn space_datetime_with_invalid_tz_returns_error() {
        let e = convert("2025-06-13 10:59:05", Some("Nowhere")).unwrap_err();
        assert!(e.contains("Unknown timezone"));
    }

    #[test]
    fn space_datetime_local_tz_returns_ok() {
        // Local TZ is non-deterministic; just verify it succeeds.
        assert!(convert("2025-06-13 10:59:05", None).is_ok());
    }

    #[test]
    fn unparseable_input_returns_error() {
        let e = convert("not-a-date", None).unwrap_err();
        assert!(e.contains("Cannot parse"));
    }

    // ── parse_fixed_offset ───────────────────────────────────────────────────

    #[test]
    fn fixed_offset_positive() {
        assert_eq!(parse_fixed_offset("+09:00").unwrap().local_minus_utc(), 9 * 3600);
    }

    #[test]
    fn fixed_offset_negative() {
        assert_eq!(parse_fixed_offset("-05:00").unwrap().local_minus_utc(), -5 * 3600);
    }

    #[test]
    fn fixed_offset_zero() {
        assert_eq!(parse_fixed_offset("+00:00").unwrap().local_minus_utc(), 0);
    }

    #[test]
    fn fixed_offset_fractional_hour() {
        assert_eq!(
            parse_fixed_offset("+05:30").unwrap().local_minus_utc(),
            5 * 3600 + 30 * 60
        );
    }

    #[test]
    fn fixed_offset_too_short_returns_none() {
        assert!(parse_fixed_offset("+9:0").is_none());
    }

    #[test]
    fn fixed_offset_no_sign_returns_none() {
        assert!(parse_fixed_offset("09:00").is_none());
    }

    #[test]
    fn fixed_offset_no_colon_returns_none() {
        assert!(parse_fixed_offset("+0900").is_none());
    }

    #[test]
    fn fixed_offset_non_numeric_returns_none() {
        assert!(parse_fixed_offset("+ab:cd").is_none());
    }

    #[test]
    fn fixed_offset_out_of_range_returns_none() {
        assert!(parse_fixed_offset("+25:00").is_none());
    }
}
