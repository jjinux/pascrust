//! Turbo Pascal's `Graph` unit (the Borland Graphics Interface, BGI).
//!
//! The BGI drew on a VGA card in 640x480, 16-colour mode. Here the screen is
//! a framebuffer in memory, drawn by our own code (see `screen.rs`) and
//! shown in a window (see `window.rs`). Every program gets the VGA driver in
//! `VGAHi` mode, whatever it asks `InitGraph` for.
//!
//! Supported: `InitGraph`/`CloseGraph`/`SetGraphMode`/`RestoreCrtMode`,
//! colours and palettes, pixels, lines (styles, thickness, XOR mode),
//! rectangles, bars, circles, arcs, ellipses, pie slices, flood fill,
//! viewports, polygons, the `Get*Settings` records, and text in the 8x8
//! default font (other fonts fall back to it, scaled), palettes, and two
//! pages. Not yet: `GetImage`/`PutImage`.
//!
//! Environment variables:
//! - `PASCRUST_GRAPH=headless`: draw without a window (for tests).
//! - `PASCRUST_GRAPH_SAVE=file.bmp`: save the screen to a BMP file at
//!   `CloseGraph` (and at the end of the program if it's still open).

mod font;
mod screen;
pub(crate) mod window;

use std::sync::Mutex;
use std::sync::atomic::{AtomicI16, Ordering};

use screen::{HEIGHT, Screen, WIDTH, WriteMode, image_bytes};

use crate::array::PArray;
use crate::bytes::ByteView;
use crate::string::PString;

/// `FillPatternType = array[1..8] of Byte`: eight rows of eight pixels, bit 7
/// leftmost, for `SetFillPattern`.
pub type FillPatternType = PArray<u8, 1, 8>;

/// `PointType`, for `DrawPoly` and `FillPoly`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PointType {
    pub x: i16,
    pub y: i16,
}

/// `ArcCoordsType`, for `GetArcCoords`: the last arc's centre and ends.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArcCoordsType {
    pub x: i16,
    pub y: i16,
    pub x_start: i16,
    pub y_start: i16,
    pub x_end: i16,
    pub y_end: i16,
}

/// `LineSettingsType`, for `GetLineSettings`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LineSettingsType {
    pub line_style: u16,
    pub pattern: u16,
    pub thickness: u16,
}

/// `FillSettingsType`, for `GetFillSettings`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FillSettingsType {
    pub pattern: u16,
    pub color: u16,
}

/// `TextSettingsType`, for `GetTextSettings`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextSettingsType {
    pub font: u16,
    pub direction: u16,
    pub char_size: u16,
    pub horiz: u16,
    pub vert: u16,
}

/// `ViewPortType`, for `GetViewSettings`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ViewPortType {
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
    pub clip: bool,
}

/// The screen, while graphics mode is on. A `Mutex` because the window
/// (main thread) reads it while the program (worker thread) draws.
static SCREEN: Mutex<Option<Screen>> = Mutex::new(None);

/// Run `f` on the screen. Turbo Pascal ignores drawing before `InitGraph`
/// (only `GraphResult` complains), and so do we: `f` isn't called, and we
/// return the default value (0) instead.
fn with<R: Default>(f: impl FnOnce(&mut Screen) -> R) -> R {
    // `as_mut()` turns `&mut Option<Screen>` into `Option<&mut Screen>`.
    SCREEN.lock().unwrap().as_mut().map(f).unwrap_or_default()
}

/// Arguments arrive as any integer type; `impl Into<i32>` accepts them all.
fn i(n: impl Into<i32>) -> i32 {
    n.into()
}

fn color(n: impl Into<i32>) -> u8 {
    (n.into() & 15) as u8
}

