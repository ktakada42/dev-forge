//! Startup animation for the REPL: a hammer striking the anvil, throwing sparks.
//!
//! Every frame is composed on a fixed `WIDTH` x `HEIGHT` character canvas, so
//! redrawing is just "move the cursor `HEIGHT` lines up and print again".
//!
//! The animation is skipped (a single still frame is printed instead) when
//! stdout is not a terminal, when `TERM=dumb`, or when `FORGE_NO_ANIMATION` is
//! set. Colors are additionally suppressed when `NO_COLOR` is set.

use std::io::{self, IsTerminal, Write};
use std::thread::sleep;
use std::time::Duration;

const WIDTH: usize = 34;
const HEIGHT: usize = 13;

/// Column the hammer falls on — also the center of the anvil.
const CENTER: usize = 17;
/// Canvas row of the anvil's top surface (where sparks are born).
const ANVIL_TOP: usize = 8;
/// Top row of the hammer sprite when raised / at the moment of impact.
const HAMMER_RAISED: usize = 0;
const HAMMER_IMPACT: usize = 3;

const RESET: &str = "\x1b[0m";

// ─── sprites ───────────────────────────────────────────────────────────────
// Spaces are transparent: they never overwrite what is already on the canvas.

/// Handle, drawn above the head. Column 4 lines up with the head's `┻`.
const HAMMER_HANDLE: [&str; 2] = [
    "    ┃    ", //
    "    ┃    ",
];

const HAMMER_HEAD: [&str; 3] = [
    "┏━━━┻━━━┓", //
    "┃███████┃",
    "┗━━━━━━━┛",
];

const ANVIL: [&str; 5] = [
    "     ▄▄▄▄▄▄▄▄▄▄▄     ",
    "  █████████████████  ",
    "  ▀▀▀▀▀▀█████▀▀▀▀▀▀  ",
    "        █████        ",
    "    █████████████    ",
];

/// Horizontal offset that centers a sprite of `width` on [`CENTER`].
const fn centered(width: usize) -> usize {
    CENTER - width / 2
}

// ─── colors ────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Paint {
    Steel,
    Handle,
    Anvil,
    GlowHot,
    GlowWarm,
    GlowDim,
    SparkHot,
    SparkWarm,
    SparkFade,
}

impl Paint {
    fn ansi(self) -> &'static str {
        match self {
            Paint::Steel => "\x1b[38;5;252m",
            Paint::Handle => "\x1b[38;5;130m",
            Paint::Anvil => "\x1b[38;5;244m",
            Paint::GlowHot => "\x1b[1;38;5;226m",
            Paint::GlowWarm => "\x1b[38;5;208m",
            Paint::GlowDim => "\x1b[38;5;130m",
            Paint::SparkHot => "\x1b[1;38;5;227m",
            Paint::SparkWarm => "\x1b[38;5;214m",
            Paint::SparkFade => "\x1b[38;5;130m",
        }
    }
}

// ─── canvas ────────────────────────────────────────────────────────────────

/// A fixed-size grid of painted characters. `None` is empty space.
struct Canvas {
    cells: Vec<Option<(char, Paint)>>,
}

impl Canvas {
    fn new() -> Self {
        Canvas {
            cells: vec![None; WIDTH * HEIGHT],
        }
    }

    fn put(&mut self, row: usize, col: usize, ch: char, paint: Paint) {
        if row < HEIGHT && col < WIDTH {
            self.cells[row * WIDTH + col] = Some((ch, paint));
        }
    }

    /// Blit a sprite with its top-left corner at (`top`, `left`), skipping
    /// spaces and clipping anything that falls outside the canvas.
    fn draw(&mut self, sprite: &[&str], top: usize, left: usize, paint: Paint) {
        for (dy, line) in sprite.iter().enumerate() {
            for (dx, ch) in line.chars().enumerate() {
                if ch != ' ' {
                    self.put(top + dy, left + dx, ch, paint);
                }
            }
        }
    }

