//! Startup animation for the REPL: a hammer swung down onto the anvil,
//! throwing sparks. The hammer then disappears, leaving the anvil — stamped
//! with "Dev Forge" — on screen as the prompt comes up. The app name,
//! version, and tagline sit to the anvil's right for the whole animation;
//! the hammer's swing never reaches that column, so nothing is ever drawn
//! over it.
//!
//! The hammer is a rigid body turning counter-clockwise through 90° about
//! [`GRIP`], the fixed point where the hand holds the end of the handle: head
//! straight up, then halfway round, then down on the anvil. The head turns
//! with it, so it is broad while the handle stands vertical and deep once the
//! handle lies flat — one hammer, three views of it, never a different size.
//!
//! A terminal cell is about twice as tall as it is wide, which is why the
//! poses look squashed on paper but read as one shape on screen: a 45° swing
//! covers twice as many columns as rows.
//!
//! Every frame is composed on a fixed `WIDTH` x `HEIGHT` character canvas, so
//! redrawing is normally just "move the cursor up by the last frame's height
//! and print again". Once the hammer is gone, the empty rows it used to swing
//! through serve no purpose, so the last few frames crop more of them off the
//! top each time ([`Frame::crop_top`]) — the void above the anvil visibly
//! closes instead of being cut off in one jump or left sitting there forever.
//!
//! The animation is skipped (a single still frame is printed instead) when
//! stdout is not a terminal, when `TERM=dumb`, or when `FORGE_NO_ANIMATION` is
//! set. Colors are additionally suppressed when `NO_COLOR` is set.

use std::io::{self, IsTerminal, Write};
use std::thread::sleep;
use std::time::Duration;

const WIDTH: usize = 60;
const HEIGHT: usize = 13;

/// Column the hammer falls on — also the center of the anvil.
const CENTER: usize = 17;
/// Canvas row of the anvil's top surface (where sparks are born).
const ANVIL_TOP: usize = 8;

const RESET: &str = "\x1b[0m";

// ─── side text ─────────────────────────────────────────────────────────────
// The app name, version, and tagline, set beside the anvil. Column chosen
// with a couple of columns of clearance past the hammer's widest reach (see
// `the_side_text_clears_the_hammers_reach`), so it can be drawn in every
// frame without ever worrying about what pose the hammer is in.

const TEXT_LEFT: usize = 32;
const VERSION: &str = env!("FORGE_VERSION");
const TAGLINE: [&str; 2] = ["A developer's workshop for", "everyday transformations."];

// ─── sprites ───────────────────────────────────────────────────────────────
// Spaces are transparent: they never overwrite what is already on the canvas.

/// The hand: the swing turns about this cell and it never moves. Every pose's
/// handle runs up to the cell next to it, so the grip stays put while the head
/// swings through its arc.
const GRIP: (usize, usize) = (6, 25);

/// One hammer position: the steel head and the wooden handle, each with the
/// canvas (row, column) its top-left corner sits at.
struct Pose {
    head: &'static [&'static str],
    head_at: (usize, usize),
    handle: &'static [&'static str],
    handle_at: (usize, usize),
}

/// Raised (0°): handle straight up out of the grip, broad side of the head to
/// the viewer.
const RAISED: Pose = Pose {
    head: &["┏━━━━━━━┓", "┃███████┃", "┗━━━┳━━━┛"],
    head_at: (1, 21),
    handle: &["┃", "┃"],
    handle_at: (GRIP.0 - 2, GRIP.1),
};

/// Halfway round (45°): the head is a slab tilted down to the left and the
/// handle runs down to the right — two columns per row, because that is what
/// 45° looks like in character cells. Half blocks carry the slab's long edges
/// through the halves of a cell the steps would otherwise leave empty.
///
/// Kept to the same 3-row, 9-column footprint as [`RAISED`]: a bigger slab
/// reads as an oversized, stair-stepped afterimage rather than the same head
/// mid-turn.
const SWING: Pose = Pose {
    head: &["    ▄████", "  ▄████▀", "████▀"],
    head_at: (2, 17),
    handle: &["━╲", "  ━"],
    handle_at: (GRIP.0 - 2, GRIP.1 - 3),
};

/// On the anvil (90°): the head has turned face-down, so it now stands deep
/// and narrow, and the handle lies flat out to the grip.
const STRIKE: Pose = Pose {
    head: &["┏━━━━━┓", "┃█████┃", "┃█████┣", "┗━━━━━┛"],
    head_at: (4, 14),
    handle: &["━━━━"],
    handle_at: (GRIP.0, GRIP.1 - 4),
};

const ANVIL: [&str; 5] = [
    "     ▄▄▄▄▄▄▄▄▄▄▄     ",
    "  █████████████████  ",
    "  ▀▀▀▀▀▀█████▀▀▀▀▀▀  ",
    "        █████        ",
    "    █████████████    ",
];