// Drivers and modes.
pub const DETECT: u8 = 0;
pub const CGA: u8 = 1;
pub const MCGA: u8 = 2;
pub const EGA: u8 = 3;
pub const EGA64: u8 = 4;
pub const EGA_MONO: u8 = 5;
pub const IBM8514: u8 = 6;
pub const HERC_MONO: u8 = 7;
pub const ATT400: u8 = 8;
pub const VGA: u8 = 9;
pub const PC3270: u8 = 10;
pub const VGA_LO: u8 = 0;
pub const VGA_MED: u8 = 1;
pub const VGA_HI: u8 = 2;
pub const GR_OK: u8 = 0;
pub const GR_ERROR: i8 = -11;
// Line styles and widths.
pub const SOLID_LN: u8 = 0;
pub const DOTTED_LN: u8 = 1;
pub const CENTER_LN: u8 = 2;
pub const DASHED_LN: u8 = 3;
pub const USER_BIT_LN: u8 = 4;
pub const NORM_WIDTH: u8 = 1;
pub const THICK_WIDTH: u8 = 3;
// Fill patterns.
pub const EMPTY_FILL: u8 = 0;
pub const SOLID_FILL: u8 = 1;
pub const LINE_FILL: u8 = 2;
pub const LT_SLASH_FILL: u8 = 3;
pub const SLASH_FILL: u8 = 4;
pub const BK_SLASH_FILL: u8 = 5;
pub const LT_BK_SLASH_FILL: u8 = 6;
pub const HATCH_FILL: u8 = 7;
pub const X_HATCH_FILL: u8 = 8;
pub const INTERLEAVE_FILL: u8 = 9;
pub const WIDE_DOT_FILL: u8 = 10;
pub const CLOSE_DOT_FILL: u8 = 11;
pub const USER_FILL: u8 = 12;
// Text.
pub const LEFT_TEXT: u8 = 0;
pub const CENTER_TEXT: u8 = 1;
pub const RIGHT_TEXT: u8 = 2;
pub const BOTTOM_TEXT: u8 = 0;
pub const TOP_TEXT: u8 = 2;
pub const DEFAULT_FONT: u8 = 0;
pub const TRIPLEX_FONT: u8 = 1;
pub const SMALL_FONT: u8 = 2;
pub const SANS_SERIF_FONT: u8 = 3;
pub const GOTHIC_FONT: u8 = 4;
pub const HORIZ_DIR: u8 = 0;
pub const VERT_DIR: u8 = 1;
// Write modes.
pub const COPY_PUT: u8 = 0;
pub const XOR_PUT: u8 = 1;
// `PutImage`'s ways to combine an image with the screen (`CopyPut` and
// `XORPut` too).
pub const NORMAL_PUT: u8 = 0;
pub const OR_PUT: u8 = 2;
pub const AND_PUT: u8 = 3;
pub const NOT_PUT: u8 = 4;
pub const MAX_COLORS: u8 = 15;
pub const USER_CHAR_SIZE: u8 = 0;
pub const CLIP_ON: bool = true;
pub const CLIP_OFF: bool = false;
pub const TOP_ON: bool = true;
pub const TOP_OFF: bool = false;

// Starting and stopping.

/// `InitGraph(Driver, Mode, PathToDriver)`: switch to graphics mode. The
/// path to the `.BGI` driver files doesn't matter: the "driver" is built in.
pub fn init_graph(driver: &mut i16, mode: &mut i16, _path: impl Into<PString>) {
    *driver = VGA.into();
    *mode = VGA_HI.into();
    *SCREEN.lock().unwrap() = Some(Screen::new());
    window::open();
}

/// `DetectGraph(Driver, Mode)`: what `InitGraph` would pick.
pub fn detect_graph(driver: &mut i16, mode: &mut i16) {
    *driver = VGA.into();
    *mode = VGA_HI.into();
}

/// `CloseGraph`: back to text mode.
pub fn close_graph() {
    save_if_requested();
    window::close();
    *SCREEN.lock().unwrap() = None;
}

/// `RestoreCrtMode`: back to text mode for a while, keeping the graphics
/// state for `SetGraphMode`.
pub fn restore_crt_mode() {
    save_if_requested();
    window::close();
}

/// `SetGraphMode(Mode)`: back to graphics mode, with a cleared screen and
/// default settings.
pub fn set_graph_mode(_mode: impl Into<i32>) {
    *SCREEN.lock().unwrap() = Some(Screen::new());
    window::open();
}

pub fn get_graph_mode() -> i16 {
    VGA_HI.into()
}

/// The error the last failing Graph routine set, for `GraphResult`: only
/// `ImageSize` fails (`grError`, -11, for 64K or more). An atomic is a
/// number that can be shared between threads without a `Mutex`.
static RESULT: AtomicI16 = AtomicI16::new(0);

/// `GraphResult`: the last error, which reading resets to `grOk`.
pub fn graph_result() -> i16 {
    RESULT.swap(0, Ordering::Relaxed)
}

pub fn graph_error_msg(code: impl Into<i32>) -> PString {
    PString::from(match code.into() {
        0 => "No error",
        -1 => "(BGI) graphics not installed",
        -2 => "Graphics hardware not detected",
        _ => "Graphics error",
    })
}