    /// Recolor existing cells without changing them — used for the hot spot
    /// the hammer leaves on the anvil.
    fn recolor(&mut self, row: usize, cols: std::ops::RangeInclusive<usize>, paint: Paint) {
        for col in cols {
            if row < HEIGHT && col < WIDTH {
                if let Some(cell) = &mut self.cells[row * WIDTH + col] {
                    cell.1 = paint;
                }
            }
        }
    }

    /// Render to one string per row, trailing blanks trimmed.
    fn render(&self, color: bool) -> Vec<String> {
        self.cells
            .chunks(WIDTH)
            .map(|row| {
                let Some(last) = row.iter().rposition(|cell| cell.is_some()) else {
                    return String::new();
                };
                let mut out = String::new();
                let mut current: Option<Paint> = None;
                for cell in &row[..=last] {
                    match cell {
                        Some((ch, paint)) => {
                            if color && current != Some(*paint) {
                                out.push_str(paint.ansi());
                                current = Some(*paint);
                            }
                            out.push(*ch);
                        }
                        None => {
                            if color && current.is_some() {
                                out.push_str(RESET);
                                current = None;
                            }
                            out.push(' ');
                        }
                    }
                }
                if color && current.is_some() {
                    out.push_str(RESET);
                }
                out
            })
            .collect()
    }
}

// ─── frames ────────────────────────────────────────────────────────────────

/// A spark: canvas row, offset from [`CENTER`] (negative flies left), glyph.
struct Spark(usize, i32, char, Paint);

fn scene(hammer_top: usize, glow: Option<Paint>, sparks: &[Spark]) -> Canvas {
    let mut canvas = Canvas::new();

    canvas.draw(
        &ANVIL,
        ANVIL_TOP,
        centered(ANVIL[0].chars().count()),
        Paint::Anvil,
    );
    if let Some(paint) = glow {
        canvas.recolor(ANVIL_TOP, CENTER - 5..=CENTER + 5, paint);
    }

    let head_left = centered(HAMMER_HEAD[0].chars().count());
    canvas.draw(&HAMMER_HANDLE, hammer_top, head_left, Paint::Handle);
    canvas.draw(
        &HAMMER_HEAD,
        hammer_top + HAMMER_HANDLE.len(),
        head_left,
        Paint::Steel,
    );

    // Sparks are drawn last so they sit in front of the steel.
    for Spark(row, dx, ch, paint) in sparks {
        let col = CENTER as i32 + dx;
        if col >= 0 {
            canvas.put(*row, col as usize, *ch, *paint);
        }
    }

    canvas
}

/// The still frame shown when the animation is skipped, and the state the
/// animation settles back into: hammer raised over a cold anvil.
fn resting_scene() -> Canvas {
    scene(HAMMER_RAISED, None, &[])
}

