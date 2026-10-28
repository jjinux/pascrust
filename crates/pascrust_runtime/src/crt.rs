//! Turbo Pascal's `Crt` unit: screen and keyboard control.
//!
//! On DOS, `Crt` wrote straight into video memory: an 80×25 grid of
//! characters with colour attributes. Here it drives a terminal with
//! `crossterm`. While `Crt` is active the terminal is in *raw mode*: keys
//! arrive one at a time without echo (so `ReadKey` works), and the runtime
//! does its own line editing for `Readln` and translates line breaks to
//! `\r\n` on output, as the terminal no longer does that for us.
//!
//! When standard output isn't a terminal (in tests, or when piping output),
//! `Crt` runs in *plain mode*: cursor movement and colours are ignored, and
//! `ReadKey` reads characters from standard input, so interactive programs
//! can be driven by a script. `PASCRUST_TERM=plain` forces plain mode.

use std::cell::RefCell;
use std::io::{IsTerminal, Write};
use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::style::{Attribute, Color, SetAttribute, SetBackgroundColor, SetForegroundColor};
use crossterm::{QueueableCommand, cursor, terminal};

// Colour constants, in the CGA order Turbo Pascal used.
pub const BLACK: u8 = 0;
pub const BLUE: u8 = 1;
pub const GREEN: u8 = 2;
pub const CYAN: u8 = 3;
pub const RED: u8 = 4;
pub const MAGENTA: u8 = 5;
pub const BROWN: u8 = 6;
pub const LIGHT_GRAY: u8 = 7;
pub const DARK_GRAY: u8 = 8;
pub const LIGHT_BLUE: u8 = 9;
pub const LIGHT_GREEN: u8 = 10;
pub const LIGHT_CYAN: u8 = 11;
pub const LIGHT_RED: u8 = 12;
pub const LIGHT_MAGENTA: u8 = 13;
pub const YELLOW: u8 = 14;
pub const WHITE: u8 = 15;
pub const BLINK: u8 = 128;
// Text modes for `TextMode`.
pub const BW40: u8 = 0;
pub const CO40: u8 = 1;
pub const BW80: u8 = 2;
pub const CO80: u8 = 3;
pub const MONO: u8 = 7;

/// The DOS screen size that Turbo Pascal programs were written for.
const SCREEN_COLS: u16 = 80;
const SCREEN_ROWS: u16 = 25;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Not started yet: the first `Crt` call decides.
    Off,
    Terminal,
    Plain,
}

struct CrtState {
    mode: Mode,
    /// Text attribute: foreground in bits 0-3, background in bits 4-6,
    /// blink in bit 7, exactly like `TextAttr` in TP.
    attr: u8,
    /// The current window, 1-based and inclusive, in screen coordinates.
    window: (u16, u16, u16, u16),
    /// The second half of an extended key (arrows, function keys), which
    /// `ReadKey` returns after first returning `#0`.
    pending_scan_code: Option<u8>,
}

thread_local! {
    static CRT: RefCell<CrtState> = const {
        RefCell::new(CrtState {
            mode: Mode::Off,
            attr: LIGHT_GRAY,
            window: (1, 1, SCREEN_COLS, SCREEN_ROWS),
            pending_scan_code: None,
        })
    };
}

/// Start `Crt` if it isn't running yet, and return the mode it's in.
fn mode() -> Mode {
    let mode = CRT.with_borrow(|crt| crt.mode);
    if mode != Mode::Off {
        return mode;
    }
    let plain = std::env::var("PASCRUST_TERM").is_ok_and(|v| v == "plain")
        || !std::io::stdout().is_terminal()
        || !std::io::stdin().is_terminal();
    let mode = if plain || terminal::enable_raw_mode().is_err() { Mode::Plain } else { Mode::Terminal };
    CRT.with_borrow_mut(|crt| crt.mode = mode);
    mode
}

/// True while `Crt` owns the terminal in raw mode.
pub(crate) fn raw_mode_active() -> bool {
    CRT.with_borrow(|crt| crt.mode == Mode::Terminal)
}

