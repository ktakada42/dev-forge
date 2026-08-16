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
//! It also means nothing typed at the prompt is a command — Escape and Ctrl-C
//! go back, Ctrl-D leaves — so a payload that happens to read like `exit` is
//! encoded rather than obeyed. A payload can span lines: Shift-Enter starts
//! another one instead of sending what is there.

use std::borrow::Cow;

use reedline::{
    default_emacs_keybindings, EditCommand, Emacs, KeyCode, KeyModifiers, Keybindings, Prompt,
    PromptEditMode, PromptHistorySearch, PromptHistorySearchStatus, Reedline, ReedlineEvent, Signal,
};

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

    let mut editor = Reedline::create()
        .with_edit_mode(Box::new(Emacs::new(keybindings())))
        // Asked once by the editor itself, and cached: without the protocol a
        // terminal cannot tell Shift-Enter from Enter, and the key that starts
        // another line would be the key that sends the line.
        .use_kitty_keyboard_enhancement(true);

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
        if session(&mut editor, mode) == Flow::Exit {
            break;
        }
    }
}

/// What a line of input is edited with.
///
/// Emacs keys, plus the two the rest of dev-forge already agreed on: Escape
/// backs out, exactly as it does in the lists, and Shift-Enter starts another
/// line instead of sending the one you are on.
fn keybindings() -> Keybindings {
    let mut keys = default_emacs_keybindings();
    let newline = ReedlineEvent::Edit(vec![EditCommand::InsertNewline]);
    keys.add_binding(KeyModifiers::SHIFT, KeyCode::Enter, newline.clone());
    // Alt-Enter does the same job for terminals that cannot report Shift-Enter
    // as distinct from Enter, which needs the kitty keyboard protocol.
    keys.add_binding(KeyModifiers::ALT, KeyCode::Enter, newline);
    keys.add_binding(KeyModifiers::NONE, KeyCode::Esc, ReedlineEvent::CtrlC);
    // Tab types a tab. The editor's own use for the key is to open a
    // completion menu, and there is nothing here to complete — while a tab is
    // an ordinary thing to find inside text that wants encoding, and pasting
    // was the only way to get one in.
    keys.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::Edit(vec![EditCommand::InsertChar('\t')]),
    );
    keys
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
fn session(editor: &mut Reedline, mode: Mode) -> Flow {
    let prompt = ForgePrompt(prompt(mode));
    loop {
        let line = match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => line,
            // Escape and Ctrl-C go back a screen; Ctrl-D — on an empty line,
            // where it means end of input rather than "delete this character"
            // — is the one that leaves. Both are what every other REPL does.
            Ok(Signal::CtrlC) => return Flow::Back,
            Ok(Signal::CtrlD) => return Flow::Exit,
            // Nothing else is bound to produce a signal here.
            Ok(_) => continue,
            Err(e) => {
                eprintln!("Error: {}", e);
                return Flow::Exit;
            }
        };

        // Enter on an empty line does nothing, as it does at every other
        // prompt. Leaving the tool used to be bound here, but Enter is the
        // key a hand presses without deciding to, and there is nothing to
        // convert either way.
        if line.trim().is_empty() {
            continue;
        }

        // Converted as typed, spaces and tabs at the ends included: they are
        // part of the payload, and `forge base64 encode " a "` keeps them too.
        // Nothing is lost by leaving them in — every tool that wants its input
        // tidied trims for itself, because the same input arrives from a pipe.
        match convert(mode, &line) {
            Ok(result) => println!("{}", result),
            Err(e) => println!("Error: {}", e),
        }
    }
}

/// The prompt, which is the whole of what dev-forge draws around the input.
///
/// Everything the line editor would add of its own — an indicator after the
/// prompt, a marker down the left of a wrapped or multi-line entry — is left
/// empty. A payload typed over two lines should look on screen like the two
/// lines it is, so that it can be read back and copied out unchanged.
struct ForgePrompt(String);

impl Prompt for ForgePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_history_search_indicator(
        &self,
        search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let failing = match search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!("({}reverse-search: {}) ", failing, search.term))
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
/// thing a prompt that treats everything as data has to spell out — no word
/// typed here will do it.
fn print_intro(mode: Mode) {
    // What the tool takes is the part worth reading, so it is the part that
    // carries a colour; the keys under it are there to be found when looked
    // for, not to compete with it.
    for line in intro(mode) {
        println!("{}", accent(&line));
    }
    for line in KEYS {
        println!("{}", dim(line));
    }
    println!();
}