/// Stamped into the anvil's flat top (row 1 of [`ANVIL`]) in every frame.
const LABEL: &str = "Dev Forge";
/// Row of [`ANVIL`] the label is centered on.
const LABEL_ROW: usize = 1;

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
    Label,
    Caption,
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
            Paint::Label => "\x1b[1;38;5;255m",
            Paint::Caption => "\x1b[38;5;245m",
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

    /// Like [`Canvas::draw`], but a space overwrites whatever was underneath
    /// instead of leaving it showing through. For text over a solid fill,
    /// where a space is a real gap rather than "nothing to draw here".
    fn draw_opaque(&mut self, sprite: &[&str], top: usize, left: usize, paint: Paint) {
        for (dy, line) in sprite.iter().enumerate() {
            for (dx, ch) in line.chars().enumerate() {
                self.put(top + dy, left + dx, ch, paint);
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
    /// `None` once the hammer has left the scene — just the anvil remains.
    pose: Option<&'static Pose>,
    glow: Option<Paint>,
    sparks: &'static [Spark],
    /// Rows sliced off the top of the canvas before this frame is printed.
    /// Only meaningful once the hammer is gone — those rows are the empty
    /// space it used to swing through, so cropping them is indistinguishable
    /// from them never having been there. Climbing this from 0 to
    /// [`VOID_HEIGHT`] over a few frames is what collapses that space away
    /// instead of just cutting it off in one jump.
    crop_top: usize,
}

/// Rows of empty space above the anvil once the hammer is gone (see
/// [`ANVIL_TOP`]) — the hammer's old swing room, with nothing left in it.
const VOID_HEIGHT: usize = ANVIL_TOP;

/// The whole animation, a little over 1 second. The last frame has no delay:
/// it is what stays on screen, the hammer gone, the void closed, and only the
/// anvil left standing.
const STORYBOARD: [Frame; 12] = [
    // Hammer up, waiting.
    Frame {
        delay_ms: 260,
        pose: Some(&RAISED),
        glow: None,
        sparks: &[],
        crop_top: 0,
    },
    // Coming round.
    Frame {
        delay_ms: 60,
        pose: Some(&SWING),
        glow: None,
        sparks: &[],
        crop_top: 0,
    },
    // Impact.
    Frame {
        delay_ms: 120,
        pose: Some(&STRIKE),
        glow: Some(Paint::GlowHot),
        sparks: &BURST,
        crop_top: 0,
    },
    Frame {
        delay_ms: 120,
        pose: Some(&STRIKE),
        glow: Some(Paint::GlowWarm),
        sparks: &SPREAD,
        crop_top: 0,
    },
    // Bouncing back up, sparks flying outward and cooling.
    Frame {
        delay_ms: 130,
        pose: Some(&SWING),
        glow: Some(Paint::GlowWarm),
        sparks: &DRIFTING,
        crop_top: 0,
    },
    Frame {
        delay_ms: 150,
        pose: Some(&RAISED),
        glow: Some(Paint::GlowDim),
        sparks: &EMBERS,
        crop_top: 0,
    },
    // Settled, just for a beat, before the hammer is gone.
    Frame {
        delay_ms: 180,
        pose: Some(&RAISED),
        glow: None,
        sparks: &[],
        crop_top: 0,
    },
    // The hammer is gone. One more beat before the void starts closing.
    Frame {
        delay_ms: 120,
        pose: None,
        glow: None,
        sparks: &[],
        crop_top: 0,
    },
    // The void collapses, the anvil rising to meet the top of the frame.
    Frame {
        delay_ms: 90,
        pose: None,
        glow: None,
        sparks: &[],
        crop_top: 2,
    },
    Frame {
        delay_ms: 90,
        pose: None,
        glow: None,
        sparks: &[],
        crop_top: 4,
    },
    Frame {
        delay_ms: 90,
        pose: None,
        glow: None,
        sparks: &[],
        crop_top: 6,
    },
    // Closed. Only the anvil, and its stamp, remain.
    Frame {
        delay_ms: 0,
        pose: None,
        glow: None,
        sparks: &[],
        crop_top: VOID_HEIGHT,
    },
];

fn scene(frame: &Frame) -> Canvas {
    let mut canvas = Canvas::new();

    let anvil_left = centered(ANVIL[0].chars().count());
    canvas.draw(&ANVIL, ANVIL_TOP, anvil_left, Paint::Anvil);

    let band_left = anvil_left + ANVIL[LABEL_ROW].chars().take_while(|&c| c == ' ').count();
    let band_width = ANVIL[LABEL_ROW].trim().chars().count();
    let label_col = band_left + (band_width - LABEL.chars().count()) / 2;
    canvas.draw_opaque(&[LABEL], ANVIL_TOP + LABEL_ROW, label_col, Paint::Label);

    canvas.draw_opaque(&[LABEL], ANVIL_TOP, TEXT_LEFT, Paint::Label);
    canvas.draw_opaque(&[VERSION], ANVIL_TOP + 1, TEXT_LEFT, Paint::Caption);
    canvas.draw_opaque(&TAGLINE, ANVIL_TOP + 3, TEXT_LEFT, Paint::Caption);

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

    if let Some(pose) = frame.pose {
        canvas.draw_pose(pose);
    }
    canvas
}

/// The still frame shown when the animation is skipped: the hammer raised
/// over the anvil, at rest — the same pose the animation opens on.
fn resting_scene() -> Canvas {
    scene(&Frame {
        delay_ms: 0,
        pose: Some(&RAISED),
        glow: None,
        sparks: &[],
        crop_top: 0,
    })
}

/// What the animation settles into once it finishes: hammer gone, just the
/// stamped anvil. Only used by tests; production code drives the same frame
/// straight off the end of [`STORYBOARD`].
#[cfg(test)]
fn settled_scene() -> Canvas {
    scene(&STORYBOARD[STORYBOARD.len() - 1])
}

// ─── output ────────────────────────────────────────────────────────────────

fn env_set(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|v| !v.is_empty())
}

fn dumb_terminal() -> bool {
    std::env::var("TERM").map(|t| t == "dumb").unwrap_or(false)
}

/// Whether output should carry colors.
///
/// Shared with the rest of the REPL, so the banner, the picker and the hint
/// lines all make the same call about a piped stdout or `NO_COLOR`.
pub fn use_color() -> bool {
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

    // How many lines the previous frame actually printed — frames shrink as
    // the void above the anvil closes, so this isn't always HEIGHT.
    let mut printed = 0;
    for (i, frame) in STORYBOARD.iter().enumerate() {
        if i > 0 {
            // \x1b[{n}F: move the cursor to the start of the line n rows up.
            let _ = write!(stdout, "\x1b[{}F", printed);
        }
        let lines = scene(frame).render(color);
        let visible = &lines[frame.crop_top..];
        for line in visible {
            // \x1b[K: clear to end of line, so a shorter frame cannot leave
            // remnants of the previous one behind.
            let _ = writeln!(stdout, "{}\x1b[K", line);
        }
        if visible.len() < printed {
            // \x1b[J: erase from the cursor to the end of the screen — this
            // frame is shorter than the last, so rows are left over below.
            let _ = write!(stdout, "\x1b[J");
        }
        let _ = stdout.flush();
        printed = visible.len();
        if frame.delay_ms > 0 {
            sleep(Duration::from_millis(frame.delay_ms));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The poses in swing order.
    const POSES: [&Pose; 3] = [&RAISED, &SWING, &STRIKE];
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
    fn animation_opens_on_the_resting_pose() {
        assert_eq!(
            scenes().first().unwrap().render(false),
            resting_scene().render(false)
        );
    }

    #[test]
    fn animation_ends_with_the_hammer_gone() {
        let last = STORYBOARD.last().unwrap();
        assert!(last.pose.is_none(), "the hammer is still on screen");
        assert_eq!(last.delay_ms, 0, "last frame must not sleep");
        assert_eq!(
            scenes().last().unwrap().render(false),
            settled_scene().render(false)
        );
        assert_eq!(
            last.crop_top, VOID_HEIGHT,
            "the void must be fully closed by the last frame"
        );
    }

    #[test]
    fn cropping_never_happens_while_the_hammer_is_still_there() {
        // crop_top slices rows off the top of the canvas on the assumption
        // that they're empty. The hammer draws into exactly that space, so
        // cropping while it's still on screen would cut the hammer itself.
        for frame in &STORYBOARD {
            if frame.pose.is_some() {
                assert_eq!(frame.crop_top, 0, "cropped a frame the hammer is still in");
            }
        }
    }

    #[test]
    fn the_void_closes_monotonically_and_never_past_itself() {
        let crops: Vec<usize> = STORYBOARD.iter().map(|f| f.crop_top).collect();
        assert!(
            crops.windows(2).all(|w| w[0] <= w[1]),
            "the void reopens partway through: {:?}",
            crops
        );
        assert!(
            crops.iter().all(|&c| c <= VOID_HEIGHT),
            "cropped past the void, into the anvil: {:?}",
            crops
        );
    }

    #[test]
    fn cropped_rows_are_always_actually_blank() {
        // The whole trick only works because these rows have nothing in
        // them — cropping is then indistinguishable from them never having
        // been drawn. If a future frame put something in the void while
        // still cropping it, this is the test that would catch it.
        for (frame, canvas) in STORYBOARD.iter().zip(scenes()) {
            let lines = canvas.render(false);
            for line in &lines[..frame.crop_top] {
                assert!(
                    line.is_empty(),
                    "row inside the cropped void isn't blank: {:?}",
                    line
                );
            }
        }
    }

    #[test]
    fn the_label_is_stamped_into_the_anvil_in_every_frame() {
        for canvas in scenes() {
            assert!(
                canvas.render(false).iter().any(|line| line.contains(LABEL)),
                "{:?} is missing from a frame",
                LABEL
            );
        }
        assert!(resting_scene()
            .render(false)
            .iter()
            .any(|line| line.contains(LABEL)));
    }

    #[test]
    fn the_side_text_shows_in_every_frame() {
        for canvas in scenes() {
            let lines = canvas.render(false);
            for text in [LABEL, VERSION, TAGLINE[0], TAGLINE[1]] {
                assert!(
                    lines.iter().any(|line| line.contains(text)),
                    "{:?} is missing from a frame",
                    text
                );
            }
        }
    }

    #[test]
    fn the_side_text_clears_the_hammers_reach() {
        // The side text is drawn in every frame regardless of the hammer's
        // pose, so it must sit to the right of anywhere the hammer ever
        // reaches — otherwise a swing frame would draw over it.
        let reach = POSES
            .iter()
            .flat_map(|pose| {
                cells_of(pose.head, pose.head_at)
                    .into_iter()
                    .chain(cells_of(pose.handle, pose.handle_at))
            })
            .map(|(_, col)| col)
            .max()
            .unwrap();
        assert!(
            TEXT_LEFT > reach,
            "the hammer reaches column {}, level with or past the side text at {}",
            reach,
            TEXT_LEFT
        );
    }

    #[test]
    fn no_hammer_glyph_survives_into_the_settled_scene() {
        // Anything the head or handle draws with that the anvil and its
        // label never use — if one of these turns up, the hammer didn't
        // actually leave.
        let hammer_only = ['┏', '┓', '┗', '┛', '┣', '┳', '┃', '━', '╲'];
        for line in settled_scene().render(false) {
            assert!(
                !line.contains(hammer_only.as_slice()),
                "hammer glyph left behind: {:?}",
                line
            );
        }
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

    /// Cells a sprite covers on the canvas.
    fn cells_of(art: &[&str], (top, left): (usize, usize)) -> Vec<(usize, usize)> {
        art.iter()
            .enumerate()
            .flat_map(|(dy, line)| {
                line.chars()
                    .enumerate()
                    .filter(|(_, ch)| *ch != ' ')
                    .map(move |(dx, _)| (top + dy, left + dx))
            })
            .collect()
    }

    #[test]
    fn the_swing_turns_about_the_grip() {
        for pose in POSES {
            let handle = cells_of(pose.handle, pose.handle_at);
            let &(row, col) = handle.last().expect("every pose shows a handle");
            let (grip_row, grip_col) = GRIP;

            assert!(
                row.abs_diff(grip_row) <= 1 && col.abs_diff(grip_col) <= 1,
                "the handle ends at {:?}, away from the grip {:?}",
                (row, col),
                GRIP
            );
            assert!(
                !cells_of(pose.head, pose.head_at).contains(&GRIP) && !handle.contains(&GRIP),
                "the hand is not something to draw over"
            );
        }
    }

    /// A sprite's bounding box, in cells (width, height).
    fn footprint(art: &[&str]) -> usize {
        let width = art.iter().map(|line| line.chars().count()).max().unwrap();
        width * art.len()
    }

    #[test]
    fn the_swing_never_bulges_past_either_end_of_the_arc() {
        // RAISED and STRIKE are the same rigid head at 0° and 90°, so their
        // bounding boxes are the two honest views of it; a rotated view drawn
        // in low-resolution character cells cannot be made pixel-exact, but it
        // must not look like a bigger, blurrier object than either end — that
        // reads as a stair-stepped afterimage instead of a mid-swing head.
        let ends = footprint(RAISED.head).max(footprint(STRIKE.head));
        let mid = footprint(SWING.head);
        assert!(
            mid <= ends,
            "the mid-swing head ({} cells) is bigger than both ends ({} cells)",
            mid,
            ends
        );
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
                pose: Some(pose),
                glow: None,
                sparks: &[],
                crop_top: 0,
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

        // The blow lands on the first frame that heats the anvil.
        let impact = STORYBOARD
            .iter()
            .position(|frame| frame.glow.is_some())
            .expect("the hammer lands");

        assert!(
            !sparky[..impact].iter().any(|&s| s),
            "no sparks before impact"
        );
        assert!(sparky[impact], "impact frame throws sparks");
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