/// Put the terminal back the way we found it. Called when the program ends,
/// halts, or stops with a runtime error.
pub(crate) fn restore() {
    if raw_mode_active() {
        let mut out = std::io::stdout();
        let _ = out.queue(SetAttribute(Attribute::Reset));
        let _ = out.queue(cursor::Show);
        let _ = out.flush();
        let _ = terminal::disable_raw_mode();
        CRT.with_borrow_mut(|crt| crt.mode = Mode::Off);
    }
}

/// Run a terminal command, ignoring errors (as TP's Crt couldn't fail), but
/// only in terminal mode.
fn terminal_do(f: impl FnOnce(&mut std::io::Stdout) -> std::io::Result<()>) {
    if mode() == Mode::Terminal {
        crate::io::flush_stdout();
        let mut out = std::io::stdout();
        let _ = f(&mut out);
        let _ = out.flush();
    }
}

/// Map a CGA colour number to the terminal's 16 ANSI colours. CGA puts
/// blue at 1 and red at 4; ANSI does it the other way round.
fn ansi_color(cga: u8) -> Color {
    const MAP: [u8; 16] = [0, 4, 2, 6, 1, 5, 3, 7, 8, 12, 10, 14, 9, 13, 11, 15];
    Color::AnsiValue(MAP[(cga & 15) as usize])
}

fn apply_attr(out: &mut std::io::Stdout, attr: u8) -> std::io::Result<()> {
    out.queue(SetForegroundColor(ansi_color(attr & 15)))?;
    out.queue(SetBackgroundColor(ansi_color((attr >> 4) & 7)))?;
    out.queue(SetAttribute(if attr & BLINK != 0 { Attribute::SlowBlink } else { Attribute::NoBlink }))?;
    Ok(())
}

fn set_attr(attr: u8) {
    CRT.with_borrow_mut(|crt| crt.attr = attr);
    terminal_do(|out| apply_attr(out, attr));
}

fn attr() -> u8 {
    CRT.with_borrow(|crt| crt.attr)
}

fn current_window() -> (u16, u16, u16, u16) {
    CRT.with_borrow(|crt| crt.window)
}

/// `TextColor(c)`: set the foreground colour (add `Blink` to blink).
pub fn text_color(color: impl Into<i32>) {
    let c = color.into() as u8;
    set_attr((attr() & 0x70) | (c & 0x8f));
}

/// `TextBackground(c)`: set the background colour (0..7).
pub fn text_background(color: impl Into<i32>) {
    let c = color.into() as u8;
    set_attr((attr() & 0x8f) | ((c & 7) << 4));
}

/// `HighVideo`, `LowVideo`, `NormVideo`: brighten, dim, or reset the text
/// colour.
pub fn high_video() {
    set_attr(attr() | 8);
}

pub fn low_video() {
    set_attr(attr() & !8);
}

pub fn norm_video() {
    set_attr(LIGHT_GRAY);
}

/// The `TextAttr` variable.
pub fn text_attr() -> u8 {
    attr()
}

/// `ClrScr`: clear the window in the current background colour and move
/// the cursor to its top-left corner.
pub fn clr_scr() {
    let (x1, y1, x2, y2) = current_window();
    let full_screen = (x1, y1, x2, y2) == (1, 1, SCREEN_COLS, SCREEN_ROWS);
    terminal_do(|out| {
        apply_attr(out, attr())?;
        if full_screen {
            out.queue(terminal::Clear(terminal::ClearType::All))?;
        } else {
            let blank = " ".repeat((x2 - x1 + 1) as usize);
            for y in y1..=y2 {
                out.queue(cursor::MoveTo(x1 - 1, y - 1))?;
                write!(out, "{blank}")?;
            }
        }
        out.queue(cursor::MoveTo(x1 - 1, y1 - 1))?;
        Ok(())
    });
}

/// `ClrEol`: clear from the cursor to the end of the line.
pub fn clr_eol() {
    terminal_do(|out| {
        apply_attr(out, attr())?;
        out.queue(terminal::Clear(terminal::ClearType::UntilNewLine))?;
        Ok(())
    });
}

