//! Startup animation for the REPL: a hammer swung down onto the anvil,
//! throwing sparks.
//!
//! The hammer is held on the right, so it comes down in an arc from the upper
//! right: head up with the handle hanging down, then diagonal, then flat on
//! the anvil with the handle pointing right.
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

const RESET: &str = "\x1b[0m";

// ─── sprites ───────────────────────────────────────────────────────────────
// Spaces are transparent: they never overwrite what is already on the canvas.

/// One hammer position: the steel head and the wooden handle, each with the
/// canvas (row, column) its top-left corner sits at. The head's border carries
/// the junction glyph the handle grows out of.
struct Pose {
    head: &'static [&'static str],
    head_at: (usize, usize),
    handle: &'static [&'static str],
    handle_at: (usize, usize),
}

/// Raised: head up and to the right, handle hanging down to the grip.
const RAISED: Pose = Pose {
    head: &["┏━━━━━━━┓", "┃███████┃", "┗━━━┳━━━┛"],
    head_at: (0, 19),
    handle: &["┃", "┃", "┃"],
    handle_at: (3, 23),
};

/// Halfway through the swing, handle at roughly 45°.
const SWING: Pose = Pose {
    head: &["┏━━━━━━━┓", "┃███████┃", "┗━━━━━━━┛"],
    head_at: (2, 16),
    handle: &["╲", " ╲", "  ╲"],
    handle_at: (5, 25),
};

/// Just short of the anvil, handle flattening out.
const FALL: Pose = Pose {
    head: &["┏━━━━━━━┓", "┃███████┣", "┗━━━━━━━┛"],
    head_at: (3, 14),
    handle: &["━━╲", "   ━━"],
    handle_at: (4, 23),
};