pub fn get_max_x() -> i16 {
    WIDTH as i16 - 1
}

pub fn get_max_y() -> i16 {
    HEIGHT as i16 - 1
}

pub fn get_max_color() -> u16 {
    15
}

/// `GetAspectRatio(Xasp, Yasp)`: VGA pixels are square, unless
/// `SetAspectRatio` said otherwise.
pub fn get_aspect_ratio(x: &mut u16, y: &mut u16) {
    (*x, *y) = with(|s| s.aspect);
}

/// `SetAspectRatio(Xasp, Yasp)`: remembered for `GetAspectRatio`. (Circles
/// stay round: the screen's pixels are square whatever it says.)
pub fn set_aspect_ratio(x: impl Into<i32>, y: impl Into<i32>) {
    let aspect = (i(x) as u16, i(y) as u16);
    with(|s| s.aspect = aspect);
}

/// `GraphDefaults`: everything back as `InitGraph` set it (colours,
/// palette, styles, text, viewport, current position), but not the
/// picture.
pub fn graph_defaults() {
    with(|s| {
        let pixels = std::mem::take(&mut s.pixels);
        let other_page = std::mem::take(&mut s.other_page);
        let pages = (s.active_page, s.visual_page);
        *s = Screen::new();
        (s.pixels, s.other_page) = (pixels, other_page);
        (s.active_page, s.visual_page) = pages;
    });
}

/// `GetDriverName`: the VGA driver's file name, as the BGI gave it.
pub fn get_driver_name() -> PString {
    PString::from("EGAVGA")
}

/// `GetModeName(ModeNumber)`: the VGA modes' names, as the BGI gave them.
pub fn get_mode_name(mode: impl Into<i32>) -> PString {
    let name = match i(mode) {
        0 => "640 x 200 VGA",
        1 => "640 x 350 VGA",
        2 => "640 x 480 VGA",
        _ => "",
    };
    PString::from(name)
}

/// `GetMaxMode`: `VGAHi`, the highest mode of the (only) driver.
pub fn get_max_mode() -> i16 {
    VGA_HI.into()
}

/// `GetModeRange(Driver, LoMode, HiMode)`: the modes each BGI driver had;
/// -1 for a driver number that doesn't exist.
pub fn get_mode_range(driver: impl Into<i32>, lo: &mut i16, hi: &mut i16) {
    (*lo, *hi) = match i(driver) {
        1 | 8 => (0, 4),     // CGA, AT&T 400
        2 => (0, 5),         // MCGA
        3 | 4 | 6 => (0, 1), // EGA, EGA64, IBM 8514
        5 => (3, 3),         // EGA monochrome
        7 | 10 => (0, 0),    // Hercules, PC 3270
        9 => (0, 2),         // VGA
        _ => (-1, -1),
    };
}

/// `InstallUserFont(FileName)`: there are no `.CHR` files to load, but the
/// program gets a new font number (drawn in the default font, as the
/// built-in stroked fonts are).
pub fn install_user_font(_file_name: impl Into<PString>) -> i16 {
    thread_local! {
        static NEXT_FONT: std::cell::Cell<i16> = const { std::cell::Cell::new(11) };
    }
    NEXT_FONT.with(|next| {
        let font = next.get();
        next.set(font + 1);
        font
    })
}

/// `SetGraphBufSize(BufSize)`: the BGI's buffer for flood fills. Ours needs
/// no setting.
pub fn set_graph_buf_size(_size: impl Into<i32>) {}

/// `SetUserCharSize(MultX, DivX, MultY, DivY)`: scales the stroked fonts.
/// Text here is the 8x8 font scaled by whole numbers, so it uses the
/// vertical factor, rounded.
pub fn set_user_char_size(
    _mult_x: impl Into<i32>,
    _div_x: impl Into<i32>,
    mult_y: impl Into<i32>,
    div_y: impl Into<i32>,
) {
    let scale = (i(mult_y) / i(div_y).max(1)).clamp(1, 10);
    with(|s| s.text_scale = scale);
}

// Colours.

pub fn set_color(c: impl Into<i32>) {
    let c = color(c);
    with(|s| s.color = c);
}

pub fn get_color() -> u16 {
    with(|s| u16::from(s.color))
}

