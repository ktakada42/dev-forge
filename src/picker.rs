//! Inline single-choice picker for the REPL.
//!
//! The list is drawn where the cursor already is — a few lines under the last
//! output — rather than on an alternate screen. A REPL is a transcript, and
//! taking the whole terminal away to ask "which tool?" would hide the answer
//! the previous command just printed. Every frame is redrawn in place, and
//! once something is chosen the block collapses into a single line that stays
//! in the transcript as a record of what was picked:
//!
//! ```text
//! ? Select a tool  base64
//! ```

use std::io::{self, IsTerminal, Write};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::{cursor, queue, style, terminal};

/// One row of the list.
pub struct Item {
    pub label: &'static str,
    pub description: &'static str,
}

/// How the picker ended.
pub enum Outcome {
    Selected(usize),
    /// Escape or Ctrl-C; the caller decides what backing out means.
    Cancelled,
}

/// `true` when there is a terminal to draw the list on and read keys from.
///
/// Both ends matter: keys come from stdin and the frames go to stdout, so a
/// pipe on either side rules the picker out.
pub fn is_available() -> bool {
    io::stdin().is_terminal() && io::stdout().is_terminal() && !dumb_terminal()
}

fn dumb_terminal() -> bool {
    std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false)
}

/// Runs the picker until a row is chosen or the user backs out.
///
/// `cancel` names what Escape does here, because it differs by list: backing
/// out of the tool list leaves dev-forge, backing out of an action list only
/// returns to the tool list.
pub fn pick(title: &str, items: &[Item], cancel: &str) -> io::Result<Outcome> {
    let mut out = io::stdout();
    let mut list = List::new(items);
    let mut drawn = 0usize;

    let outcome = {
        let _raw = Raw::enter()?;
        queue!(out, cursor::Hide)?;
        loop {
            drawn = draw(&mut out, title, &mut list, cancel, size(), drawn)?;

            let Event::Key(key) = event::read()? else {
                continue;
            };
            // Windows reports both press and release; act on press only.
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match step(&mut list, action(&key)) {
                Step::Stay => {}
                Step::Done(outcome) => break outcome,
            }
        }
    };

    erase(&mut out, drawn)?;
    queue!(out, cursor::Show)?;
    out.flush()?;

    if let Outcome::Selected(index) = outcome {
        summary(&mut out, title, items[index].label)?;
    }
    Ok(outcome)
}

/// Prints the one line the chosen row leaves behind.
fn summary(out: &mut impl Write, title: &str, label: &str) -> io::Result<()> {
    if use_color() {
        writeln!(out, "\x1b[32m?\x1b[0m {}  \x1b[36m{}\x1b[0m", title, label)?;
    } else {
        writeln!(out, "? {}  {}", title, label)?;
    }
    out.flush()
}

fn use_color() -> bool {
    crate::banner::use_color()
}

/// Raw mode, restored however the picker exits — including on panic.
struct Raw;

impl Raw {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        Ok(Self)
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
    }
}

/// The shape of the screen a frame is drawn for.
///
/// Passed in rather than asked for inside the drawing code, so the layout can
/// be tested without a terminal to ask.
#[derive(Debug, Clone, Copy)]
struct Size {
    cols: usize,
    rows: usize,
}

/// Falls back to a conventional 80x24 when the terminal will not say.
fn size() -> Size {
    let (cols, rows) = terminal::size().unwrap_or((0, 0));
    size_of(cols, rows)
}

/// A terminal that answers with zero is as good as one that does not answer:
/// a pty opened without a window size does exactly that, and taking it at its
/// word truncates every line to nothing and draws an empty box.
fn size_of(cols: u16, rows: u16) -> Size {
    Size {
        cols: if cols == 0 { 80 } else { cols as usize },
        rows: if rows == 0 { 24 } else { rows as usize },
    }
}