/// On the anvil: head flat, handle straight out to the right.
const STRIKE: Pose = Pose {
    head: &["┏━━━━━━━┓", "┃███████┣", "┗━━━━━━━┛"],
    head_at: (5, 13),
    handle: &["━━━━━━"],
    handle_at: (6, 22),
};

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

    /// Blit a hammer pose: wooden handle first, steel head over it.
    fn draw_pose(&mut self, pose: &Pose) {
        let (top, left) = pose.handle_at;
        self.draw(pose.handle, top, left, Paint::Handle);
        let (top, left) = pose.head_at;
        self.draw(pose.head, top, left, Paint::Steel);
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

// ─── storyboard ────────────────────────────────────────────────────────────

/// A spark: canvas row, offset from [`CENTER`] (negative flies left), glyph.
struct Spark(usize, i32, char, Paint);

/// The moment of impact: bright, tight, close to the struck face.
const BURST: [Spark; 8] = [
    Spark(7, -5, '✦', Paint::SparkHot),
    Spark(7, 5, '✦', Paint::SparkHot),
    Spark(6, -7, '✧', Paint::SparkHot),
    Spark(5, 7, '✧', Paint::SparkHot),
    Spark(5, -9, '·', Paint::SparkWarm),
    Spark(4, 9, '·', Paint::SparkWarm),
    Spark(ANVIL_TOP, -8, '·', Paint::SparkWarm),
    Spark(ANVIL_TOP, 8, '·', Paint::SparkWarm),
];

/// A beat later: flying outward and starting to cool.
const SPREAD: [Spark; 10] = [
    Spark(4, -8, '✧', Paint::SparkHot),
    Spark(3, 8, '✧', Paint::SparkHot),
    Spark(5, -12, '·', Paint::SparkWarm),
    Spark(4, 12, '·', Paint::SparkWarm),
    Spark(6, -10, '✦', Paint::SparkWarm),
    Spark(2, 10, '·', Paint::SparkWarm),
    Spark(7, -13, '˙', Paint::SparkFade),
    Spark(7, 13, '˙', Paint::SparkFade),
    Spark(ANVIL_TOP, -11, '·', Paint::SparkFade),
    Spark(ANVIL_TOP, 11, '·', Paint::SparkFade),
];

/// Drifting away as the hammer lifts.
const DRIFTING: [Spark; 6] = [
    Spark(1, -9, '·', Paint::SparkWarm),
    Spark(1, 9, '·', Paint::SparkWarm),
    Spark(2, -13, '˙', Paint::SparkFade),
    Spark(3, 13, '˙', Paint::SparkFade),
    Spark(5, -15, '˙', Paint::SparkFade),
    Spark(6, 15, '˙', Paint::SparkFade),
];

/// The last two embers before everything goes cold.
const EMBERS: [Spark; 2] = [
    Spark(0, -12, '˙', Paint::SparkFade),
    Spark(1, 13, '˙', Paint::SparkFade),
];

struct Frame {
    delay_ms: u64,
    pose: &'static Pose,
    glow: Option<Paint>,
    sparks: &'static [Spark],
}

/// The whole animation, about 900 ms. The last frame has no delay: it is what
/// stays on screen.
const STORYBOARD: [Frame; 8] = [
    // Hammer up, waiting.
    Frame {
        delay_ms: 260,
        pose: &RAISED,
        glow: None,
        sparks: &[],
    },
    // Coming down, picking up speed.
    Frame {
        delay_ms: 70,
        pose: &SWING,
        glow: None,
        sparks: &[],
    },
    Frame {
        delay_ms: 45,
        pose: &FALL,
        glow: None,
        sparks: &[],
    },
    // Impact.
    Frame {
        delay_ms: 120,
        pose: &STRIKE,
        glow: Some(Paint::GlowHot),
        sparks: &BURST,
    },
    Frame {
        delay_ms: 120,
        pose: &STRIKE,
        glow: Some(Paint::GlowWarm),
        sparks: &SPREAD,
    },
    // Bouncing back up, sparks flying outward and cooling.
    Frame {
        delay_ms: 130,
        pose: &SWING,
        glow: Some(Paint::GlowWarm),
        sparks: &DRIFTING,
    },
    Frame {
        delay_ms: 150,
        pose: &RAISED,
        glow: Some(Paint::GlowDim),
        sparks: &EMBERS,
    },
    // Settled.
    Frame {
        delay_ms: 0,
        pose: &RAISED,
        glow: None,
        sparks: &[],
    },
];

fn scene(frame: &Frame) -> Canvas {
    let mut canvas = Canvas::new();

    canvas.draw(
        &ANVIL,
        ANVIL_TOP,
        centered(ANVIL[0].chars().count()),
        Paint::Anvil,
    );
    if let Some(paint) = frame.glow {
        canvas.recolor(ANVIL_TOP, CENTER - 5..=CENTER + 5, paint);
    }

    // Sparks sit in front of the anvil but behind the hammer, so the hammer
    // hides any that fly into it.
    for Spark(row, dx, ch, paint) in frame.sparks {
        let col = CENTER as i32 + dx;
        if col >= 0 {
            canvas.put(*row, col as usize, *ch, *paint);
        }
    }

    canvas.draw_pose(frame.pose);
    canvas
}

/// The still frame shown when the animation is skipped, and the state the
/// animation settles back into: hammer raised over a cold anvil.
fn resting_scene() -> Canvas {
    scene(&STORYBOARD[STORYBOARD.len() - 1])
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

    for (i, frame) in STORYBOARD.iter().enumerate() {
        if i > 0 {
            // \x1b[{n}F: move the cursor to the start of the line n rows up.
            let _ = write!(stdout, "\x1b[{}F", HEIGHT);
        }
        for line in scene(frame).render(color) {
            // \x1b[K: clear to end of line, so a shorter frame cannot leave
            // remnants of the previous one behind.
            let _ = writeln!(stdout, "{}\x1b[K", line);
        }
        let _ = stdout.flush();
        if frame.delay_ms > 0 {
            sleep(Duration::from_millis(frame.delay_ms));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POSES: [&Pose; 4] = [&RAISED, &SWING, &FALL, &STRIKE];
    const SPARK_GLYPHS: [char; 4] = ['✦', '✧', '·', '˙'];

    fn scenes() -> Vec<Canvas> {
        STORYBOARD.iter().map(scene).collect()
    }

    /// Rows of a rendered frame that contain any of `glyphs`.
    fn rows_with(lines: &[String], glyphs: &[char]) -> Vec<usize> {
        lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(glyphs))
            .map(|(row, _)| row)
            .collect()
    }

    #[test]
    fn every_frame_fills_the_canvas_height() {
        for canvas in scenes() {
            assert_eq!(canvas.render(false).len(), HEIGHT);
        }
    }

    #[test]
    fn no_frame_exceeds_the_canvas_width() {
        for canvas in scenes() {
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
        for canvas in scenes() {
            for line in canvas.render(false) {
                assert!(!line.contains('\x1b'), "unexpected escape in {:?}", line);
            }
        }
    }

    #[test]
    fn colored_render_resets_every_line_it_paints() {
        for canvas in scenes() {
            for line in canvas.render(true) {
                if line.contains('\x1b') {
                    assert!(line.ends_with(RESET), "line not reset: {:?}", line);
                }
            }
        }
    }

    #[test]
    fn animation_starts_and_ends_at_rest() {
        let scenes = scenes();
        let resting = resting_scene().render(false);
        assert_eq!(scenes.first().unwrap().render(false), resting);
        assert_eq!(scenes.last().unwrap().render(false), resting);
        assert_eq!(
            STORYBOARD.last().unwrap().delay_ms,
            0,
            "last frame must not sleep"
        );
    }

    /// Canvas columns the handle covers.
    fn handle_columns(pose: &Pose) -> Vec<usize> {
        let (_, left) = pose.handle_at;
        pose.handle
            .iter()
            .flat_map(|line| {
                line.chars()
                    .enumerate()
                    .filter(|(_, ch)| *ch != ' ')
                    .map(move |(dx, _)| left + dx)
            })
            .collect()
    }

    fn head_center(pose: &Pose) -> usize {
        pose.head_at.1 + pose.head[0].chars().count() / 2
    }

    #[test]
    fn the_swing_comes_down_from_the_upper_right() {
        // Head position per pose, in swing order.
        let head_top: Vec<usize> = POSES.iter().map(|p| p.head_at.0).collect();
        let centers: Vec<usize> = POSES.iter().map(|p| head_center(p)).collect();

        assert!(
            head_top.windows(2).all(|w| w[0] < w[1]),
            "the head only ever moves downward: {:?}",
            head_top
        );
        assert!(
            centers.windows(2).all(|w| w[0] > w[1]),
            "the head travels leftward as it falls: {:?}",
            centers
        );
        assert_eq!(
            *centers.last().unwrap(),
            CENTER,
            "the blow lands on the middle of the anvil"
        );
        // Three rows of head, resting directly on the anvil's top surface.
        assert_eq!(STRIKE.head_at.0 + STRIKE.head.len() - 1, ANVIL_TOP - 1);
    }

    #[test]
    fn the_handle_is_always_held_to_the_right_of_the_head() {
        for pose in POSES {
            let columns = handle_columns(pose);
            assert!(!columns.is_empty(), "every pose shows a handle");
            assert!(
                columns.iter().all(|&col| col >= head_center(pose)),
                "handle drifted left of the head: {:?}",
                columns
            );
        }
    }

    #[test]
    fn the_handle_is_wood_and_the_head_is_steel() {
        for pose in POSES {
            let canvas = scene(&Frame {
                delay_ms: 0,
                pose,
                glow: None,
                sparks: &[],
            });
            let cells = canvas.cells.iter().flatten();
            let (wood, steel): (Vec<_>, Vec<_>) =
                cells.partition(|(_, paint)| *paint == Paint::Handle);

            assert_eq!(
                wood.len(),
                handle_columns(pose).len(),
                "the whole handle is wood, and nothing else is"
            );
            assert!(
                steel.iter().any(|(_, paint)| *paint == Paint::Steel),
                "the head is steel"
            );
        }
    }

    #[test]
    fn sparks_only_fly_once_the_hammer_lands() {
        let sparky: Vec<bool> = scenes()
            .iter()
            .map(|canvas| !rows_with(&canvas.render(false), &SPARK_GLYPHS).is_empty())
            .collect();

        assert!(!sparky[..3].iter().any(|&s| s), "no sparks before impact");
        assert!(sparky[3], "impact frame throws sparks");
        assert!(!sparky.last().unwrap(), "sparks are gone once settled");
    }

    #[test]
    fn no_spark_is_swallowed_by_the_hammer() {
        for (frame, canvas) in STORYBOARD.iter().zip(scenes()) {
            let drawn: usize = canvas
                .render(false)
                .iter()
                .map(|line| line.chars().filter(|ch| SPARK_GLYPHS.contains(ch)).count())
                .sum();
            assert_eq!(
                drawn,
                frame.sparks.len(),
                "a spark is hidden behind the hammer"
            );
        }
    }

    #[test]
    fn the_anvil_stays_put_for_the_whole_animation() {
        let anvil_base = ANVIL[ANVIL.len() - 1].trim();
        for canvas in scenes() {
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
        for (i, canvas) in scenes().iter().enumerate() {
            println!("--- frame {} ---", i);
            for line in canvas.render(false) {
                println!("|{}|", line);
            }
        }
    }
}