/// `SetBkColor(Color)`: colour 0 is the background, so this changes what
/// colour 0 looks like, everywhere on the screen at once.
pub fn set_bk_color(c: impl Into<i32>) {
    let c = color(c);
    with(|s| {
        s.bk_color = c;
        s.palette[0] = screen::DEFAULT_PALETTE[usize::from(c)];
    });
}

pub fn get_bk_color() -> u16 {
    with(|s| u16::from(s.bk_color))
}

/// `SetPalette(ColorNum, Color)`: colour number 0..15 shows EGA colour
/// 0..63. Anything else was an error in the BGI (`grError`), which changed
/// nothing; so it changes nothing here.
pub fn set_palette(n: impl Into<i32>, ega: impl Into<i32>) {
    let (n, ega) = (i(n), i(ega));
    if (0..16).contains(&n) && (0..64).contains(&ega) {
        with(|s| s.palette[n as usize] = ega as u8);
    }
}

/// `PaletteType`: the number of colours, and the EGA colour of each.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PaletteType {
    pub size: u8,
    pub colors: PArray<i8, 0, 16>,
}

/// `GetPalette(Palette)`: the 16 colour numbers' EGA colours.
pub fn get_palette(palette: &mut PaletteType) {
    let colors = with(|s| s.palette);
    fill_palette(palette, colors);
}

/// `GetDefaultPalette(Palette)`: the palette as `InitGraph` set it.
pub fn get_default_palette(palette: &mut PaletteType) {
    fill_palette(palette, screen::DEFAULT_PALETTE);
}

fn fill_palette(palette: &mut PaletteType, colors: [u8; 16]) {
    palette.size = 16;
    for (n, ega) in colors.into_iter().enumerate() {
        palette.colors[n as i16] = ega as i8;
    }
}

/// `SetAllPalette(Palette)`: set every colour number at once. An entry of
/// -1 leaves that colour as it was.
pub fn set_all_palette(palette: &mut PaletteType) {
    let count = usize::from(palette.size).min(16);
    let colors: Vec<i8> = palette.colors.iter().take(count).copied().collect();
    // A colour beyond the EGA's 64 was an error, and nothing changed.
    if colors.iter().any(|&ega| !(-1..=63).contains(&ega)) {
        return;
    }
    with(|s| {
        for (n, &ega) in colors.iter().enumerate() {
            if ega >= 0 {
                s.palette[n] = (ega & 63) as u8;
            }
        }
    });
}

/// `GetPaletteSize`: 16 colours.
pub fn get_palette_size() -> i16 {
    16
}

/// `SetRGBPalette(ColorNum, Red, Green, Blue)`: what one of the VGA's 64
/// colour registers shows, each component 0 to 63 (the low 6 bits). In 16-
/// colour mode, colour number `n` shows register `palette[n]`, so
/// `SetRGBPalette(DefaultPalette[Red], ...)` recolours red.
pub fn set_rgb_palette(n: impl Into<i32>, red: impl Into<i32>, green: impl Into<i32>, blue: impl Into<i32>) {
    let rgb = screen::dac_rgb(i(red), i(green), i(blue));
    let n = (i(n) & 63) as usize;
    with(|s| s.dac[n] = rgb);
}

// Pixels and lines.

pub fn put_pixel(x: impl Into<i32>, y: impl Into<i32>, c: impl Into<i32>) {
    let (x, y, c) = (i(x), i(y), color(c));
    with(|s| s.put_pixel(x, y, c));
}

pub fn get_pixel(x: impl Into<i32>, y: impl Into<i32>) -> u16 {
    let (x, y) = (i(x), i(y));
    with(|s| u16::from(s.get_pixel(x, y)))
}

pub fn line(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>) {
    let (x1, y1, x2, y2) = (i(x1), i(y1), i(x2), i(y2));
    with(|s| s.line(x1, y1, x2, y2));
}

pub fn move_to(x: impl Into<i32>, y: impl Into<i32>) {
    let (x, y) = (i(x), i(y));
    with(|s| s.cp = (x, y));
}

pub fn move_rel(dx: impl Into<i32>, dy: impl Into<i32>) {
    let (dx, dy) = (i(dx), i(dy));
    with(|s| s.cp = (s.cp.0 + dx, s.cp.1 + dy));
}

pub fn line_to(x: impl Into<i32>, y: impl Into<i32>) {
    let (x, y) = (i(x), i(y));
    with(|s| {
        s.line(s.cp.0, s.cp.1, x, y);
        s.cp = (x, y);
    });
}