/// List state: the rows, what has been typed, where the cursor is.
struct List<'a> {
    items: &'a [Item],
    filter: String,
    /// Index into the *filtered* list.
    cursor: usize,
    /// First visible row, for lists taller than the space available.
    offset: usize,
}

impl<'a> List<'a> {
    fn new(items: &'a [Item]) -> Self {
        Self {
            items,
            filter: String::new(),
            cursor: 0,
            offset: 0,
        }
    }

    /// Indices of the rows matching the filter.
    ///
    /// A case-insensitive substring test over the label and its description,
    /// so both "b64"-style guessing at the name and typing a word from the
    /// description ("decode") narrow the list.
    fn matches(&self) -> Vec<usize> {
        if self.filter.is_empty() {
            return (0..self.items.len()).collect();
        }
        let needle = self.filter.to_lowercase();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.label.to_lowercase().contains(&needle)
                    || item.description.to_lowercase().contains(&needle)
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn selected(&self) -> Option<usize> {
        self.matches().get(self.cursor).copied()
    }

    fn move_down(&mut self) {
        let len = self.matches().len();
        if len > 0 {
            self.cursor = (self.cursor + 1) % len;
        }
    }

    fn move_up(&mut self) {
        let len = self.matches().len();
        if len > 0 {
            self.cursor = (self.cursor + len - 1) % len;
        }
    }

    fn push_filter(&mut self, c: char) {
        self.filter.push(c);
        self.clamp();
    }

    fn pop_filter(&mut self) {
        self.filter.pop();
        self.clamp();
    }

    /// Keeps the cursor inside the filtered list after it shrinks or grows.
    fn clamp(&mut self) {
        let len = self.matches().len();
        if len == 0 {
            self.cursor = 0;
        } else if self.cursor >= len {
            self.cursor = len - 1;
        }
    }

    /// Scrolls so the cursor stays visible in a window of `height` rows.
    fn scroll_into_view(&mut self, height: usize) {
        if height == 0 {
            return;
        }
        if self.cursor < self.offset {
            self.offset = self.cursor;
        } else if self.cursor >= self.offset + height {
            self.offset = self.cursor + 1 - height;
        }
    }
}

/// What the loop does after a key has been applied.
enum Step {
    /// Redraw and wait for the next key.
    Stay,
    Done(Outcome),
}

/// Applies one key to the list.
///
/// Split from the I/O so every branch is reachable from a `KeyEvent` alone,
/// with no terminal to draw on.
fn step(list: &mut List, action: Action) -> Step {
    match action {
        Action::Cancel => Step::Done(Outcome::Cancelled),
        Action::Confirm => match list.selected() {
            Some(index) => Step::Done(Outcome::Selected(index)),
            // Nothing matches what was typed, so there is nothing to confirm.
            None => Step::Stay,
        },
        Action::Down => {
            list.move_down();
            Step::Stay
        }
        Action::Up => {
            list.move_up();
            Step::Stay
        }
        Action::Backspace => {
            list.pop_filter();
            Step::Stay
        }
        Action::Insert(c) => {
            list.push_filter(c);
            Step::Stay
        }
        Action::None => Step::Stay,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Action {
    Confirm,
    Cancel,
    Up,
    Down,
    Backspace,
    Insert(char),
    None,
}

/// Maps a key to an action.
///
/// The emacs pairs (Ctrl-N / Ctrl-P) are here because the prompt below the
/// picker is a readline, and hands already in that habit reach for them. So
/// is Ctrl-D: it leaves the prompt, and a key that leaves one screen should
/// not be dead on the next.
fn action(key: &KeyEvent) -> Action {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Enter => Action::Confirm,
        KeyCode::Esc => Action::Cancel,
        KeyCode::Char('c') | KeyCode::Char('d') | KeyCode::Char('g') if ctrl => Action::Cancel,
        KeyCode::Down | KeyCode::Tab => Action::Down,
        KeyCode::Up | KeyCode::BackTab => Action::Up,
        KeyCode::Char('n') if ctrl => Action::Down,
        KeyCode::Char('p') if ctrl => Action::Up,
        KeyCode::Backspace => Action::Backspace,
        // Terminals told to send 0x08 for the erase key surface it as Ctrl-H.
        KeyCode::Char('h') if ctrl => Action::Backspace,
        KeyCode::Char(c) if !ctrl => Action::Insert(c),
        _ => Action::None,
    }
}

/// Removes the block the last frame left on screen.
fn erase(out: &mut impl Write, drawn: usize) -> io::Result<()> {
    if drawn == 0 {
        return Ok(());
    }
    queue!(out, cursor::MoveToColumn(0))?;
    if drawn > 1 {
        queue!(out, cursor::MoveUp(drawn as u16 - 1))?;
    }
    queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown))?;
    out.flush()
}

