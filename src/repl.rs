use std::borrow::Cow;

use rustyline::{
    completion::Completer,
    error::ReadlineError,
    highlight::Highlighter,
    hint::{Hint, Hinter},
    history::DefaultHistory,
    validate::Validator,
    Context, Editor, Helper,
};

use crate::tools;

enum State {
    Root,
    Timestamp,
    Base64,
    Url,
    Jwt,
}

#[derive(Clone)]
struct StringHint(String);

impl Hint for StringHint {
    fn display(&self) -> &str {
        &self.0
    }
    fn completion(&self) -> Option<&str> {
        None
    }
}

#[derive(Default)]
struct ReplHelper;

impl Completer for ReplHelper {
    type Candidate = String;
}

impl Hinter for ReplHelper {
    type Hint = StringHint;

    fn hint(&self, line: &str, pos: usize, _ctx: &Context) -> Option<StringHint> {
        if pos != line.len() {
            return None;
        }
        match line {
            "/base64 " | "/url " => Some(StringHint("encode|decode".to_string())),
            _ => None,
        }
    }
}

impl Highlighter for ReplHelper {
    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(format!("\x1b[2m{}\x1b[0m", hint))
    }
}

impl Validator for ReplHelper {}
impl Helper for ReplHelper {}

pub fn run() {
    let mut rl = match Editor::<ReplHelper, DefaultHistory>::new() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            return;
        }
    };
    rl.set_helper(Some(ReplHelper));

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
            State::Root => match parse_root_command(&line) {
                Some(("base64", "encode")) => {
                    if let Some(input) = prompt_input(&mut rl) {
                        println!("{}", tools::base64::encode(&input));
                    } else {
                        break;
                    }
                }
                Some(("base64", "decode")) => {
                    if let Some(input) = prompt_input(&mut rl) {
                        match tools::base64::decode(&input) {
                            Ok(result) => println!("{}", result),
                            Err(e) => println!("Error: {}", e),
                        }
                    } else {
                        break;
                    }
                }
                Some(("url", "encode")) => {
                    if let Some(input) = prompt_input(&mut rl) {
                        println!("{}", tools::url::encode(&input));
                    } else {
                        break;
                    }
                }
                Some(("url", "decode")) => {
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

fn parse_root_command<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let without_slash = line.strip_prefix('/')?;
    let (cmd, rest) = without_slash.split_once(' ')?;
    let sub = rest.trim();
    if sub.is_empty() {
        return None;
    }
    Some((cmd, sub))
}

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

fn prompt_input(rl: &mut Editor<ReplHelper, DefaultHistory>) -> Option<String> {
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

    #[test]
    fn root_command_base64_encode() {
        assert_eq!(parse_root_command("/base64 encode"), Some(("base64", "encode")));
    }

    #[test]
    fn root_command_url_decode() {
        assert_eq!(parse_root_command("/url decode"), Some(("url", "decode")));
    }

    #[test]
    fn root_command_no_subcommand() {
        assert_eq!(parse_root_command("/base64"), None);
    }

    #[test]
    fn root_command_no_slash() {
        assert_eq!(parse_root_command("base64 encode"), None);
    }

    #[test]
    fn root_command_trailing_space_only() {
        assert_eq!(parse_root_command("/base64 "), None);
    }
}