pub fn line_rel(dx: impl Into<i32>, dy: impl Into<i32>) {
    let (dx, dy) = (i(dx), i(dy));
    with(|s| {
        let (x, y) = (s.cp.0 + dx, s.cp.1 + dy);
        s.line(s.cp.0, s.cp.1, x, y);
        s.cp = (x, y);
    });
}

pub fn get_x() -> i16 {
    with(|s| s.cp.0 as i16)
}

pub fn get_y() -> i16 {
    with(|s| s.cp.1 as i16)
}

/// `SetLineStyle(LineStyle, Pattern, Thickness)`. `Pattern` is used only
/// with `UserBitLn`.
pub fn set_line_style(style: impl Into<i32>, pattern: impl Into<i32>, thickness: impl Into<i32>) {
    let (style, pattern, thickness) = (i(style), i(pattern) as u16, i(thickness));
    with(|s| s.set_line_style(style, pattern, thickness));
}

/// `SetWriteMode(CopyPut or XorPut)`, for lines and rectangles.
pub fn set_write_mode(mode: impl Into<i32>) {
    let mode = if i(mode) == i32::from(XOR_PUT) { WriteMode::Xor } else { WriteMode::Copy };
    with(|s| s.write_mode = mode);
}

// Shapes.

pub fn rectangle(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>) {
    let (x1, y1, x2, y2) = (i(x1), i(y1), i(x2), i(y2));
    with(|s| s.rectangle(x1, y1, x2, y2));
}

/// `SetFillStyle(Pattern, Color)`. A pattern number beyond `CloseDotFill`
/// (including `UserFill`, which only `SetFillPattern` sets) was an error,
/// and nothing changed.
pub fn set_fill_style(pattern: impl Into<i32>, c: impl Into<i32>) {
    let (pattern, c) = (i(pattern), color(c));
    if (0..i32::from(USER_FILL)).contains(&pattern) {
        with(|s| s.set_fill_style(pattern, c));
    }
}

/// `SetFillPattern(Pattern, Color)`: fill with a pattern of your own.
pub fn set_fill_pattern(pattern: FillPatternType, c: impl Into<i32>) {
    let c = color(c);
    let rows: [u8; 8] = std::array::from_fn(|row| pattern[row as i16 + 1]);
    with(|s| {
        s.fill_pattern = rows;
        s.fill_color = c;
        s.fill_style = i32::from(USER_FILL);
    });
}

/// `GetFillPattern(Pattern)`: the current fill pattern.
pub fn get_fill_pattern(pattern: &mut FillPatternType) {
    let rows = with(|s| s.fill_pattern);
    for (row, bits) in rows.into_iter().enumerate() {
        pattern[row as i16 + 1] = bits;
    }
}

/// `Bar(X1, Y1, X2, Y2)`: a filled rectangle with no outline.
pub fn bar(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>) {
    let (x1, y1, x2, y2) = (i(x1), i(y1), i(x2), i(y2));
    with(|s| s.bar(x1, y1, x2, y2));
}

/// `Bar3D(X1, Y1, X2, Y2, Depth, Top)`: a bar with an outline and a 3-D
/// side `Depth` pixels deep (and a top, if `Top`).
pub fn bar3d(
    x1: impl Into<i32>,
    y1: impl Into<i32>,
    x2: impl Into<i32>,
    y2: impl Into<i32>,
    depth: impl Into<i32>,
    top: bool,
) {
    let (x1, y1, x2, y2, d) = (i(x1), i(y1), i(x2), i(y2), i(depth));
    with(|s| {
        s.bar(x1, y1, x2, y2);
        s.rectangle(x1, y1, x2, y2);
        if d != 0 {
            let dy = d * 3 / 4; // the BGI's 3-D sides slope at 3/4
            s.line(x2, y2, x2 + d, y2 - dy);
            s.line(x2 + d, y2 - dy, x2 + d, y1 - dy);
            if top {
                s.line(x2 + d, y1 - dy, x1 + d, y1 - dy);
                s.line(x1 + d, y1 - dy, x1, y1);
                s.line(x2, y1, x2 + d, y1 - dy);
            }
        }
    });
}

pub fn circle(x: impl Into<i32>, y: impl Into<i32>, r: impl Into<i32>) {
    let (x, y, r) = (i(x), i(y), i(r));
    with(|s| s.ellipse(x, y, 0, 360, r, r));
}