const PLACEHOLDER: &str = "type to filter";
const NO_MATCH: &str = "no match";

/// Builds the lines of one frame.
///
/// The cursor row is returned separately from the text so the caller can
/// highlight it; everything else about the frame is decided here, which is
/// what makes the layout testable without a terminal.
fn frame(title: &str, list: &mut List, cancel: &str, size: Size) -> (Vec<String>, Option<usize>) {
    // The header and the hint line, which sandwich the rows.
    let height = size.rows.saturating_sub(2).max(1);
    let matches = list.matches();
    list.scroll_into_view(height);

    let mut lines = Vec::new();
    lines.push(fit(&header(title, &list.filter), size.cols));

    let label_width = matches
        .iter()
        .map(|&i| list.items[i].label.chars().count())
        .max()
        .unwrap_or(0);

    let mut cursor_row = None;
    if matches.is_empty() {
        lines.push(fit(&format!("  {NO_MATCH}"), size.cols));
    } else {
        for (row, &index) in matches.iter().skip(list.offset).take(height).enumerate() {
            let item = &list.items[index];
            let is_cursor = list.offset + row == list.cursor;
            if is_cursor {
                cursor_row = Some(lines.len());
            }
            lines.push(fit(
                &format!(
                    "{} {:<label_width$}  {}",
                    if is_cursor { ">" } else { " " },
                    item.label,
                    item.description
                ),
                size.cols,
            ));
        }
    }

    lines.push(fit(&hints(cancel), size.cols));
    (lines, cursor_row)
}

/// The line above the rows: the question, then what has been typed.
fn header(title: &str, filter: &str) -> String {
    if filter.is_empty() {
        format!("? {title}  ({PLACEHOLDER})")
    } else {
        format!("? {title}  {filter}")
    }
}

/// The line under the rows.
///
/// Plain ASCII on purpose: arrow glyphs land in ranges terminals disagree
/// about, and the ones with an emoji presentation render double width and
/// shift the whole row.
fn hints(cancel: &str) -> String {
    format!("  up/down move   enter select   esc/ctrl-d {cancel}")
}

/// Truncates to `width` so a line can never wrap.
///
/// Wrapping would break the redraw: the block is erased by moving up as many
/// lines as were printed, and a wrapped line takes two.
fn fit(line: &str, width: usize) -> String {
    line.chars().take(width).collect()
}