/// `GotoXY(x, y)`: move the cursor, in window-relative 1-based coordinates.
pub fn goto_xy(x: impl Into<i32>, y: impl Into<i32>) {
    let (x, y) = (x.into(), y.into());
    let (x1, y1, x2, y2) = current_window();
    // TP ignores coordinates outside the window.
    if x < 1 || y < 1 || x > (x2 - x1 + 1) as i32 || y > (y2 - y1 + 1) as i32 {
        return;
    }
    terminal_do(|out| {
        out.queue(cursor::MoveTo(x1 - 1 + x as u16 - 1, y1 - 1 + y as u16 - 1))?;
        Ok(())
    });
}

fn cursor_position() -> (u16, u16) {
    if mode() != Mode::Terminal {
        return (1, 1);
    }
    crate::io::flush_stdout();
    let (x1, y1, ..) = current_window();
    match cursor::position() {
        Ok((col, row)) => ((col + 2).saturating_sub(x1), (row + 2).saturating_sub(y1)),
        Err(_) => (1, 1),
    }
}

/// `WhereX`: the cursor column, relative to the window.
pub fn where_x() -> u8 {
    cursor_position().0 as u8
}

/// `WhereY`: the cursor row, relative to the window.
pub fn where_y() -> u8 {
    cursor_position().1 as u8
}

/// `Window(x1, y1, x2, y2)`: restrict `ClrScr` and `GotoXY` to a rectangle.
pub fn window(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>) {
    let (x1, y1, x2, y2) = (x1.into(), y1.into(), x2.into(), y2.into());
    let valid = 1 <= x1 && x1 <= x2 && x2 <= SCREEN_COLS as i32 && 1 <= y1 && y1 <= y2 && y2 <= SCREEN_ROWS as i32;
    if valid {
        CRT.with_borrow_mut(|crt| crt.window = (x1 as u16, y1 as u16, x2 as u16, y2 as u16));
        goto_xy(1, 1);
    }
}

/// `WindMin`: the window's top-left corner, 0-based, packed as TP does:
/// column in the low byte, row in the high byte.
pub fn wind_min() -> u16 {
    let (x1, y1, ..) = current_window();
    (x1 - 1) | ((y1 - 1) << 8)
}

/// `WindMax`: the window's bottom-right corner, packed like `WindMin`.
pub fn wind_max() -> u16 {
    let (_, _, x2, y2) = current_window();
    (x2 - 1) | ((y2 - 1) << 8)
}

/// `InsLine`: insert a blank line at the cursor.
pub fn ins_line() {
    terminal_do(|out| {
        out.queue(terminal::ScrollDown(1))?;
        Ok(())
    });
}

/// `DelLine`: delete the line at the cursor.
pub fn del_line() {
    terminal_do(|out| {
        out.queue(terminal::ScrollUp(1))?;
        Ok(())
    });
}

/// `TextMode(mode)`: TP switched video modes; we reset colours and clear.
pub fn text_mode(_mode: impl Into<i32>) {
    CRT.with_borrow_mut(|crt| crt.window = (1, 1, SCREEN_COLS, SCREEN_ROWS));
    norm_video();
    clr_scr();
}

/// `Delay(ms)`: pause. `PASCRUST_FAST=1` skips delays (handy in tests).
pub fn delay(ms: impl Into<i32>) {
    let ms = ms.into().max(0) as u64;
    if std::env::var_os("PASCRUST_FAST").is_none() {
        crate::io::flush_stdout();
        std::thread::sleep(Duration::from_millis(ms));
    }
}

/// `Sound(hz)`: the PC speaker is gone, so ring the terminal bell instead.
pub fn sound(_hz: impl Into<i32>) {
    terminal_do(|out| write!(out, "\x07"));
}

/// `NoSound`: nothing to stop.
pub fn no_sound() {}