/// `Arc(X, Y, StAngle, EndAngle, Radius)`: angles in degrees,
/// counterclockwise from 3 o'clock.
pub fn arc(x: impl Into<i32>, y: impl Into<i32>, start: impl Into<i32>, end: impl Into<i32>, r: impl Into<i32>) {
    let (x, y, start, end, r) = (i(x), i(y), i(start), i(end), i(r));
    with(|s| s.ellipse(x, y, start, end, r, r));
}

pub fn ellipse(
    x: impl Into<i32>,
    y: impl Into<i32>,
    start: impl Into<i32>,
    end: impl Into<i32>,
    xr: impl Into<i32>,
    yr: impl Into<i32>,
) {
    let (x, y, start, end, xr, yr) = (i(x), i(y), i(start), i(end), i(xr), i(yr));
    with(|s| s.ellipse(x, y, start, end, xr, yr));
}

/// `FillEllipse(X, Y, XRadius, YRadius)`: filled with the fill style and
/// outlined in the current colour.
pub fn fill_ellipse(x: impl Into<i32>, y: impl Into<i32>, xr: impl Into<i32>, yr: impl Into<i32>) {
    let (x, y, xr, yr) = (i(x), i(y), i(xr), i(yr));
    with(|s| {
        s.fill_ellipse_inside(x, y, xr, yr);
        s.ellipse(x, y, 0, 360, xr, yr);
    });
}

pub fn pie_slice(x: impl Into<i32>, y: impl Into<i32>, start: impl Into<i32>, end: impl Into<i32>, r: impl Into<i32>) {
    let (x, y, start, end, r) = (i(x), i(y), i(start), i(end), i(r));
    with(|s| s.sector(x, y, start, end, r, r));
}

pub fn sector(
    x: impl Into<i32>,
    y: impl Into<i32>,
    start: impl Into<i32>,
    end: impl Into<i32>,
    xr: impl Into<i32>,
    yr: impl Into<i32>,
) {
    let (x, y, start, end, xr, yr) = (i(x), i(y), i(start), i(end), i(xr), i(yr));
    with(|s| s.sector(x, y, start, end, xr, yr));
}

/// `FloodFill(X, Y, Border)`: fill outwards from `(X, Y)` up to pixels of
/// colour `Border`.
pub fn flood_fill(x: impl Into<i32>, y: impl Into<i32>, border: impl Into<i32>) {
    let (x, y, border) = (i(x), i(y), color(border));
    with(|s| s.flood_fill(x, y, border));
}

// Polygons.

/// The first `count` points, as `(x, y)`.
fn points(count: impl Into<i32>, points: &[PointType]) -> Vec<(i32, i32)> {
    let count = usize::try_from(count.into()).unwrap_or(0);
    points.iter().take(count).map(|p| (i32::from(p.x), i32::from(p.y))).collect()
}

/// `DrawPoly(NumPoints, PolyPoints)`: lines from point to point, in the
/// current line style. It isn't closed unless the last point repeats the
/// first. (TP's `PolyPoints` is an untyped `var`; here it's an array of
/// `PointType`.)
pub fn draw_poly(count: impl Into<i32>, poly_points: &[PointType]) {
    let points = points(count, poly_points);
    with(|s| {
        for pair in points.windows(2) {
            s.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1);
        }
    });
}

/// `FillPoly(NumPoints, PolyPoints)`: the polygon filled with the fill
/// style, and outlined, closed, in the current line style.
pub fn fill_poly(count: impl Into<i32>, poly_points: &[PointType]) {
    let points = points(count, poly_points);
    with(|s| {
        s.fill_polygon_inside(&points);
        for (i, &(x1, y1)) in points.iter().enumerate() {
            let (x2, y2) = points[(i + 1) % points.len()];
            s.line(x1, y1, x2, y2);
        }
    });
}

// What's been set: the `Get*` routines.

/// `GetArcCoords(ArcCoords)`: where the last `Arc`, `Ellipse`, `PieSlice`
/// or `Sector` was drawn.
pub fn get_arc_coords(coords: &mut ArcCoordsType) {
    let [x, y, x_start, y_start, x_end, y_end] = with(|s| s.arc).map(|n| n as i16);
    *coords = ArcCoordsType { x, y, x_start, y_start, x_end, y_end };
}

pub fn get_line_settings(settings: &mut LineSettingsType) {
    let ((style, pattern), thickness) = with(|s| (s.line_style, s.thickness));
    *settings = LineSettingsType { line_style: style as u16, pattern, thickness: thickness as u16 };
}

