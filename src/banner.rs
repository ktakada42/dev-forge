use std::{io::Write, thread::sleep, time::Duration};

const Y: &str = "\x1b[33m";        // yellow
const O: &str = "\x1b[38;5;208m";  // orange
const R: &str = "\x1b[0m";         // reset

// ─── hammer shape (side view, 3-D) ─────────────────────────────────────────
//
//           │             ← handle (enters center-top of head)
//           │
//   ┌───────┴───────┐
//   │░░░░░░░░░░░▒▒▒▒├┐    ← face (░ lit, ▒ shadow near depth)
//   │░░░░░░░░░░░▒▒▒▒││
//   └───────────────┘│    ← striking face (bottom)
//    └───────────────┘    ← depth back-edge
//
// Sparks appear on lines 8-9, flying left and right from the strike point.

const W: usize = 22; // visible line width (all frames share this)

pub fn animate() {
    let frames: Vec<(u64, Vec<String>)> = vec![
        // ── 1: hammer raised, handle visible ──────────────────────── 300 ms
        (300, plain(&[
            "          │           ",
            "          │           ",
            "  ┌───────┴───────┐   ",
            "  │░░░░░░░░░░░▒▒▒▒├┐  ",
            "  │░░░░░░░░░░░▒▒▒▒││  ",
            "  └───────────────┘│  ",
            "   └───────────────┘  ",
            "                      ",
            "                      ",
            "                      ",
        ])),
        // ── 2: swinging down (fast) ────────────────────────────────── 80 ms
        (80, plain(&[
            "                      ",
            "          │           ",
            "          │           ",
            "  ┌───────┴───────┐   ",
            "  │░░░░░░░░░░░▒▒▒▒├┐  ",
            "  │░░░░░░░░░░░▒▒▒▒││  ",
            "  └───────────────┘│  ",
            "   └───────────────┘  ",
            "                      ",
            "                      ",
        ])),
        // ── 3: just before impact (handle tip only) ────────────────── 60 ms
        (60, plain(&[
            "                      ",
            "                      ",
            "          │           ",
            "  ┌───────┴───────┐   ",
            "  │░░░░░░░░░░░▒▒▒▒├┐  ",
            "  │░░░░░░░░░░░▒▒▒▒││  ",
            "  └───────────────┘│  ",
            "   └───────────────┘  ",
            "                      ",
            "                      ",
        ])),
        // ── 4: IMPACT — sparks burst left and right ──────────────── 200 ms
        (200, {
            let mut f = plain(&[
                "                      ",
                "                      ",
                "          │           ",
                "  ┌───────┴───────┐   ",
                "  │░░░░░░░░░░░▒▒▒▒├┐  ",
                "  │░░░░░░░░░░░▒▒▒▒││  ",
                "  └───────────────┘│  ",
                "   └───────────────┘  ",
            ]);
            // sparks close to impact (dense, bright)
            f.push(format!(" {Y}✦{R}{O}*{R}{Y}·{R}             {Y}·{R}{O}*{R}{Y}✦{R} "));
            // sparks one row below (wider spread)
            f.push(format!("{Y}✦{R} {O}·{R}               {O}·{R} {Y}✦{R}"));
            f
        }),
        // ── 5: sparks fading ─────────────────────────────────────── 220 ms
        (220, {
            let mut f = plain(&[
                "                      ",
                "                      ",
                "          │           ",
                "  ┌───────┴───────┐   ",
                "  │░░░░░░░░░░░▒▒▒▒├┐  ",
                "  │░░░░░░░░░░░▒▒▒▒││  ",
                "  └───────────────┘│  ",
                "   └───────────────┘  ",
            ]);
            f.push(format!("   {Y}·{R}               {Y}·{R}   "));
            f.push("                      ".into());
            f
        }),
        // ── 6: settled ────────────────────────────────────────────────── 0 ms
        (0, plain(&[
            "                      ",
            "                      ",
            "          │           ",
            "  ┌───────┴───────┐   ",
            "  │░░░░░░░░░░░▒▒▒▒├┐  ",
            "  │░░░░░░░░░░░▒▒▒▒││  ",
            "  └───────────────┘│  ",
            "   └───────────────┘  ",
            "                      ",
            "                      ",
        ])),
    ];

    debug_assert!(
        frames.iter().all(|(_, lines)| lines.len() == 10),
        "all frames must have 10 lines"
    );

    let height = frames[0].1.len();
    let mut stdout = std::io::stdout();

    for (i, (delay_ms, lines)) in frames.iter().enumerate() {
        if i > 0 {
            // \x1b[nF: cursor to start of line n rows up
            print!("\x1b[{}F", height);
        }
        for line in lines {
            // \x1b[K: clear to end of line (removes leftovers if prev line was wider)
            println!("{}\x1b[K", line);
        }
        stdout.flush().ok();
        if *delay_ms > 0 {
            sleep(Duration::from_millis(*delay_ms));
        }
    }

    let _ = W; // width constant kept for documentation
}

fn plain(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|s| s.to_string()).collect()
}
