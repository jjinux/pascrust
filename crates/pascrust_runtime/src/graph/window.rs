//! Showing the graphics screen in a window, and keyboard input from it.
//!
//! Most GUI toolkits, and macOS in particular, insist that windows are
//! created and serviced on the process's *main* thread. The Pascal program
//! runs on a worker thread (see `run.rs`), so the two cooperate:
//!
//! - The program thread draws into the shared [`SCREEN`](super::SCREEN) and
//!   asks for the window to open or close by setting [`STATE`].
//! - The main thread runs [`main_loop`]: it opens and closes the window when
//!   asked, copies the screen into it about 60 times a second, and queues
//!   the keys typed into it for `ReadKey`.
//!
//! With `PASCRUST_GRAPH=headless`, or when no window can be opened (no
//! display, as on a CI server), drawing still works but nothing is shown,
//! and `ReadKey` reads standard input as usual.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use minifb::{InputCallback, Key, KeyRepeat, Window, WindowOptions};

use super::screen::{HEIGHT, WIDTH};

// The window's state. An atomic integer is the simplest thing two threads
// can share without a lock: every read and write is indivisible.
const CLOSED: u8 = 0;
const OPEN_REQUESTED: u8 = 1;
const OPEN: u8 = 2;
const CLOSE_REQUESTED: u8 = 3;
/// No window, ever (headless, or opening one failed).
const HEADLESS: u8 = 4;

static STATE: AtomicU8 = AtomicU8::new(CLOSED);

/// Keys typed into the window, as `ReadKey` returns them (a special key is
/// two entries: 0, then its scan code). `Condvar` lets `ReadKey` sleep until
/// the main thread pushes a key, instead of spinning.
static KEYS: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
static KEY_PUSHED: Condvar = Condvar::new();

// These three are how `Crt`'s `ReadKey` and `KeyPressed` read the window's
// keys; without the `crt` feature nothing calls them.

/// True while the window is showing.
#[cfg(feature = "crt")]
pub(crate) fn is_open() -> bool {
    STATE.load(Ordering::SeqCst) == OPEN
}

/// Ask the main thread for the window, and wait until it's there (or has
/// failed to appear).
pub(super) fn open() {
    if std::env::var("PASCRUST_GRAPH").is_ok_and(|v| v == "headless") {
        STATE.store(HEADLESS, Ordering::SeqCst);
    }
    // `compare_exchange` changes the state only if it is what we expect, so
    // a headless state stays headless.
    if STATE.compare_exchange(CLOSED, OPEN_REQUESTED, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        wait_while(OPEN_REQUESTED);
    }
}

/// Ask the main thread to close the window, and wait until it has.
pub(super) fn close() {
    if STATE.compare_exchange(OPEN, CLOSE_REQUESTED, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
        wait_while(CLOSE_REQUESTED);
    }
    KEYS.lock().unwrap().clear();
}

/// Wait for the main thread to move on from `state`. Polling is simpler
/// than a second `Condvar`, and it only happens when the window opens or
/// closes. (If the program isn't started by `run`, nothing services the
/// window and this would wait forever; generated programs always use `run`.)
fn wait_while(state: u8) {
    while STATE.load(Ordering::SeqCst) == state {
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// `KeyPressed` while the window is open.
#[cfg(feature = "crt")]
pub(crate) fn key_pressed() -> bool {
    !KEYS.lock().unwrap().is_empty()
}

/// `ReadKey` while the window is open: wait for a key typed into it.
#[cfg(feature = "crt")]
pub(crate) fn read_key() -> u8 {
    let mut keys = KEYS.lock().unwrap();
    loop {
        if let Some(key) = keys.pop_front() {
            return key;
        }
        // `wait` releases the lock while sleeping and takes it back when
        // woken, so the main thread can push keys meanwhile.
        keys = KEY_PUSHED.wait(keys).unwrap();
    }
}

fn push_keys(new: &[u8]) {
    if !new.is_empty() {
        KEYS.lock().unwrap().extend(new);
        KEY_PUSHED.notify_all();
    }
}

/// Receives typed characters (with Shift and so on applied) from minifb.
struct Typed;

impl InputCallback for Typed {
    fn add_char(&mut self, c: u32) {
        // Control keys (Enter, Esc, ...) are handled in `special_keys`.
        if (32..127).contains(&c) {
            push_keys(&[c as u8]);
        }
    }
}

/// The keys that aren't characters, as `ReadKey` reports them.
fn special_keys(window: &Window) -> Vec<u8> {
    let mut out = Vec::new();
    for key in window.get_keys_pressed(KeyRepeat::Yes) {
        match key {
            Key::Enter | Key::NumPadEnter => out.push(13),
            Key::Escape => out.push(27),
            Key::Backspace => out.push(8),
            Key::Tab => out.push(9),
            other => {
                // Extended keys: #0 followed by the PC keyboard scan code.
                let scan = match other {
                    Key::F1 => 59,
                    Key::F2 => 60,
                    Key::F3 => 61,
                    Key::F4 => 62,
                    Key::F5 => 63,
                    Key::F6 => 64,
                    Key::F7 => 65,
                    Key::F8 => 66,
                    Key::F9 => 67,
                    Key::F10 => 68,
                    Key::Home => 71,
                    Key::Up => 72,
                    Key::PageUp => 73,
                    Key::Left => 75,
                    Key::Right => 77,
                    Key::End => 79,
                    Key::Down => 80,
                    Key::PageDown => 81,
                    Key::Insert => 82,
                    Key::Delete => 83,
                    _ => continue,
                };
                out.extend([0, scan]);
            }
        }
    }
    out
}

/// The main thread's loop while the program runs: see the module docs.
pub(crate) fn main_loop(worker: &JoinHandle<()>) {
    let mut window: Option<Window> = None;
    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    while !worker.is_finished() {
        match STATE.load(Ordering::SeqCst) {
            OPEN_REQUESTED => {
                let options = WindowOptions { resize: true, ..WindowOptions::default() };
                match Window::new("PascRust", WIDTH, HEIGHT, options) {
                    Ok(mut w) => {
                        w.set_target_fps(60);
                        w.set_input_callback(Box::new(Typed));
                        window = Some(w);
                        STATE.store(OPEN, Ordering::SeqCst);
                    }
                    Err(e) => {
                        eprintln!("[PascRust] cannot open a graphics window ({e}); drawing without one");
                        STATE.store(HEADLESS, Ordering::SeqCst);
                    }
                }
            }
            CLOSE_REQUESTED => {
                window = None; // dropping the `Window` closes it
                STATE.store(CLOSED, Ordering::SeqCst);
            }
            _ => {}
        }
        match window.as_mut() {
            Some(w) => {
                if !w.is_open() {
                    // Closing the window ends the program, like Ctrl-Break.
                    super::save_if_requested();
                    crate::io::shutdown();
                    std::process::exit(0);
                }
                if let Some(screen) = super::SCREEN.lock().unwrap().as_ref() {
                    screen.render(&mut buffer);
                }
                // This also processes the window's events, and sleeps to
                // keep to the target frame rate.
                let _ = w.update_with_buffer(&buffer, WIDTH, HEIGHT);
                push_keys(&special_keys(w));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}