/// Draws one frame over the last one, returning how many lines it used.
fn draw(
    out: &mut impl Write,
    title: &str,
    list: &mut List,
    cancel: &str,
    size: Size,
    drawn: usize,
) -> io::Result<usize> {
    erase(out, drawn)?;
    let (lines, cursor_row) = frame(title, list, cancel, size);

    for (i, line) in lines.iter().enumerate() {
        // Raw mode means a newline only moves down; the carriage return is
        // ours to send. The last line gets none, so the cursor finishes on it
        // and the next frame knows how far up to go.
        if i > 0 {
            queue!(out, style::Print("\r\n"))?;
        }
        let last = i + 1 == lines.len();
        if Some(i) == cursor_row {
            // Reverse video rather than a colour: it stays readable on any
            // theme. The row is padded to the full width so the highlight
            // runs edge to edge.
            queue!(
                out,
                style::SetAttribute(style::Attribute::Reverse),
                style::Print(pad(line, size.cols)),
                style::SetAttribute(style::Attribute::Reset),
            )?;
        } else if i == 0 || last {
            queue!(
                out,
                style::SetAttribute(style::Attribute::Dim),
                style::Print(line),
                style::SetAttribute(style::Attribute::Reset),
            )?;
        } else {
            queue!(out, style::Print(line))?;
        }
    }
    out.flush()?;
    Ok(lines.len())
}

