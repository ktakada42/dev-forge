//! The interactive REPL.
//!
//! Two questions are asked with a list rather than a command: which tool, and
//! — for the tools that go both ways — which direction. dev-forge only ever
//! accepts a handful of answers to each, so a list shows all of them at once
//! and costs one keystroke, where a typed `/base64` had to be known in
//! advance and spelled right.
//!
//! Once a direction is chosen it stays chosen, and every line typed after it
//! is data. That is what a converter is for: paste, read, paste the next one.
//! It also means nothing typed at the prompt is a command — an empty line
//! returns to the tool list, and Ctrl-D leaves — so a payload that happens to
//! read like `exit` is encoded rather than obeyed.

use rustyline::{error::ReadlineError, Cmd, Config, DefaultEditor, KeyCode, KeyEvent, Modifiers};

use crate::picker::{self, Item, Outcome};
use crate::tools;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tool {
    Timestamp,
    Base64,
    Url,
    Jwt,
}

const TOOLS_TITLE: &str = "Select a tool";

/// The rows of the tool list, in the order they are shown.
const TOOLS: &[Item] = &[
    Item {
        label: "timestamp",
        description: "Unix timestamp <-> datetime conversion",
    },
    Item {
        label: "base64",
        description: "Base64 encode/decode",
    },
    Item {
        label: "url",
        description: "URL encode/decode",
    },
    Item {
        label: "jwt",
        description: "JWT decode (no signature verification)",
    },
];

/// The tool each row of [`TOOLS`] stands for.
const TOOL_ORDER: &[Tool] = &[Tool::Timestamp, Tool::Base64, Tool::Url, Tool::Jwt];

const DIRECTIONS: &[Item] = &[
    Item {
        label: "encode",
        description: "text -> encoded",
    },
    Item {
        label: "decode",
        description: "encoded -> text",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Encode,
    Decode,
}

/// A tool with everything it needed to know already answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Timestamp,
    Base64(Direction),
    Url(Direction),
    Jwt,
}

/// What ends a session at the prompt.
#[derive(Debug, PartialEq, Eq)]
enum Flow {
    /// Back to the tool list.
    Back,
    /// Leave dev-forge.
    Exit,
}

pub fn run() {
    // The picker needs a terminal on both ends, so there is nothing to fall
    // back to here — but there is a command that does the same job in a pipe.
    if !picker::is_available() {
        eprintln!("Interactive mode needs a terminal.");
        eprintln!("Use the subcommand form instead, e.g. `forge base64 encode`");
        eprintln!("or `echo hello | forge base64 encode`.");
        // Asking for the REPL where it cannot run is a failed request, and a
        // script that piped into `forge` with no subcommand needs to hear it.
        std::process::exit(1);
    }

    // Escape only ever arrives as the first byte of something: an arrow key
    // is `ESC [ A`. Readline's emacs mode therefore waits forever for the
    // rest, which is why an Escape on its own used to do nothing until the
    // next keystroke — and then eat it. A timeout is what turns a lone
    // Escape into a key of its own; 100ms is long enough for the rest of a
    // real sequence to land and short enough to feel like a press.
    let config = Config::builder().keyseq_timeout(Some(100)).build();
    let mut rl = match DefaultEditor::with_config(config) {
        Ok(rl) => rl,
        Err(e) => {
            eprintln!("Failed to initialize REPL: {}", e);
            return;
        }
    };

    // Escape means at the prompt what it meant in the list the prompt was
    // reached through: back to the tool list. Left as readline found it, it
    // is the meta prefix — nothing visible happens and the next keystroke is
    // swallowed, which is the worst of both answers. `Interrupt` is the same
    // door Ctrl-C uses, so the two keys agree.
    rl.bind_sequence(KeyEvent(KeyCode::Esc, Modifiers::NONE), Cmd::Interrupt);

    crate::banner::animate();
    println!();

    // Backing out of the tool list is how dev-forge is left, so the list is
    // both where a turn starts and where the loop ends.
    while let Some(row) = choose(TOOLS_TITLE, TOOLS, "quit") {
        let Some(mode) = choose_mode(TOOL_ORDER[row]) else {
            // Backed out of the direction list: ask which tool again.
            continue;
        };

        print_intro(mode);
        if session(&mut rl, mode) == Flow::Exit {
            break;
        }
    }
}

