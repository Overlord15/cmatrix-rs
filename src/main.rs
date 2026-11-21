//! cmatrix 2.0 - Rust port for the Windows terminal.
//!
//! Original C version: Copyright (C) 1999-2017 Chris Allegretta,
//! Copyright (C) 2017-Present Abishek V Ashok (GPL-3.0-or-later).
//! This port keeps the same options, key bindings and animation logic.

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute, queue,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
    terminal::{self, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self, BufWriter, Write};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = "2.0";

// ---------------------------------------------------------------- colours

#[derive(Clone, Copy, PartialEq)]
enum Clr {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Clr {
    fn to_color(self) -> Color {
        match self {
            Clr::Black => Color::Black,
            Clr::Red => Color::DarkRed,
            Clr::Green => Color::DarkGreen,
            Clr::Yellow => Color::DarkYellow,
            Clr::Blue => Color::DarkBlue,
            Clr::Magenta => Color::DarkMagenta,
            Clr::Cyan => Color::DarkCyan,
            Clr::White => Color::Grey, // ANSI "white" (37), like curses COLOR_WHITE
        }
    }
}

// -------------------------------------------------------------------- rng

struct Rng(u64);

impl Rng {
    fn new() -> Self {
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        Rng((t ^ 0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u32 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 32) as u32
    }
    /// like `rand() % n` (n must be > 0)
    fn below(&mut self, n: i32) -> i32 {
        (self.next() % (n.max(1) as u32)) as i32
    }
}

// ----------------------------------------------------------------- matrix

#[derive(Clone, Copy)]
struct Cell {
    val: i32,
    is_head: bool,
}

struct Matrix {
    lines: usize,
    cols: usize,
    cells: Vec<Cell>, // (lines + 1) rows x cols
    length: Vec<i32>, // length of the stream in each column
    spaces: Vec<i32>, // spaces left to fill
    updates: Vec<i32>, // per column update speed (used by async mode)
}

impl Matrix {
    fn new(lines: usize, cols: usize, rng: &mut Rng) -> Self {
        let mut m = Matrix {
            lines,
            cols,
            cells: vec![Cell { val: -1, is_head: false }; (lines + 1) * cols],
            length: vec![0; cols],
            spaces: vec![0; cols],
            updates: vec![0; cols],
        };
        let l = lines as i32;
        let mut j = 0;
        while j < cols {
            m.spaces[j] = rng.below(l) + 1;
            m.length[j] = rng.below(l - 3) + 3;
            m.at_mut(1, j).val = ' ' as i32; // sentinel for creation of new objects
            m.updates[j] = rng.below(3) + 1;
            j += 2;
        }
        m
    }
    #[inline]
    fn at(&self, i: usize, j: usize) -> &Cell {
        &self.cells[i * self.cols + j]
    }
    #[inline]
    fn at_mut(&mut self, i: usize, j: usize) -> &mut Cell {
        let c = self.cols;
        &mut self.cells[i * c + j]
    }
    #[inline]
    fn val(&self, i: usize, j: usize) -> i32 {
        self.at(i, j).val
    }
}

// --------------------------------------------------------------- options

struct Opts {
    screensaver: bool,
    asynch: bool,
    bold: i32, // -1/0 none, 1 some, 2 all
    oldstyle: bool,
    update: i32,
    color: Clr,
    rainbow: bool,
    lambda: bool,
    classic: bool,
    console: bool,
    xwindow: bool,
    lock: bool,
}

fn usage() {
    println!(" Usage: cmatrix -[abBcfhlsmVx] [-u delay] [-C color]");
    println!(" -a: Asynchronous scroll");
    println!(" -b: Bold characters on");
    println!(" -B: All bold characters (overrides -b)");
    println!(" -c: Use Japanese characters as seen in the original matrix. Requires appropriate fonts");
    println!(" -f: Force the linux $TERM type to be on (no effect on Windows)");
    println!(" -l: Linux mode (uses matrix console font) (no effect on Windows)");
    println!(" -L: Lock mode (cannot be closed with q / Ctrl+C)");
    println!(" -o: Use old-style scrolling");
    println!(" -h: Print usage and exit");
    println!(" -n: No bold characters (overrides -b and -B, default)");
    println!(" -s: \"Screensaver\" mode, exits on first keystroke");
    println!(" -x: X window mode, use if your xterm is using mtx.pcf");
    println!(" -V: Print version information and exit");
    println!(" -u delay (0 - 10, default 4): Screen update delay");
    println!(" -C [color]: Use this color for matrix (default green)");
    println!(" -r: rainbow mode");
    println!(" -m: lambda mode");
}

fn version() {
    println!(" CMatrix version {} (Rust port for Windows)", VERSION);
    println!("Original: https://github.com/abishekvashok/cmatrix");
}

/// C-style atoi
fn atoi(s: &str) -> i32 {
    let s = s.trim_start();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    let n: i32 = digits.parse().unwrap_or(0);
    if neg {
        -n
    } else {
        n
    }
}

fn parse_color(s: &str) -> Option<Clr> {
    match s.to_ascii_lowercase().as_str() {
        "green" => Some(Clr::Green),
        "red" => Some(Clr::Red),
        "blue" => Some(Clr::Blue),
        "white" => Some(Clr::White),
        "yellow" => Some(Clr::Yellow),
        "cyan" => Some(Clr::Cyan),
        "magenta" => Some(Clr::Magenta),
        "black" => Some(Clr::Black),
        _ => None,
    }
}

/// getopt-style parser for "abBcfhlLnrosmxVu:C:" (clustered flags supported)
fn parse_args(args: &[String]) -> Opts {
    let mut o = Opts {
        screensaver: false,
        asynch: false,
        bold: 0,
        oldstyle: false,
        update: 4,
        color: Clr::Green,
        rainbow: false,
        lambda: false,
        classic: false,
        console: false,
        xwindow: false,
        lock: false,
    };
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        i += 1;
        if a == "--" {
            break;
        }
        if !a.starts_with('-') || a.len() == 1 {
            continue;
        }
        let chars: Vec<char> = a[1..].chars().collect();
        let mut k = 0;
        while k < chars.len() {
            let c = chars[k];
            k += 1;
            match c {
                's' => o.screensaver = true,
                'a' => o.asynch = true,
                'b' => {
                    if o.bold != 2 {
                        o.bold = 1;
                    }
                }
                'B' => o.bold = 2,
                'c' => o.classic = true,
                'f' => {} // forcing TERM=linux is meaningless on Windows
                'l' => o.console = true,
                'L' => o.lock = true,
                'n' => o.bold = -1,
                'o' => o.oldstyle = true,
                'x' => o.xwindow = true,
                'r' => o.rainbow = true,
                'm' => o.lambda = true,
                'V' => {
                    version();
                    std::process::exit(0);
                }
                'u' | 'C' => {
                    let optarg: String = if k < chars.len() {
                        let s: String = chars[k..].iter().collect();
                        k = chars.len();
                        s
                    } else if i < args.len() {
                        i += 1;
                        args[i - 1].clone()
                    } else {
                        usage();
                        std::process::exit(0);
                    };
                    if c == 'u' {
                        o.update = atoi(&optarg);
                    } else {
                        match parse_color(&optarg) {
                            Some(cl) => o.color = cl,
                            None => {
                                eprintln!(
                                    " Invalid color selection\n Valid colors are green, red, blue, white, yellow, cyan, magenta and black."
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                }
                _ => {
                    // 'h' and unknown options ('?')
                    usage();
                    std::process::exit(0);
                }
            }
        }
    }
    o
}

// -------------------------------------------------------------- terminal

/// Restores the console on every exit path (including panics).
struct TermGuard;

impl TermGuard {
    fn new() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(ClearType::All)
        )?;
        Ok(TermGuard)
    }
}

impl Drop for TermGuard {
    fn drop(&mut self) {
        let _ = execute!(
            io::stdout(),
            SetAttribute(Attribute::Reset),
            cursor::Show,
            LeaveAlternateScreen
        );
        let _ = terminal::disable_raw_mode();
    }
}

fn term_size() -> (usize, usize) {
    let (w, h) = terminal::size().unwrap_or((80, 24));
    (w as usize, h as usize)
}

fn char_of(v: i32) -> char {
    if v <= 0 {
        ' '
    } else {
        char::from_u32(v as u32).unwrap_or(' ')
    }
}

type Out = BufWriter<io::Stdout>;

fn set_style(
    out: &mut Out,
    cur: &mut Option<(Color, bool)>,
    color: Color,
    bold: bool,
) -> io::Result<()> {
    if *cur != Some((color, bold)) {
        queue!(out, SetAttribute(Attribute::Reset), SetForegroundColor(color))?;
        if bold {
            queue!(out, SetAttribute(Attribute::Bold))?;
        }
        *cur = Some((color, bold));
    }
    Ok(())
}

// ------------------------------------------------------------------ main

fn run(mut o: Opts) -> io::Result<()> {
    let mut rng = Rng::new();

    // character range, same as the original
    let (randmin, highnum): (i32, i32) = if o.classic {
        (12288, 12351) // Japanese range of the original
    } else if o.console || o.xwindow {
        (166, 217)
    } else {
        (33, 123)
    };
    let randnum = highnum - randmin;
    let alt = o.console || o.xwindow;

    let _guard = TermGuard::new()?;
    let mut out: Out = BufWriter::with_capacity(1 << 16, io::stdout());

    let (rc, rr) = term_size();
    let (mut real_cols, mut real_rows) = (rc, rr);
    let mut m = Matrix::new(real_rows.max(10), real_cols.max(10), &mut rng);

    let mut count = 0;
    let mut pause = false;

    loop {
        count += 1;
        if count > 4 {
            count = 1;
        }

        // ------------------------------------------------ input / events
        while event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Resize(w, h) => {
                    real_cols = w as usize;
                    real_rows = h as usize;
                    m = Matrix::new(real_rows.max(10), real_cols.max(10), &mut rng);
                    queue!(out, SetAttribute(Attribute::Reset), terminal::Clear(ClearType::All))?;
                }
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    // Ctrl+C == SIGINT
                    if k.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(k.code, KeyCode::Char('c') | KeyCode::Char('C'))
                    {
                        if !o.lock {
                            return Ok(());
                        }
                        continue;
                    }
                    if o.screensaver {
                        return Ok(());
                    }
                    if let KeyCode::Char(c) = k.code {
                        match c {
                            'q' => {
                                if !o.lock {
                                    return Ok(());
                                }
                            }
                            'a' => o.asynch = !o.asynch,
                            'b' => o.bold = 1,
                            'B' => o.bold = 2,
                            'L' => o.lock = true,
                            'n' => o.bold = 0,
                            '0'..='9' => o.update = c as i32 - '0' as i32,
                            '!' => {
                                o.color = Clr::Red;
                                o.rainbow = false;
                            }
                            '@' => {
                                o.color = Clr::Green;
                                o.rainbow = false;
                            }
                            '#' => {
                                o.color = Clr::Yellow;
                                o.rainbow = false;
                            }
                            '$' => {
                                o.color = Clr::Blue;
                                o.rainbow = false;
                            }
                            '%' => {
                                o.color = Clr::Magenta;
                                o.rainbow = false;
                            }
                            '^' => {
                                o.color = Clr::Cyan;
                                o.rainbow = false;
                            }
                            '&' => {
                                o.color = Clr::White;
                                o.rainbow = false;
                            }
                            'r' => o.rainbow = true,
                            'm' => o.lambda = !o.lambda,
                            'p' | 'P' => pause = !pause,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        // ------------------------------------------- update + draw columns
        let lines = m.lines;
        let cols = m.cols;
        let l = lines as i32;
        let mut cur: Option<(Color, bool)> = None;

        let mut j = 0;
        while j < cols {
            if (count > m.updates[j] || !o.asynch) && !pause {
                if o.oldstyle {
                    // old-style scrolling
                    for i in (1..lines).rev() {
                        let v = m.val(i - 1, j);
                        m.at_mut(i, j).val = v;
                    }
                    let random = rng.below(randnum + 8) + randmin;
                    let v1 = m.val(1, j);

                    if v1 == 0 {
                        m.at_mut(0, j).val = 1;
                    } else if v1 == ' ' as i32 || v1 == -1 {
                        if m.spaces[j] > 0 {
                            m.at_mut(0, j).val = ' ' as i32;
                            m.spaces[j] -= 1;
                        } else {
                            // random: does the next column of chars start with a white 'head'?
                            if rng.below(3) == 1 {
                                m.at_mut(0, j).val = 0;
                            } else {
                                m.at_mut(0, j).val = rng.below(randnum) + randmin;
                            }
                            m.spaces[j] = rng.below(l) + 1;
                        }
                    } else if random > highnum && v1 != 1 {
                        m.at_mut(0, j).val = ' ' as i32;
                    } else {
                        m.at_mut(0, j).val = rng.below(randnum) + randmin;
                    }
                } else {
                    // new-style scrolling (default)
                    if m.val(0, j) == -1 && m.val(1, j) == ' ' as i32 && m.spaces[j] > 0 {
                        m.at_mut(0, j).val = -1;
                        m.spaces[j] -= 1;
                    } else if m.val(0, j) == -1 && m.val(1, j) == ' ' as i32 {
                        m.length[j] = rng.below(l - 3) + 3;
                        m.at_mut(0, j).val = rng.below(randnum) + randmin;
                        m.spaces[j] = rng.below(l) + 1;
                    }
                    let mut i = 0usize;
                    let mut firstcoldone = false;
                    while i <= lines {
                        // skip over spaces
                        while i <= lines && (m.val(i, j) == ' ' as i32 || m.val(i, j) == -1) {
                            i += 1;
                        }
                        if i > lines {
                            break;
                        }
                        // go to the head of this column
                        let z = i;
                        let mut y = 0;
                        while i <= lines && (m.val(i, j) != ' ' as i32 && m.val(i, j) != -1) {
                            m.at_mut(i, j).is_head = false;
                            i += 1;
                            y += 1;
                        }
                        if i > lines {
                            m.at_mut(z, j).val = ' ' as i32;
                            continue;
                        }
                        m.at_mut(i, j).val = rng.below(randnum) + randmin;
                        m.at_mut(i, j).is_head = true;

                        // reached full length: start moving down
                        if y > m.length[j] || firstcoldone {
                            m.at_mut(z, j).val = ' ' as i32;
                            m.at_mut(0, j).val = -1;
                        }
                        firstcoldone = true;
                        i += 1;
                    }
                }
            }

            // ---- draw this column
            let (y0, z) = if !o.oldstyle { (1, lines) } else { (0, lines - 1) };
            // wide (CJK) glyphs need 2 cells; don't let them wrap on the last column
            let fits = !o.classic || j + 1 < real_cols;
            if j < real_cols && fits {
                for i in y0..=z {
                    let row = i - y0;
                    if row >= real_rows {
                        break;
                    }
                    let c = *m.at(i, j);
                    queue!(out, cursor::MoveTo(j as u16, row as u16))?;

                    let head = (c.is_head || (o.oldstyle && c.val == 0)) && !o.rainbow;
                    if head {
                        set_style(&mut out, &mut cur, Clr::White.to_color(), o.bold > 0)?;
                        if c.val == 0 {
                            queue!(out, Print(if alt { '\u{b7}' } else { '&' }))?;
                        } else {
                            queue!(out, Print(char_of(c.val)))?;
                        }
                    } else {
                        let color = if o.rainbow {
                            match rng.below(6) {
                                0 => Clr::Green,
                                1 => Clr::Blue,
                                2 => Clr::Black,
                                3 => Clr::Yellow,
                                4 => Clr::Cyan,
                                _ => Clr::Magenta,
                            }
                        } else {
                            o.color
                        };
                        if c.val == 1 {
                            set_style(&mut out, &mut cur, color.to_color(), o.bold > 0)?;
                            queue!(out, Print('|'))?;
                        } else {
                            let b = o.bold == 2 || (o.bold == 1 && c.val % 2 == 0);
                            set_style(&mut out, &mut cur, color.to_color(), b)?;
                            if c.val == -1 {
                                queue!(out, Print(' '))?;
                            } else if o.lambda && c.val != ' ' as i32 {
                                queue!(out, Print('\u{3bb}'))?; // λ
                            } else {
                                queue!(out, Print(char_of(c.val)))?;
                            }
                        }
                    }
                }
            }
            j += 2;
        }

        // ------------------------------------------------- lock message
        if o.lock {
            queue!(out, SetAttribute(Attribute::Reset))?;
            let msg = "Computer locked.";
            let n = msg.len() as i32;
            let msg_row = (lines / 2) as i32;
            let msg_col = ((cols / 2) as i32 - n / 2 - 2).max(0);
            let blank = " ".repeat((n + 4) as usize);
            let row_ok = |r: i32| r >= 0 && (r as usize) < real_rows;
            if row_ok(msg_row - 1) {
                queue!(out, cursor::MoveTo(msg_col as u16, (msg_row - 1) as u16), Print(&blank))?;
            }
            if row_ok(msg_row) {
                queue!(
                    out,
                    cursor::MoveTo(msg_col as u16, msg_row as u16),
                    Print(format!("  {}  ", msg))
                )?;
            }
            if row_ok(msg_row + 1) {
                queue!(out, cursor::MoveTo(msg_col as u16, (msg_row + 1) as u16), Print(&blank))?;
            }
        }

        out.flush()?;

        // napms(update * 10)
        let ms = o.update.max(0) as u64 * 10;
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opts = parse_args(&args);
    if let Err(e) = run(opts) {
        eprintln!("cmatrix: {}", e);
        std::process::exit(1);
    }
}