pub fn get_fill_settings(settings: &mut FillSettingsType) {
    let (pattern, color) = with(|s| (s.fill_style, s.fill_color));
    *settings = FillSettingsType { pattern: pattern as u16, color: u16::from(color) };
}

pub fn get_text_settings(settings: &mut TextSettingsType) {
    let ((font, size), vertical, (horiz, vert)) = with(|s| (s.font, s.vertical, s.justify));
    *settings = TextSettingsType {
        font: font as u16,
        direction: u16::from(vertical),
        char_size: size as u16,
        horiz: u16::from(horiz),
        vert: u16::from(vert),
    };
}

pub fn get_view_settings(view: &mut ViewPortType) {
    let ((x1, y1, x2, y2), clip) = with(|s| (s.view, s.clip));
    *view = ViewPortType { x1: x1 as i16, y1: y1 as i16, x2: x2 as i16, y2: y2 as i16, clip };
}

// Pages.

/// `SetActivePage(Page)`: draw on page 0 or 1 from now on. VGA had two
/// pages only in its 640x350 mode; here every program has two, for
/// flicker-free animation: draw on the hidden page, then show it.
pub fn set_active_page(page: impl Into<i32>) {
    let page = i(page) as usize;
    with(|s| s.set_active_page(page));
}

/// `SetVisualPage(Page)`: show page 0 or 1.
pub fn set_visual_page(page: impl Into<i32>) {
    let page = i(page) as usize;
    with(|s| {
        if page < 2 {
            s.visual_page = page;
        }
    });
}

// The screen and viewports.

// Images.

/// `ImageSize(X1, Y1, X2, Y2)`: the bytes `GetImage` needs. TP's buffers
/// were at most 64K: anything bigger gives 0 and `GraphResult` `grError`.
pub fn image_size(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>) -> u16 {
    let size = image_bytes((i(x2) - i(x1)).abs() + 1, (i(y2) - i(y1)).abs() + 1);
    u16::try_from(size).unwrap_or_else(|_| {
        RESULT.store(-11, Ordering::Relaxed);
        0
    })
}

/// `GetImage(X1, Y1, X2, Y2, BitMap)`: copy the rectangle into `BitMap`
/// (any variable, usually `p^` after `GetMem(p, ImageSize(...))`). A
/// buffer too small for it is runtime error 201, where TP overwrote
/// whatever came next.
pub fn get_image(
    x1: impl Into<i32>,
    y1: impl Into<i32>,
    x2: impl Into<i32>,
    y2: impl Into<i32>,
    bitmap: &mut dyn ByteView,
) {
    // The corners in either order.
    let (x1, y1, x2, y2) = (i(x1), i(y1), i(x2), i(y2));
    let (x1, x2) = (x1.min(x2), x1.max(x2));
    let (y1, y2) = (y1.min(y2), y1.max(y2));
    let image = with(|s| s.get_image(x1, y1, x2, y2));
    if image.is_empty() {
        return; // not in graphics mode
    }
    let mut bytes = bitmap.to_bytes();
    if image.len() > bytes.len() {
        crate::rte(201, format!("Range check error: GetImage of {} bytes into {}", image.len(), bytes.len()));
    }
    bytes[..image.len()].copy_from_slice(&image);
    bitmap.set_bytes(&bytes);
}

/// `PutImage(X, Y, BitMap, BitBlt)`: draw an image `GetImage` saved, with
/// `NormalPut`, `XORPut`, `OrPut`, `AndPut` or `NotPut`.
pub fn put_image(x: impl Into<i32>, y: impl Into<i32>, bitmap: &[u8], bit_blt: impl Into<i32>) {
    let (x, y, bit_blt) = (i(x), i(y), i(bit_blt));
    let header_ok = bitmap.len() >= 6;
    let needed = if header_ok {
        let word = |i: usize| i32::from(u16::from_le_bytes([bitmap[i], bitmap[i + 1]]));
        image_bytes(word(0) + 1, word(2) + 1)
    } else {
        6
    };
    if needed > bitmap.len() {
        crate::rte(201, format!("Range check error: PutImage of {needed} bytes from {}", bitmap.len()));
    }
    with(|s| s.put_image(x, y, bitmap, bit_blt));
}

pub fn clear_device() {
    with(|s| {
        s.pixels.fill(0);
        s.cp = (0, 0);
    });
}