/// Runs a picker, treating a terminal that has stopped answering as a cancel.
fn choose(title: &str, items: &[Item], cancel: &str) -> Option<usize> {
    match picker::pick(title, items, cancel) {
        Ok(Outcome::Selected(index)) => Some(index),
        Ok(Outcome::Cancelled) => None,
        Err(e) => {
            eprintln!("Error: {}", e);
            None
        }
    }
}

/// Asks for the direction, for the tools that have one to ask about.
///
/// A list with a single row is a question with one answer, so JWT — which
/// only decodes — is never asked.
fn choose_mode(tool: Tool) -> Option<Mode> {
    match tool {
        Tool::Timestamp => Some(Mode::Timestamp),
        Tool::Jwt => Some(Mode::Jwt),
        Tool::Base64 => Some(Mode::Base64(direction("base64")?)),
        Tool::Url => Some(Mode::Url(direction("url")?)),
    }
}

fn direction(title: &str) -> Option<Direction> {
    match choose(title, DIRECTIONS, "back")? {
        0 => Some(Direction::Encode),
        _ => Some(Direction::Decode),
    }
}

/// Reads lines and converts them until the user leaves the tool.
fn session(rl: &mut DefaultEditor, mode: Mode) -> Flow {
    loop {
        let line = match rl.readline(&prompt(mode)) {
            Ok(line) => line,
            // Ctrl-C and Escape back out of the tool the same way an empty
            // line does; Ctrl-D is the one that leaves.
            Err(ReadlineError::Interrupted) => return Flow::Back,
            Err(ReadlineError::Eof) => return Flow::Exit,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Flow::Exit;
            }
        };

        if line.trim().is_empty() {
            return Flow::Back;
        }
        let _ = rl.add_history_entry(&line);

        match convert(mode, line.trim()) {
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {}", e),
        }
    }
}

fn prompt(mode: Mode) -> String {
    let inner = match mode {
        Mode::Timestamp => "timestamp".to_string(),
        Mode::Jwt => "jwt decode".to_string(),
        Mode::Base64(d) => format!("base64 {}", verb(d)),
        Mode::Url(d) => format!("url {}", verb(d)),
    };
    format!("forge({})> ", inner)
}

fn verb(direction: Direction) -> &'static str {
    match direction {
        Direction::Encode => "encode",
        Direction::Decode => "decode",
    }
}

/// Applies the chosen tool to one line.
fn convert(mode: Mode, input: &str) -> Result<String, String> {
    match mode {
        Mode::Timestamp => {
            let (value, tz) = parse_timestamp_line(input);
            tools::timestamp::convert(value, tz)
        }
        Mode::Base64(Direction::Encode) => Ok(tools::base64::encode(input)),
        Mode::Base64(Direction::Decode) => tools::base64::decode(input),
        Mode::Url(Direction::Encode) => Ok(tools::url::encode(input)),
        Mode::Url(Direction::Decode) => tools::url::decode(input),
        Mode::Jwt => tools::jwt::decode(input),
    }
}

/// The few lines shown on entering a tool.
///
/// This replaces the old `/help`: what a tool accepts is worth saying once,
/// when it starts mattering, rather than leaving it behind a command that has
/// to be discovered first. The last lines are the way out, which is the one
/// thing a prompt that treats everything as data has to spell out.
fn print_intro(mode: Mode) {
    for line in intro(mode) {
        println!("{}", dim(&line));
    }
    println!();
}

fn intro(mode: Mode) -> Vec<String> {
    let mut lines: Vec<String> = match mode {
        Mode::Timestamp => vec![
            "  <timestamp> [tz]   seconds or milliseconds -> datetime".to_string(),
            "  <datetime> [tz]    datetime -> Unix seconds".to_string(),
            "  formats            2025-06-13T15:19:05+09:00, 2025-06-13 15:19:05,".to_string(),
            "                     2025/06/13 15:19:05".to_string(),
            "  timezones          Asia/Tokyo, UTC, America/New_York, +09:00".to_string(),
        ],
        Mode::Base64(Direction::Encode) => vec!["  Text to encode.".to_string()],
        Mode::Base64(Direction::Decode) => vec!["  Base64 to decode.".to_string()],
        Mode::Url(Direction::Encode) => vec!["  Text to percent-encode.".to_string()],
        Mode::Url(Direction::Decode) => vec!["  Percent-encoded text to decode.".to_string()],
        Mode::Jwt => vec!["  A JWT to decode. The signature is not verified.".to_string()],
    };
    lines.push("  esc, empty line    back to the tool list".to_string());
    lines.push("  ctrl-d             quit".to_string());
    lines
}