/// (delay after the frame, frame). The last frame has no delay.
fn frames() -> Vec<(Duration, Canvas)> {
    use Paint::{SparkFade, SparkHot, SparkWarm};

    let burst = [
        Spark(ANVIL_TOP - 1, -5, '✦', SparkHot),
        Spark(ANVIL_TOP - 1, 5, '✦', SparkHot),
        Spark(ANVIL_TOP - 2, -7, '✧', SparkHot),
        Spark(ANVIL_TOP - 2, 7, '✧', SparkHot),
        Spark(ANVIL_TOP - 3, -9, '·', SparkWarm),
        Spark(ANVIL_TOP - 3, 9, '·', SparkWarm),
        Spark(ANVIL_TOP, -8, '·', SparkWarm),
        Spark(ANVIL_TOP, 8, '·', SparkWarm),
    ];

    let spread = [
        Spark(ANVIL_TOP - 4, -8, '✧', SparkHot),
        Spark(ANVIL_TOP - 4, 8, '✧', SparkHot),
        Spark(ANVIL_TOP - 3, -12, '·', SparkWarm),
        Spark(ANVIL_TOP - 3, 12, '·', SparkWarm),
        Spark(ANVIL_TOP - 2, -10, '✦', SparkWarm),
        Spark(ANVIL_TOP - 2, 10, '·', SparkWarm),
        Spark(ANVIL_TOP - 1, -13, '˙', SparkFade),
        Spark(ANVIL_TOP - 1, 13, '˙', SparkFade),
        Spark(ANVIL_TOP, -11, '·', SparkFade),
        Spark(ANVIL_TOP, 11, '·', SparkFade),
    ];

    let drifting = [
        Spark(ANVIL_TOP - 5, -9, '·', SparkWarm),
        Spark(ANVIL_TOP - 5, 9, '·', SparkWarm),
        Spark(ANVIL_TOP - 4, -13, '˙', SparkFade),
        Spark(ANVIL_TOP - 4, 13, '˙', SparkFade),
        Spark(ANVIL_TOP - 2, -15, '˙', SparkFade),
        Spark(ANVIL_TOP - 2, 15, '˙', SparkFade),
    ];

    let embers = [
        Spark(ANVIL_TOP - 6, -11, '˙', SparkFade),
        Spark(ANVIL_TOP - 6, 11, '˙', SparkFade),
    ];

    vec![
        // Hammer up, waiting.
        (Duration::from_millis(260), scene(HAMMER_RAISED, None, &[])),
        // Coming down, picking up speed.
        (
            Duration::from_millis(70),
            scene(HAMMER_RAISED + 1, None, &[]),
        ),
        (
            Duration::from_millis(45),
            scene(HAMMER_RAISED + 2, None, &[]),
        ),
        // Impact.
        (
            Duration::from_millis(120),
            scene(HAMMER_IMPACT, Some(Paint::GlowHot), &burst),
        ),
        (
            Duration::from_millis(120),
            scene(HAMMER_IMPACT, Some(Paint::GlowWarm), &spread),
        ),
        // Lifting off, sparks flying outward and cooling.
        (
            Duration::from_millis(130),
            scene(HAMMER_IMPACT - 1, Some(Paint::GlowWarm), &drifting),
        ),
        (
            Duration::from_millis(150),
            scene(HAMMER_RAISED, Some(Paint::GlowDim), &embers),
        ),
        // Settled.
        (Duration::ZERO, resting_scene()),
    ]
}

// ─── output ────────────────────────────────────────────────────────────────

fn env_set(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|v| !v.is_empty())
}

fn dumb_terminal() -> bool {
    std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false)
}

fn use_color() -> bool {
    io::stdout().is_terminal() && !env_set("NO_COLOR") && !dumb_terminal()
}

fn use_animation() -> bool {
    io::stdout().is_terminal() && !env_set("FORGE_NO_ANIMATION") && !dumb_terminal()
}