/// `SetViewPort(X1, Y1, X2, Y2, Clip)`: from now on, coordinates are
/// relative to `(X1, Y1)`, and with `Clip` nothing is drawn outside.
pub fn set_view_port(x1: impl Into<i32>, y1: impl Into<i32>, x2: impl Into<i32>, y2: impl Into<i32>, clip: bool) {
    let view = (i(x1), i(y1), i(x2), i(y2));
    with(|s| {
        s.view = view;
        s.clip = clip;
        s.cp = (0, 0);
    });
}

pub fn clear_view_port() {
    with(Screen::clear_view_port);
}

// Text.

/// `SetTextJustify(Horiz, Vert)`: where `OutTextXY`'s point is relative to
/// the text.
pub fn set_text_justify(horiz: impl Into<i32>, vert: impl Into<i32>) {
    let (h, v) = (i(horiz) as u8, i(vert) as u8);
    with(|s| s.justify = (h, v));
}

/// `SetTextStyle(Font, Direction, CharSize)`. Only the 8x8 default font is
/// built in; the stroked fonts are drawn with it, at a similar size.
pub fn set_text_style(font: impl Into<i32>, direction: impl Into<i32>, size: impl Into<i32>) {
    let (font, vertical, size) = (i(font), i(direction) == i32::from(VERT_DIR), i(size).max(1));
    // A stroked font at size 4 is about as tall as the default font at 2.
    let scale = if font == i32::from(DEFAULT_FONT) { size } else { (size / 2).max(1) };
    with(|s| {
        s.vertical = vertical;
        s.text_scale = scale.min(10);
        s.font = (font, size);
    });
}

pub fn out_text_xy(x: impl Into<i32>, y: impl Into<i32>, text: impl Into<PString>) {
    let (x, y, text) = (i(x), i(y), text.into());
    with(|s| s.out_text_xy(x, y, text.as_bytes()));
}

/// `OutText(S)`: text at the current position, which moves past it if the
/// text is left-justified and horizontal.
pub fn out_text(text: impl Into<PString>) {
    let text = text.into();
    with(|s| {
        let (x, y) = s.cp;
        s.out_text_xy(x, y, text.as_bytes());
        if s.justify.0 == 0 && !s.vertical {
            s.cp.0 += s.text_size(text.as_bytes()).0;
        }
    });
}

pub fn text_width(text: impl Into<PString>) -> u16 {
    let text = text.into();
    with(|s| s.text_size(text.as_bytes()).0 as u16)
}

pub fn text_height(text: impl Into<PString>) -> u16 {
    let text = text.into();
    with(|s| s.text_size(text.as_bytes()).1 as u16)
}

// Saving the screen.

/// With `PASCRUST_GRAPH_SAVE=file.bmp`, write the screen to that file.
pub(crate) fn save_if_requested() {
    let Ok(path) = std::env::var("PASCRUST_GRAPH_SAVE") else { return };
    let guard = SCREEN.lock().unwrap();
    let Some(screen) = guard.as_ref() else { return };
    let mut rgb = vec![0u32; WIDTH * HEIGHT];
    screen.render(&mut rgb);
    if let Err(e) = std::fs::write(&path, bmp(&rgb)) {
        eprintln!("[PascRust] cannot save the screen to {path}: {e}");
    }
}

/// A 24-bit BMP file: a 54-byte header, then the rows bottom to top, each
/// pixel as blue, green, red. (640 * 3 bytes is a multiple of 4, so the rows
/// need no padding.)
fn bmp(rgb: &[u32]) -> Vec<u8> {
    let data_size = (WIDTH * HEIGHT * 3) as u32;
    let mut out = Vec::with_capacity(54 + data_size as usize);
    out.extend(b"BM");
    for n in [54 + data_size, 0, 54, 40] {
        out.extend(n.to_le_bytes()); // file size, reserved, data offset, header size
    }
    out.extend((WIDTH as i32).to_le_bytes());
    out.extend((HEIGHT as i32).to_le_bytes());
    out.extend(1u16.to_le_bytes()); // planes
    out.extend(24u16.to_le_bytes()); // bits per pixel
    for n in [0u32, data_size, 2835, 2835, 0, 0] {
        out.extend(n.to_le_bytes()); // compression, size, resolution x/y, colours
    }
    for row in rgb.chunks(WIDTH).rev() {
        for &p in row {
            out.extend([p as u8, (p >> 8) as u8, (p >> 16) as u8]);
        }
    }
    out
}