fn dim(line: &str) -> String {
    if crate::banner::use_color() {
        format!("\x1b[2m{}\x1b[0m", line)
    } else {
        line.to_string()
    }
}

/// Parse "value [tz]" from a timestamp REPL line.
///
/// Rules:
///   - 1 token → value only
///   - 2 tokens, first is numeric → value + tz
///   - 2 tokens, first is not numeric → datetime without tz (e.g. "2025-06-13 15:19:05")
///   - 3+ tokens → last token is tz, rest is value
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
    fn every_row_of_the_tool_list_stands_for_a_tool() {
        assert_eq!(TOOLS.len(), TOOL_ORDER.len());
    }

    #[test]
    fn the_rows_are_labelled_with_the_names_the_subcommands_use() {
        // The list and `forge <tool>` name the same things, so what is learned
        // interactively is what gets typed in a pipe.
        let labels: Vec<&str> = TOOLS.iter().map(|i| i.label).collect();
        assert_eq!(labels, vec!["timestamp", "base64", "url", "jwt"]);
        let directions: Vec<&str> = DIRECTIONS.iter().map(|i| i.label).collect();
        assert_eq!(directions, vec!["encode", "decode"]);
    }

    #[test]
    fn a_tool_with_one_answer_is_never_asked() {
        // Both of these skip the direction list, so choosing them cannot be
        // backed out of half way — and neither opens a picker, which is what
        // lets this be tested without a terminal.
        assert_eq!(choose_mode(Tool::Timestamp), Some(Mode::Timestamp));
        assert_eq!(choose_mode(Tool::Jwt), Some(Mode::Jwt));
    }

    #[test]
    fn the_prompt_says_which_direction_is_live() {
        assert_eq!(
            prompt(Mode::Base64(Direction::Encode)),
            "forge(base64 encode)> "
        );
        assert_eq!(prompt(Mode::Url(Direction::Decode)), "forge(url decode)> ");
        assert_eq!(prompt(Mode::Timestamp), "forge(timestamp)> ");
        assert_eq!(prompt(Mode::Jwt), "forge(jwt decode)> ");
    }

    #[test]
    fn each_mode_converts_with_its_own_tool() {
        assert_eq!(
            convert(Mode::Base64(Direction::Encode), "Hello").unwrap(),
            "SGVsbG8="
        );
        assert_eq!(
            convert(Mode::Base64(Direction::Decode), "SGVsbG8=").unwrap(),
            "Hello"
        );
        assert_eq!(
            convert(Mode::Url(Direction::Encode), "hello world").unwrap(),
            "hello%20world"
        );
        assert_eq!(
            convert(Mode::Url(Direction::Decode), "hello%20world").unwrap(),
            "hello world"
        );
        assert_eq!(
            convert(Mode::Timestamp, "1749812345 UTC").unwrap(),
            "2025-06-13T10:59:05+00:00"
        );
        assert!(convert(Mode::Jwt, "not-a-jwt").is_err());
    }

    #[test]
    fn a_payload_that_reads_like_a_command_is_still_a_payload() {
        // Nothing at the prompt is a command, so the words that used to leave
        // the REPL are encoded like anything else.
        for word in ["exit", "quit", "help"] {
            assert_eq!(
                convert(Mode::Base64(Direction::Encode), word).unwrap(),
                tools::base64::encode(word)
            );
        }
    }

    #[test]
    fn every_tool_says_how_to_leave_it() {
        for mode in [
            Mode::Timestamp,
            Mode::Base64(Direction::Encode),
            Mode::Base64(Direction::Decode),
            Mode::Url(Direction::Encode),
            Mode::Url(Direction::Decode),
            Mode::Jwt,
        ] {
            let text = intro(mode).join("\n");
            assert!(text.contains("back to the tool list"), "{text}");
            assert!(text.contains("ctrl-d"), "{text}");
            // Escape does here what it did in the list, so it is named here
            // too rather than left to be discovered.
            assert!(text.contains("esc"), "{text}");
            // Plain ASCII: arrows and the like render inconsistently, and the
            // emoji-presentation ones are drawn double width.
            assert!(text.is_ascii(), "{text}");
        }
    }

    #[test]
    fn colors_are_dropped_when_they_are_not_wanted() {
        // NO_COLOR and a pipe both land here; the text survives either way.
        let line = dim("hello");
        assert!(line.contains("hello"));
    }
}