/// Play the forge animation, or print a single still frame when animating
/// would be inappropriate (piped output, `TERM=dumb`, `FORGE_NO_ANIMATION`).
pub fn animate() {
    let color = use_color();
    let mut stdout = io::stdout();

    if !use_animation() {
        for line in resting_scene().render(color) {
            let _ = writeln!(stdout, "{}", line);
        }
        let _ = stdout.flush();
        return;
    }

    for (i, (delay, canvas)) in frames().iter().enumerate() {
        if i > 0 {
            // \x1b[{n}F: move the cursor to the start of the line n rows up.
            let _ = write!(stdout, "\x1b[{}F", HEIGHT);
        }
        for line in canvas.render(color) {
            // \x1b[K: clear to end of line, so a shorter frame cannot leave
            // remnants of the previous one behind.
            let _ = writeln!(stdout, "{}\x1b[K", line);
        }
        let _ = stdout.flush();
        if !delay.is_zero() {
            sleep(*delay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPARK_GLYPHS: [char; 4] = ['✦', '✧', '·', '˙'];

    #[test]
    fn every_frame_fills_the_canvas_height() {
        for (_, canvas) in frames() {
            assert_eq!(canvas.render(false).len(), HEIGHT);
        }
    }

    #[test]
    fn no_frame_exceeds_the_canvas_width() {
        for (_, canvas) in frames() {
            for line in canvas.render(false) {
                assert!(
                    line.chars().count() <= WIDTH,
                    "line wider than canvas: {:?}",
                    line
                );
            }
        }
    }

    #[test]
    fn uncolored_render_has_no_escape_sequences() {
        for (_, canvas) in frames() {
            for line in canvas.render(false) {
                assert!(!line.contains('\x1b'), "unexpected escape in {:?}", line);
            }
        }
    }

    #[test]
    fn colored_render_resets_every_line_it_paints() {
        for (_, canvas) in frames() {
            for line in canvas.render(true) {
                if line.contains('\x1b') {
                    assert!(line.ends_with(RESET), "line not reset: {:?}", line);
                }
            }
        }
    }

    #[test]
    fn animation_starts_and_ends_at_rest() {
        let frames = frames();
        let resting = resting_scene().render(false);
        assert_eq!(frames.first().unwrap().1.render(false), resting);
        assert_eq!(frames.last().unwrap().1.render(false), resting);
        assert!(
            frames.last().unwrap().0.is_zero(),
            "last frame must not sleep"
        );
    }

    #[test]
    fn hammer_travels_down_to_the_anvil_and_back_up() {
        let hammer_row = |canvas: &Canvas| {
            canvas
                .render(false)
                .iter()
                .position(|line| line.contains('┃'))
                .expect("hammer is visible in every frame")
        };
        let rows: Vec<usize> = frames().iter().map(|(_, c)| hammer_row(c)).collect();

        assert_eq!(rows.first(), Some(&HAMMER_RAISED));
        assert_eq!(rows.iter().max(), Some(&HAMMER_IMPACT));
        assert_eq!(rows.last(), Some(&HAMMER_RAISED));

        // At impact the head rests directly on the anvil's top surface.
        let head_bottom = HAMMER_IMPACT + HAMMER_HANDLE.len() + HAMMER_HEAD.len() - 1;
        assert_eq!(head_bottom, ANVIL_TOP - 1);
    }

    #[test]
    fn sparks_only_appear_after_the_hammer_lands() {
        let has_sparks = |canvas: &Canvas| {
            canvas
                .render(false)
                .iter()
                .any(|line| line.contains(SPARK_GLYPHS))
        };
        let sparky: Vec<bool> = frames().iter().map(|(_, c)| has_sparks(c)).collect();

        assert!(!sparky[..3].iter().any(|&s| s), "no sparks before impact");
        assert!(sparky[3], "impact frame throws sparks");
        assert!(!sparky.last().unwrap(), "sparks are gone once settled");
    }

    #[test]
    fn sparks_fly_clear_of_the_hammer_head() {
        let head_left = centered(HAMMER_HEAD[0].chars().count());
        let head_right = head_left + HAMMER_HEAD[0].chars().count() - 1;
        for (_, canvas) in frames() {
            for (row, line) in canvas.render(false).iter().enumerate() {
                for (col, ch) in line.chars().enumerate() {
                    if SPARK_GLYPHS.contains(&ch) {
                        assert!(
                            col < head_left || col > head_right || row >= ANVIL_TOP,
                            "spark at ({}, {}) is hidden behind the hammer",
                            row,
                            col
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn the_anvil_stays_put_for_the_whole_animation() {
        let anvil_base = ANVIL[ANVIL.len() - 1].trim();
        for (_, canvas) in frames() {
            let lines = canvas.render(false);
            assert!(
                lines[ANVIL_TOP + ANVIL.len() - 1].contains(anvil_base),
                "anvil base moved"
            );
        }
    }

    #[test]
    fn drawing_outside_the_canvas_is_clipped() {
        let mut canvas = Canvas::new();
        canvas.draw(&["███"], HEIGHT + 5, WIDTH + 5, Paint::Steel);
        canvas.put(HEIGHT, 0, '█', Paint::Steel);
        canvas.put(0, WIDTH, '█', Paint::Steel);
        assert!(canvas.render(false).iter().all(|line| line.is_empty()));
    }

    /// Not an assertion — run with `--nocapture` to eyeball the frames.
    #[test]
    fn print_frames() {
        for (i, (_, canvas)) in frames().iter().enumerate() {
            println!("--- frame {} ---", i);
            for line in canvas.render(false) {
                println!("|{}|", line);
            }
        }
    }
}