/// Pads to `width`, one column short of it: a highlight that reaches the last
/// column makes some terminals wrap to the next line.
fn pad(line: &str, width: usize) -> String {
    let width = width.saturating_sub(1);
    let mut out: String = line.chars().take(width).collect();
    let len = out.chars().count();
    if len < width {
        out.extend(std::iter::repeat_n(' ', width - len));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEMS: &[Item] = &[
        Item {
            label: "timestamp",
            description: "Unix timestamp <-> datetime",
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
            description: "JWT decode",
        },
    ];

    fn list() -> List<'static> {
        List::new(ITEMS)
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn size() -> Size {
        Size { cols: 80, rows: 24 }
    }

    #[test]
    fn filter_matches_label_and_description_case_insensitively() {
        let mut l = list();
        l.filter = "BASE".to_string();
        assert_eq!(l.matches(), vec![1]);

        // "encode" appears only in descriptions.
        l.filter = "encode".to_string();
        assert_eq!(l.matches(), vec![1, 2]);
    }

    #[test]
    fn cursor_wraps_around() {
        let mut l = list();
        l.move_up();
        assert_eq!(l.selected(), Some(3));
        l.move_down();
        assert_eq!(l.selected(), Some(0));
    }

    #[test]
    fn cursor_stays_inside_a_shrinking_list() {
        let mut l = list();
        l.cursor = 3;
        for c in "base".chars() {
            l.push_filter(c);
        }
        assert_eq!(l.cursor, 0);
        assert_eq!(l.selected(), Some(1));
    }

    #[test]
    fn erasing_the_filter_brings_the_rows_back() {
        let mut l = list();
        l.push_filter('j');
        assert_eq!(l.matches().len(), 1);
        l.pop_filter();
        assert_eq!(l.matches().len(), ITEMS.len());
    }

    #[test]
    fn nothing_matching_leaves_nothing_to_confirm() {
        let mut l = list();
        for c in "zzz".chars() {
            l.push_filter(c);
        }
        assert!(l.selected().is_none());
        assert!(matches!(step(&mut l, Action::Confirm), Step::Stay));
    }

    #[test]
    fn enter_selects_the_row_under_the_cursor() {
        let mut l = list();
        l.move_down();
        match step(&mut l, Action::Confirm) {
            Step::Done(Outcome::Selected(index)) => assert_eq!(index, 1),
            _ => panic!("expected a selection"),
        }
    }

    #[test]
    fn escape_and_ctrl_c_back_out() {
        // Ctrl-D is here too: it is what leaves the prompt these lists sit
        // above, so it backs out of the list rather than doing nothing.
        for k in [key(KeyCode::Esc), ctrl('c'), ctrl('d'), ctrl('g')] {
            assert_eq!(action(&k), Action::Cancel);
        }
        let mut l = list();
        assert!(matches!(
            step(&mut l, Action::Cancel),
            Step::Done(Outcome::Cancelled)
        ));
    }

    #[test]
    fn arrows_tab_and_emacs_keys_all_move() {
        assert_eq!(action(&key(KeyCode::Down)), Action::Down);
        assert_eq!(action(&key(KeyCode::Tab)), Action::Down);
        assert_eq!(action(&ctrl('n')), Action::Down);
        assert_eq!(action(&key(KeyCode::Up)), Action::Up);
        assert_eq!(action(&key(KeyCode::BackTab)), Action::Up);
        assert_eq!(action(&ctrl('p')), Action::Up);
    }

    #[test]
    fn printable_keys_type_into_the_filter_and_ctrl_ones_do_not() {
        assert_eq!(action(&key(KeyCode::Char('b'))), Action::Insert('b'));
        assert_eq!(action(&ctrl('a')), Action::None);
        // A terminal sending 0x08 for the erase key surfaces it as Ctrl-H.
        assert_eq!(action(&ctrl('h')), Action::Backspace);
        assert_eq!(action(&key(KeyCode::Backspace)), Action::Backspace);
    }

    #[test]
    fn a_frame_is_the_header_the_rows_and_the_hints() {
        let mut l = list();
        let (lines, cursor_row) = frame("Select a tool", &mut l, "quit", size());
        assert_eq!(lines.len(), ITEMS.len() + 2);
        assert!(lines[0].contains("Select a tool"));
        assert!(lines[0].contains(PLACEHOLDER));
        assert_eq!(cursor_row, Some(1));
        assert!(lines[1].starts_with("> timestamp"));
        assert!(lines[2].starts_with("  base64"));
        // The hint line names both keys that back out, and says what backing
        // out means on this particular list.
        assert!(lines.last().unwrap().contains("esc/ctrl-d quit"));
    }

    #[test]
    fn the_header_shows_what_has_been_typed_instead_of_the_placeholder() {
        let mut l = list();
        l.push_filter('u');
        let (lines, _) = frame("Select a tool", &mut l, "quit", size());
        assert!(lines[0].ends_with("  u"));
        assert!(!lines[0].contains(PLACEHOLDER));
    }

    #[test]
    fn filtering_everything_away_says_so_instead_of_going_blank() {
        let mut l = list();
        for c in "zzz".chars() {
            l.push_filter(c);
        }
        let (lines, cursor_row) = frame("Select a tool", &mut l, "quit", size());
        assert_eq!(lines.len(), 3);
        assert!(lines[1].contains(NO_MATCH));
        assert_eq!(cursor_row, None);
    }

    #[test]
    fn a_short_terminal_scrolls_rather_than_drawing_more_rows_than_it_has() {
        let mut l = list();
        let short = Size { cols: 80, rows: 4 };
        // Two rows fit between the header and the hints.
        let (lines, _) = frame("Select a tool", &mut l, "quit", short);
        assert_eq!(lines.len(), 4);
        assert!(lines[1].starts_with("> timestamp"));

        // Moving past the window scrolls it instead of growing the frame.
        l.move_up();
        let (lines, cursor_row) = frame("Select a tool", &mut l, "quit", short);
        assert_eq!(lines.len(), 4);
        assert_eq!(cursor_row, Some(2));
        assert!(lines[2].starts_with("> jwt"));
    }

    #[test]
    fn a_terminal_that_reports_nothing_gets_the_conventional_size() {
        let guessed = size_of(0, 0);
        assert_eq!((guessed.cols, guessed.rows), (80, 24));
        // Anything it does report is taken as given.
        let stated = size_of(120, 40);
        assert_eq!((stated.cols, stated.rows), (120, 40));
    }

    #[test]
    fn no_line_can_wrap_the_terminal() {
        let mut l = list();
        let narrow = Size { cols: 12, rows: 24 };
        let (lines, _) = frame("Select a tool", &mut l, "quit", narrow);
        for line in &lines {
            assert!(line.chars().count() <= narrow.cols, "{line}");
        }
        // The highlight stops one column short of the edge.
        assert_eq!(pad("x", narrow.cols).chars().count(), narrow.cols - 1);
    }
}