/// The way out. Spelled out because no word typed at the prompt will do it.
const KEYS: &[&str] = &[
    "  esc, ctrl-c        back to the tool list",
    "  ctrl-d             quit",
];

fn intro(mode: Mode) -> Vec<String> {
    match mode {
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
    }
}

fn dim(line: &str) -> String {
    paint("\x1b[2m", line)
}

/// The amber the sparks are struck in, borrowed from the banner so the two
/// things dev-forge says in its own voice look like one voice.
fn accent(line: &str) -> String {
    paint("\x1b[38;5;214m", line)
}

fn paint(sequence: &str, line: &str) -> String {
    if crate::banner::use_color() {
        format!("{}{}\x1b[0m", sequence, line)
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
    fn whitespace_at_the_ends_of_a_payload_is_part_of_it() {
        // The prompt converts what was typed. `forge base64 encode " a "`
        // keeps the spaces, and the two ways in should not disagree.
        assert_eq!(
            convert(Mode::Base64(Direction::Encode), " a ").unwrap(),
            tools::base64::encode(" a ")
        );
        assert_eq!(
            convert(Mode::Base64(Direction::Encode), "a\tb").unwrap(),
            "YQli"
        );
        assert_eq!(
            convert(Mode::Base64(Direction::Encode), "a\nb").unwrap(),
            "YQpi"
        );
        // Timestamps are read out of the line rather than converted whole, so
        // stray whitespace around them still does no harm.
        assert_eq!(
            convert(Mode::Timestamp, "  1749812345 UTC  ").unwrap(),
            "2025-06-13T10:59:05+00:00"
        );
    }

    #[test]
    fn the_keys_that_type_a_control_character_are_bound() {
        let keys = keybindings();
        let inserts = |modifier, code| {
            matches!(
                keys.find_binding(modifier, code),
                Some(ReedlineEvent::Edit(edits)) if edits == expected_edit(code)
            )
        };
        assert!(inserts(KeyModifiers::SHIFT, KeyCode::Enter));
        assert!(inserts(KeyModifiers::ALT, KeyCode::Enter));
        assert!(inserts(KeyModifiers::NONE, KeyCode::Tab));
        // Escape backs out, the way it does in the lists.
        assert_eq!(
            keys.find_binding(KeyModifiers::NONE, KeyCode::Esc),
            Some(ReedlineEvent::CtrlC)
        );
    }

    /// The edit each of those keys is expected to make.
    fn expected_edit(code: KeyCode) -> Vec<EditCommand> {
        match code {
            KeyCode::Tab => vec![EditCommand::InsertChar('\t')],
            _ => vec![EditCommand::InsertNewline],
        }
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
    fn every_tool_says_what_it_takes() {
        for mode in [
            Mode::Timestamp,
            Mode::Base64(Direction::Encode),
            Mode::Base64(Direction::Decode),
            Mode::Url(Direction::Encode),
            Mode::Url(Direction::Decode),
            Mode::Jwt,
        ] {
            let text = intro(mode).join("\n");
            assert!(!text.trim().is_empty(), "{mode:?}");
            // Plain ASCII: arrows and the like render inconsistently, and the
            // emoji-presentation ones are drawn double width.
            assert!(text.is_ascii(), "{text}");
        }
    }

    #[test]
    fn the_way_out_is_spelled_out_under_every_tool() {
        let text = KEYS.join("\n");
        assert!(text.contains("back to the tool list"), "{text}");
        assert!(text.contains("ctrl-d"), "{text}");
        // Escape and Ctrl-C do here what they did in the list, so both are
        // named rather than left to be discovered.
        assert!(text.contains("esc, ctrl-c"), "{text}");
        assert!(text.is_ascii(), "{text}");
    }

    #[test]
    fn colors_are_dropped_when_they_are_not_wanted() {
        // NO_COLOR and a pipe both land here; the text survives either way.
        for line in [dim("hello"), accent("hello")] {
            assert!(line.contains("hello"));
        }
    }
}