/// `KeyPressed`: is a key waiting? In plain mode (scripted input) there's
/// always a key "waiting", the next character of input.
pub fn key_pressed() -> bool {
    if CRT.with_borrow(|crt| crt.pending_scan_code.is_some()) {
        return true;
    }
    if mode() == Mode::Plain {
        return true;
    }
    crate::io::flush_stdout();
    // Throw away events that aren't key presses (resizes, key releases).
    while crossterm::event::poll(Duration::ZERO).unwrap_or(false) {
        match crossterm::event::read() {
            Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                // Put it back by remembering it; `read_key` checks first.
                PEEKED.with_borrow_mut(|peeked| *peeked = Some(key));
                return true;
            }
            Ok(_) => continue,
            Err(_) => return false,
        }
    }
    PEEKED.with_borrow(|peeked| peeked.is_some())
}

thread_local! {
    /// A key that `KeyPressed` read to find out it was there.
    static PEEKED: RefCell<Option<KeyEvent>> = const { RefCell::new(None) };
}

/// `ReadKey`: wait for a key and return its character, without echoing it.
/// Keys with no character (arrows, function keys) come back as `#0`
/// followed, on the next call, by the key's PC scan code.
pub fn read_key() -> u8 {
    if let Some(code) = CRT.with_borrow_mut(|crt| crt.pending_scan_code.take()) {
        return code;
    }
    if mode() == Mode::Plain {
        return crate::io::read_key_plain();
    }
    crate::io::flush_stdout();
    loop {
        let key = match PEEKED.with_borrow_mut(|peeked| peeked.take()) {
            Some(key) => key,
            None => match crossterm::event::read() {
                Ok(Event::Key(key)) if key.kind != KeyEventKind::Release => key,
                Ok(_) => continue,
                Err(_) => return 26,
            },
        };
        let (char_code, scan_code) = translate_key(key);
        if let Some(scan) = scan_code {
            CRT.with_borrow_mut(|crt| crt.pending_scan_code = Some(scan));
        }
        return char_code;
    }
}

/// A key as TP's `ReadKey` would report it: a character code, plus a scan
/// code for extended keys (where the character code is 0).
fn translate_key(key: KeyEvent) -> (u8, Option<u8>) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        // Ctrl-C has to stop the program: raw mode turned off the terminal's
        // own handling of it. (TP's equivalent was Ctrl-Break.)
        KeyCode::Char('c') if ctrl => {
            restore();
            std::process::exit(130);
        }
        KeyCode::Char(c) if ctrl && c.is_ascii_alphabetic() => (c.to_ascii_uppercase() as u8 - b'@', None),
        KeyCode::Char(c) => (crate::cp437::from_char(c), None),
        KeyCode::Enter => (13, None),
        KeyCode::Esc => (27, None),
        KeyCode::Backspace => (8, None),
        KeyCode::Tab => (9, None),
        KeyCode::Up => (0, Some(72)),
        KeyCode::Down => (0, Some(80)),
        KeyCode::Left => (0, Some(75)),
        KeyCode::Right => (0, Some(77)),
        KeyCode::Home => (0, Some(71)),
        KeyCode::End => (0, Some(79)),
        KeyCode::PageUp => (0, Some(73)),
        KeyCode::PageDown => (0, Some(81)),
        KeyCode::Insert => (0, Some(82)),
        KeyCode::Delete => (0, Some(83)),
        KeyCode::F(n @ 1..=10) => (0, Some(58 + n)),
        _ => (0, Some(0)),
    }
}

/// Read a line with simple editing (typing, Backspace, Enter), echoing as
/// we go: what `Readln` does while the terminal is in raw mode.
pub(crate) fn read_line_raw() -> Option<Vec<u8>> {
    let mut line = Vec::new();
    loop {
        match read_key() {
            13 => {
                crate::io::write_stdout(b"\n");
                return Some(line);
            }
            8 => {
                if line.pop().is_some() {
                    crate::io::write_stdout(b"\x08 \x08");
                }
            }
            26 if line.is_empty() => return None,
            0 => {
                // An extended key: swallow its scan code.
                read_key();
            }
            c if c >= 32 => {
                line.push(c);
                crate::io::write_stdout(&[c]);
            }
            _ => {}
        }
        crate::io::flush_stdout();
    }
}
