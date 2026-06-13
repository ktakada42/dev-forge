use rustyline::{error::ReadlineError, DefaultEditor};

use crate::tools;

enum State {
    Root,
    Timestamp,
    Base64,
    Url,
    Jwt,
}

pub fn run() {
    let mut rl = match DefaultEditor::new() {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            return;
        }
    };

    crate::banner::animate();
    println!("\x1b[1;97mDev Forge\x1b[0m  \x1b[37mv{}\x1b[0m", env!("CARGO_PKG_VERSION"));
    println!("\x1b[37m\nA developer's workshop for everyday transformations.️\n\x1b[0m");
    println!("\x1b[37m\nType /help to see available commands.\n\x1b[0m");
    print_root_help();

    let mut state = State::Root;

    loop {
        let prompt = match state {
            State::Root => "forge> ",
            State::Timestamp => "forge(timestamp)> ",
            State::Base64 => "forge(base64)> ",
            State::Url => "forge(url)> ",
            State::Jwt => "forge(jwt)> ",
        };

        let line = match rl.readline(prompt) {
            Ok(l) => l,
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        };

        let _ = rl.add_history_entry(&line);
        let line = line.trim().to_string();

        if line.is_empty() {
            continue;
        }

        // Global navigation
        match line.as_str() {
            "/timestamp" => {
                state = State::Timestamp;
                continue;
            }
            "/base64" => {
                state = State::Base64;
                continue;
            }
            "/url" => {
                state = State::Url;
                continue;
            }
            "/jwt" => {
                state = State::Jwt;
                continue;
            }
            "/help" => {
                match state {
                    State::Root => print_root_help(),
                    State::Timestamp => print_timestamp_help(),
                    State::Base64 | State::Url => print_encode_decode_help(),
                    State::Jwt => print_jwt_help(),
                }
                continue;
            }
            "exit" | "quit" | "/exit" | "/quit" => break,
            _ => {}
        }

        match state {
            State::Root => {
                println!("Unknown command. Type /help for usage.");
            }
            State::Timestamp => {
                let (value, tz) = parse_timestamp_line(&line);
                match tools::timestamp::convert(value, tz) {
                    Ok(result) => println!("{}", result),
                    Err(e) => println!("Error: {}", e),
                }
            }
            State::Base64 => match line.as_str() {
                "encode" => {
                    if let Some(input) = prompt_input(&mut rl) {
                        println!("{}", tools::base64::encode(&input));
                    } else {
                        break;
                    }
                }
                "decode" => {
                    if let Some(input) = prompt_input(&mut rl) {
                        match tools::base64::decode(&input) {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        break;
                    }
                }
                _ => println!("Unknown command. Type /help for usage."),
            },
            State::Url => match line.as_str() {
                "encode" => {
                    if let Some(input) = prompt_input(&mut rl) {
                        println!("{}", tools::url::encode(&input));
                    } else {
                        break;
                    }
                }
                "decode" => {
                    if let Some(input) = prompt_input(&mut rl) {
                        match tools::url::decode(&input) {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        break;
                    }
                }
                _ => println!("Unknown command. Type /help for usage."),
            },
            State::Jwt => match line.as_str() {
                "decode" => {
                    if let Some(input) = prompt_input(&mut rl) {
                        match tools::jwt::decode(&input) {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        break;
                    }
                }
                _ => println!("Unknown command. Type /help for usage."),
            },
        }
    }
}

/// Parse "value [tz]" from a timestamp REPL line.
///
/// Rules:
///   - 1 token              → value only
///   - 2 tokens, first is numeric → value + tz
///   - 2 tokens, first is not numeric → datetime without tz (e.g. "2025-06-13 15:19:05")
///   - 3+ tokens            → last token is tz, rest is value
fn parse_timestamp_line(line: &str) -> (&str, Option<&str>) {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    match tokens.len() {
        0 | 1 => (line.trim(), None),
        2 => {
            if tokens[0].parse::<i64>().is_ok() {
                (tokens[0], Some(tokens[1]))
            } else {
                (line.trim(), None)
            }
        }
        _ => {
            let last_space = line.trim_end().rfind(' ').unwrap();
            (line[..last_space].trim(), Some(line[last_space + 1..].trim()))
        }
    }
}

fn prompt_input(rl: &mut DefaultEditor) -> Option<String> {
    println!("Input:");
    match rl.readline("") {
        Ok(input) => Some(input),
        Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => None,
        Err(e) => {
            eprintln!("Error: {}", e);
            None
        }
    }
}

fn print_root_help() {
    println!("Available tools:");
    println!("  /timestamp   Unix timestamp \u{21d4} datetime conversion");
    println!("  /base64      Base64 encode/decode");
    println!("  /url         URL encode/decode");
    println!("  /jwt         JWT decode");
    println!();
    println!("Commands:");
    println!("  /help        Show this help");
    println!("  exit, quit   Exit dev-forge");
    println!();
}

fn print_timestamp_help() {
    println!("Usage:");
    println!("  <timestamp>          Seconds or milliseconds \u{2192} datetime (auto-detected)");
    println!("  <timestamp> <tz>     Convert with specified timezone");
    println!("  <datetime>           datetime \u{2192} Unix timestamp (seconds)");
    println!("  <datetime> <tz>      Interpret datetime in specified timezone");
    println!();
    println!("Datetime formats:");
    println!("  2025-06-13T15:19:05+09:00");
    println!("  2025-06-13 15:19:05");
    println!("  2025/06/13 15:19:05");
    println!();
    println!("Timezone examples:");
    println!("  Asia/Tokyo, UTC, America/New_York, Europe/London, +09:00");
    println!();
    println!("Global commands:");
    println!("  /timestamp, /base64, /url, /jwt   Switch tool");
    println!("  /help                              Show this help");
    println!("  exit, quit                         Exit");
    println!();
}

fn print_encode_decode_help() {
    println!("Commands:");
    println!("  encode   Encode string");
    println!("  decode   Decode string");
    println!();
    println!("Global commands:");
    println!("  /timestamp, /base64, /url, /jwt   Switch tool");
    println!("  /help                              Show this help");
    println!("  exit, quit                         Exit");
    println!();
}

fn print_jwt_help() {
    println!("Commands:");
    println!("  decode   Decode JWT header and payload (no signature verification)");
    println!();
    println!("Global commands:");
    println!("  /timestamp, /base64, /url, /jwt   Switch tool");
    println!("  /help                              Show this help");
    println!("  exit, quit                         Exit");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_token_no_tz() {
        assert_eq!(parse_timestamp_line("1749812345"), ("1749812345", None));
    }

    #[test]
    fn two_tokens_numeric_gives_value_and_tz() {
        assert_eq!(
            parse_timestamp_line("1749812345 UTC"),
            ("1749812345", Some("UTC"))
        );
    }

    #[test]
    fn two_tokens_non_numeric_is_datetime_no_tz() {
        assert_eq!(
            parse_timestamp_line("2025-06-13 15:19:05"),
            ("2025-06-13 15:19:05", None)
        );
    }

    #[test]
    fn three_tokens_last_is_tz() {
        assert_eq!(
            parse_timestamp_line("2025-06-13 15:19:05 UTC"),
            ("2025-06-13 15:19:05", Some("UTC"))
        );
    }

    #[test]
    fn three_tokens_iana_tz() {
        assert_eq!(
            parse_timestamp_line("2025-06-13 15:19:05 Asia/Tokyo"),
            ("2025-06-13 15:19:05", Some("Asia/Tokyo"))
        );
    }

    #[test]
    fn slash_datetime_three_tokens() {
        assert_eq!(
            parse_timestamp_line("2025/06/13 15:19:05 UTC"),
            ("2025/06/13 15:19:05", Some("UTC"))
        );
    }

    #[test]
    fn extra_whitespace_trimmed() {
        let (val, tz) = parse_timestamp_line("  1749812345  UTC  ");
        assert_eq!(val, "1749812345");
        assert_eq!(tz, Some("UTC"));
    }
}
