//! The graphics screen: a framebuffer of colour numbers plus the BGI's
//! drawing state (current colour, line and fill styles, text settings,
//! viewport, current position), and the drawing primitives.
//!
//! Coordinates passed to the drawing methods are relative to the viewport,
//! as in the BGI. Pixels outside the screen (or outside a clipping viewport)
//! are silently dropped.

use super::font::FONT;

pub const WIDTH: usize = 640;
pub const HEIGHT: usize = 480;

/// The EGA's default palette: for each of the 16 colour numbers, a 6-bit
/// `rgbRGB` colour (lowercase bits are 1/3 intensity, uppercase 2/3). Colour
/// 6 is 20, not 6, which turns dark yellow into brown.
pub const DEFAULT_PALETTE: [u8; 16] = [0, 1, 2, 3, 4, 5, 20, 7, 56, 57, 58, 59, 60, 61, 62, 63];

/// A 6-bit EGA colour as `0xRRGGBB`.
pub fn ega_rgb(ega: u8) -> u32 {
    // Each channel has a 2/3 bit and a 1/3 bit: 0x00, 0x55, 0xAA or 0xFF.
    let channel = |high: u8, low: u8| -> u32 {
        (if ega & (1 << high) != 0 { 0xAA } else { 0 }) + (if ega & (1 << low) != 0 { 0x55 } else { 0 })
    };
    (channel(2, 5) << 16) | (channel(1, 4) << 8) | channel(0, 3)
}

/// A VGA colour register's 6-bit components (the low 6 bits of each) as
/// `0xRRGGBB`, each scaled from 0..63 to 0..255.
pub fn dac_rgb(red: i32, green: i32, blue: i32) -> u32 {
    let component = |c: i32| ((c & 63) * 255 / 63) as u32;
    (component(red) << 16) | (component(green) << 8) | component(blue)
}

/// Line patterns for `SetLineStyle`: one bit per pixel along the line.
const LINE_PATTERNS: [u16; 4] = [
    0xFFFF, // SolidLn
    0xCCCC, // DottedLn
    0xFC78, // CenterLn
    0xF8F8, // DashedLn
];

/// Fill patterns for `SetFillStyle`, 8x8, bit 7 is the leftmost pixel.
/// Where a pattern has a 0 bit, the BGI paints the background colour.
const FILL_PATTERNS: [[u8; 8]; 12] = [
    [0x00; 8],                                        // EmptyFill
    [0xFF; 8],                                        // SolidFill
    [0xFF, 0xFF, 0x00, 0x00, 0xFF, 0xFF, 0x00, 0x00], // LineFill
    [0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80], // LtSlashFill
    [0xE0, 0xC1, 0x83, 0x07, 0x0E, 0x1C, 0x38, 0x70], // SlashFill
    [0xF0, 0x78, 0x3C, 0x1E, 0x0F, 0x87, 0xC3, 0xE1], // BkSlashFill
    [0xA5, 0xD2, 0x69, 0xB4, 0x5A, 0x2D, 0x96, 0x4B], // LtBkSlashFill
    [0xFF, 0x88, 0x88, 0x88, 0xFF, 0x88, 0x88, 0x88], // HatchFill
    [0x81, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x81], // XHatchFill
    [0xCC, 0x33, 0xCC, 0x33, 0xCC, 0x33, 0xCC, 0x33], // InterleaveFill
    [0x80, 0x00, 0x08, 0x00, 0x80, 0x00, 0x08, 0x00], // WideDotFill
    [0x88, 0x00, 0x22, 0x00, 0x88, 0x00, 0x22, 0x00], // CloseDotFill
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteMode {
    Copy,
    Xor,
}

pub struct Screen {
    /// One colour number (0..15) per pixel, row by row: the active page,
    /// where drawing goes.
    pub pixels: Vec<u8>,
    /// The other page, put aside while it isn't active, and the numbers of
    /// the active page and the page on show (`SetActivePage`,
    /// `SetVisualPage`).
    pub other_page: Vec<u8>,
    pub active_page: usize,
    pub visual_page: usize,
    /// The EGA colour each colour number is displayed as.
    pub palette: [u8; 16],
    /// The VGA's colour registers (its DAC): what each of the 64 EGA colours
    /// really looks like, as `0xRRGGBB`. `SetRGBPalette` changes them.
    pub dac: [u32; 64],
    /// `SetAspectRatio`, for `GetAspectRatio`.
    pub aspect: (u16, u16),
    pub color: u8,
    pub bk_color: u8,
    /// One bit per pixel along a line; thickness is 1 or 3. And the style
    /// number and pattern as `SetLineStyle` gave them, for `GetLineSettings`.
    pub line_pattern: u16,
    pub thickness: i32,
    pub line_style: (i32, u16),
    pub fill_pattern: [u8; 8],
    pub fill_color: u8,
    /// The `SetFillStyle` pattern number (`UserFill` after `SetFillPattern`).
    pub fill_style: i32,
    /// `SetTextStyle`'s font and size, for `GetTextSettings`.
    pub font: (i32, i32),
    /// Where the last arc was drawn: centre, start and end points, for
    /// `GetArcCoords`.
    pub arc: [i32; 6],
    /// `SetTextJustify`: horizontal 0 left, 1 centre, 2 right; vertical
    /// 0 bottom, 1 centre, 2 top.
    pub justify: (u8, u8),
    /// `SetTextStyle`: true for vertical text, and the magnification.
    pub vertical: bool,
    pub text_scale: i32,
    /// The viewport (absolute screen coordinates) and whether it clips.
    pub view: (i32, i32, i32, i32),
    pub clip: bool,
    /// The current position, relative to the viewport.
    pub cp: (i32, i32),
    pub write_mode: WriteMode,
}

/// The bytes `GetImage` needs for a `width` by `height` rectangle: the
/// 6-byte header, then four planes of `width / 8` bytes (rounded up) for
/// each row.
pub fn image_bytes(width: i32, height: i32) -> usize {
    6 + (width.max(0) as usize).div_ceil(8) * 4 * height.max(0) as usize
}

impl Screen {
    pub fn new() -> Screen {
        Screen {
            pixels: vec![0; WIDTH * HEIGHT],
            other_page: vec![0; WIDTH * HEIGHT],
            active_page: 0,
            visual_page: 0,
            palette: DEFAULT_PALETTE,
            // `from_fn` builds an array from its index: entry `i` is EGA
            // colour `i`.
            dac: std::array::from_fn(|i| ega_rgb(i as u8)),
            aspect: (10000, 10000),
            color: 15,
            bk_color: 0,
            line_pattern: LINE_PATTERNS[0],
            thickness: 1,
            line_style: (0, 0xFFFF),
            fill_pattern: FILL_PATTERNS[1],
            fill_color: 15,
            fill_style: 1,
            font: (0, 1),
            arc: [0; 6],
            justify: (0, 2),
            vertical: false,
            text_scale: 1,
            view: (0, 0, WIDTH as i32 - 1, HEIGHT as i32 - 1),
            clip: true,
            cp: (0, 0),
            write_mode: WriteMode::Copy,
        }
    }

    /// The screen as `0xRRGGBB` pixels, for a window or an image file.
    pub fn render(&self, out: &mut [u32]) {
        let rgb: Vec<u32> = self.palette.iter().map(|&e| self.dac[usize::from(e & 63)]).collect();
        // The page on show, which needn't be the one being drawn on.
        let shown = if self.visual_page == self.active_page { &self.pixels } else { &self.other_page };
        for (dst, &src) in out.iter_mut().zip(shown) {
            *dst = rgb[usize::from(src & 15)];
        }
    }

    // Pixels.

    /// `SetActivePage(Page)`: from now on, draw on page 0 or 1. The pages
    /// trade places, so drawing always goes to `pixels`.
    pub fn set_active_page(&mut self, page: usize) {
        if page < 2 && page != self.active_page {
            std::mem::swap(&mut self.pixels, &mut self.other_page);
            self.active_page = page;
        }
    }

    /// The index of viewport-relative `(x, y)` in `pixels`, if it's visible.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        let (left, top, right, bottom) = self.view;
        let (ax, ay) = (x + left, y + top);
        if self.clip && (ax < left || ax > right || ay < top || ay > bottom) {
            return None;
        }
        if ax < 0 || ay < 0 || ax >= WIDTH as i32 || ay >= HEIGHT as i32 {
            return None;
        }
        Some(ay as usize * WIDTH + ax as usize)
    }

    pub fn put_pixel(&mut self, x: i32, y: i32, color: u8) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = color & 15;
        }
    }

    /// A pixel of a line or outline: honours `SetWriteMode`.
    fn plot(&mut self, x: i32, y: i32) {
        if let Some(i) = self.index(x, y) {
            self.pixels[i] = match self.write_mode {
                WriteMode::Copy => self.color,
                WriteMode::Xor => self.pixels[i] ^ self.color,
            };
        }
    }

    pub fn get_pixel(&self, x: i32, y: i32) -> u8 {
        // GetPixel isn't clipped to the viewport, only to the screen.
        let (ax, ay) = (x + self.view.0, y + self.view.1);
        if ax < 0 || ay < 0 || ax >= WIDTH as i32 || ay >= HEIGHT as i32 {
            0
        } else {
            self.pixels[ay as usize * WIDTH + ax as usize]
        }
    }

    /// `GetImage`: the rectangle's pixels in the layout TP's EGA/VGA driver
    /// used, so a buffer sized by hand, a program that reads the header and
    /// an image saved to a file by the original program all still work.
    /// Three `Word`s (the width and height, each minus one, and one TP
    /// left reserved), then each row as the four bit planes from plane 3
    /// (intensity) down to plane 0 (blue), each one bit per pixel, eight
    /// pixels to a byte with the leftmost in bit 7.
    pub fn get_image(&self, x1: i32, y1: i32, x2: i32, y2: i32) -> Vec<u8> {
        let (width, height) = (x2 - x1 + 1, y2 - y1 + 1);
        let row_bytes = (width as usize).div_ceil(8);
        let mut image = Vec::with_capacity(image_bytes(width, height));
        for word in [width - 1, height - 1, 0] {
            image.extend_from_slice(&(word as u16).to_le_bytes());
        }
        for y in y1..=y2 {
            for plane in (0..4).rev() {
                let mut bits = vec![0u8; row_bytes];
                for x in 0..width {
                    if self.get_pixel(x1 + x, y) & (1 << plane) != 0 {
                        bits[x as usize / 8] |= 0x80 >> (x % 8);
                    }
                }
                image.extend(bits);
            }
        }
        image
    }

    /// `PutImage(X, Y, BitMap, BitBlt)`: draw an image from `GetImage` with
    /// its top left corner at `(x, y)`, combining each pixel with the
    /// screen's as `BitBlt` says. Pixels outside the viewport (when it
    /// clips) or off the screen are dropped.
    pub fn put_image(&mut self, x: i32, y: i32, image: &[u8], bit_blt: i32) {
        let word = |i: usize| i32::from(u16::from_le_bytes([image[i], image[i + 1]]));
        let (width, height) = (word(0) + 1, word(2) + 1);
        let row_bytes = (width as usize).div_ceil(8);
        for row in 0..height {
            let start = 6 + row as usize * 4 * row_bytes;
            for col in 0..width {
                let (byte, bit) = (col as usize / 8, 0x80 >> (col % 8));
                let mut color = 0;
                for plane in 0..4 {
                    // Plane 3 comes first in the row.
                    if image[start + (3 - plane) * row_bytes + byte] & bit != 0 {
                        color |= 1 << plane;
                    }
                }
                let Some(i) = self.index(x + col, y + row) else { continue };
                let pixel = &mut self.pixels[i];
                *pixel = match bit_blt {
                    1 => *pixel ^ color, // XORPut
                    2 => *pixel | color, // OrPut
                    3 => *pixel & color, // AndPut
                    4 => !color & 15,    // NotPut
                    _ => color,          // NormalPut
                };
            }
        }
    }

    /// A pixel of a filled area: the fill colour where the pattern has a 1,
    /// the background (colour 0) where it has a 0.
    fn fill_pixel(&mut self, x: i32, y: i32) {
        // The pattern is anchored to the screen, not the viewport.
        let (ax, ay) = (x + self.view.0, y + self.view.1);
        let bit = self.fill_pattern[ay.rem_euclid(8) as usize] & (0x80 >> ax.rem_euclid(8));
        let color = if bit != 0 { self.fill_color } else { 0 };
        self.put_pixel(x, y, color);
    }

    fn fill_span(&mut self, x1: i32, x2: i32, y: i32) {
        for x in x1.min(x2)..=x1.max(x2) {
            self.fill_pixel(x, y);
        }
    }

    // Lines.

    /// Bresenham's line algorithm: step along the longer axis one pixel at a
    /// time, and along the shorter one whenever the accumulated error says
    /// the true line has moved half a pixel. `pattern` is the line style.
    fn bresenham(&mut self, (x1, y1): (i32, i32), (x2, y2): (i32, i32), pattern: u16) {
        let (dx, dy) = ((x2 - x1).abs(), -(y2 - y1).abs());
        let (sx, sy) = ((x2 - x1).signum(), (y2 - y1).signum());
        let (mut x, mut y, mut err) = (x1, y1, dx + dy);
        let mut bit = 0;
        loop {
            if pattern & (0x8000 >> (bit % 16)) != 0 {
                self.plot(x, y);
            }
            bit += 1;
            if x == x2 && y == y2 {
                return;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// A line in the current style. A thick line is three lines side by
    /// side, offset across the line's main direction, as the BGI does it.
    pub fn line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        let pattern = self.line_pattern;
        if self.thickness < 3 {
            self.bresenham((x1, y1), (x2, y2), pattern);
            return;
        }
        let mostly_horizontal = (x2 - x1).abs() >= (y2 - y1).abs();
        for d in -1..=1 {
            let (ox, oy) = if mostly_horizontal { (0, d) } else { (d, 0) };
            self.bresenham((x1 + ox, y1 + oy), (x2 + ox, y2 + oy), pattern);
        }
    }

    pub fn rectangle(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        self.line(x1, y1, x2, y1);
        self.line(x2, y1, x2, y2);
        self.line(x2, y2, x1, y2);
        self.line(x1, y2, x1, y1);
    }

    pub fn bar(&mut self, x1: i32, y1: i32, x2: i32, y2: i32) {
        for y in y1.min(y2)..=y1.max(y2) {
            self.fill_span(x1, x2, y);
        }
    }

    // Curves.

    /// Points on an elliptical arc, `start`..`end` degrees counterclockwise
    /// from 3 o'clock (screen y grows downwards, hence `cy - ...`).
    fn arc_points(cx: i32, cy: i32, start: i32, end: i32, xr: i32, yr: i32) -> Vec<(i32, i32)> {
        let mut end = end;
        while end < start {
            end += 360;
        }
        // Enough steps that consecutive points are about a pixel apart.
        let span = f64::from(end - start).to_radians();
        let steps = ((f64::from(xr.max(yr).max(1)) * span).ceil() as i32).max(1);
        (0..=steps)
            .map(|i| {
                let a = f64::from(start).to_radians() + span * f64::from(i) / f64::from(steps);
                (cx + (f64::from(xr) * a.cos()).round() as i32, cy - (f64::from(yr) * a.sin()).round() as i32)
            })
            .collect()
    }

    /// An elliptical arc in the current colour. Arcs are always solid; a
    /// thick one is three arcs of neighbouring radii.
    pub fn ellipse(&mut self, cx: i32, cy: i32, start: i32, end: i32, xr: i32, yr: i32) {
        self.remember_arc(cx, cy, start, end, xr, yr);
        let radii: &[i32] = if self.thickness >= 3 { &[-1, 0, 1] } else { &[0] };
        for &d in radii {
            let points = Screen::arc_points(cx, cy, start, end, xr + d, yr + d);
            // Join the points with short lines so the curve has no gaps.
            for pair in points.windows(2) {
                self.bresenham(pair[0], pair[1], 0xFFFF);
            }
        }
    }

    /// The inside of an ellipse, one horizontal span per row.
    pub fn fill_ellipse_inside(&mut self, cx: i32, cy: i32, xr: i32, yr: i32) {
        for dy in -yr..=yr {
            let t = 1.0 - (f64::from(dy) / f64::from(yr.max(1))).powi(2);
            let half = (f64::from(xr) * t.max(0.0).sqrt()).round() as i32;
            self.fill_span(cx - half, cx + half, cy + dy);
        }
    }

    /// The centre and the end points of an arc, for `GetArcCoords`.
    fn remember_arc(&mut self, cx: i32, cy: i32, start: i32, end: i32, xr: i32, yr: i32) {
        // Angles go counterclockwise from 3 o'clock, and y grows downwards.
        let point = |angle: i32| {
            let radians = f64::from(angle).to_radians();
            let x = cx + (radians.cos() * f64::from(xr)).round() as i32;
            let y = cy - (radians.sin() * f64::from(yr)).round() as i32;
            (x, y)
        };
        let ((xs, ys), (xe, ye)) = (point(start), point(end));
        self.arc = [cx, cy, xs, ys, xe, ye];
    }

    /// `FillPoly`'s inside: for each row, where the edges cross it, filled
    /// between alternate crossings (the even-odd rule).
    pub fn fill_polygon_inside(&mut self, points: &[(i32, i32)]) {
        let Some(top) = points.iter().map(|p| p.1).min() else { return };
        let bottom = points.iter().map(|p| p.1).max().unwrap_or(top);
        for y in top..=bottom {
            let mut crossings: Vec<i32> = Vec::new();
            for (i, &(x1, y1)) in points.iter().enumerate() {
                let (x2, y2) = points[(i + 1) % points.len()];
                // Each edge counts for the rows from its top up to, but not
                // including, its bottom, so a shared corner counts once.
                if (y1 <= y && y < y2) || (y2 <= y && y < y1) {
                    crossings.push(x1 + (y - y1) * (x2 - x1) / (y2 - y1));
                }
            }
            crossings.sort_unstable();
            for pair in crossings.chunks(2) {
                if let [from, to] = pair {
                    self.fill_span(*from, *to, y);
                }
            }
        }
    }

    /// A filled wedge of an ellipse (`PieSlice`, `Sector`), outlined.
    pub fn sector(&mut self, cx: i32, cy: i32, start: i32, end: i32, xr: i32, yr: i32) {
        self.remember_arc(cx, cy, start, end, xr, yr);
        let (start_f, mut end_f) = (f64::from(start.rem_euclid(360)), f64::from(end.rem_euclid(360)));
        if end_f <= start_f {
            end_f += 360.0;
        }
        for dy in -yr..=yr {
            for dx in -xr..=xr {
                let inside =
                    (f64::from(dx) / f64::from(xr.max(1))).powi(2) + (f64::from(dy) / f64::from(yr.max(1))).powi(2);
                if inside > 1.0 {
                    continue;
                }
                let mut angle = f64::from(-dy).atan2(f64::from(dx)).to_degrees().rem_euclid(360.0);
                if angle < start_f {
                    angle += 360.0;
                }
                if angle <= end_f {
                    self.fill_pixel(cx + dx, cy + dy);
                }
            }
        }
        self.ellipse(cx, cy, start, end, xr, yr);
        let points = Screen::arc_points(cx, cy, start, end, xr, yr);
        if let (Some(&first), Some(&last)) = (points.first(), points.last()) {
            self.line(cx, cy, first.0, first.1);
            self.line(cx, cy, last.0, last.1);
        }
    }

    /// Fill the area around `(x, y)` up to pixels of colour `border`.
    pub fn flood_fill(&mut self, x: i32, y: i32, border: u8) {
        let mut seen = vec![false; WIDTH * HEIGHT];
        let mut stack = vec![(x, y)];
        while let Some((x, y)) = stack.pop() {
            let Some(i) = self.index(x, y) else { continue };
            if seen[i] || self.pixels[i] == border {
                continue;
            }
            seen[i] = true;
            self.fill_pixel(x, y);
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
    }

    // Text.

    /// The width and height of `text` in pixels, before rotation.
    pub fn text_size(&self, text: &[u8]) -> (i32, i32) {
        (8 * self.text_scale * text.len() as i32, 8 * self.text_scale)
    }

    /// Draw `text` with its reference point at `(x, y)`, justified as set by
    /// `SetTextJustify`.
    pub fn out_text_xy(&mut self, x: i32, y: i32, text: &[u8]) {
        let (w, h) = self.text_size(text);
        let s = self.text_scale;
        let shift = |justify: u8, size: i32| match justify {
            1 => size / 2,
            2 => size,
            _ => 0,
        };
        if !self.vertical {
            // Horizontal: 0 = left; vertical: 0 = bottom, 2 = top.
            let left = x - shift(self.justify.0, w);
            let top = y - h + shift(self.justify.1, h);
            for (n, &c) in text.iter().enumerate() {
                self.glyph(c, |gx, gy| (left + n as i32 * 8 * s + gx, top + gy));
            }
        } else {
            // Vertical text reads upwards: rotated 90 degrees anticlockwise.
            let bottom = y + shift(self.justify.0, w);
            let left = x - h + shift(self.justify.1, h);
            for (n, &c) in text.iter().enumerate() {
                self.glyph(c, |gx, gy| (left + gy, bottom - n as i32 * 8 * s - gx));
            }
        }
    }

    /// Draw one character; `place` maps a pixel of the (scaled) glyph to the
    /// screen.
    fn glyph(&mut self, c: u8, place: impl Fn(i32, i32) -> (i32, i32)) {
        let rows = FONT.get(usize::from(c)).copied().unwrap_or([0; 8]);
        let s = self.text_scale;
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..8 {
                if bits & (1 << col) == 0 {
                    continue;
                }
                for sy in 0..s {
                    for sx in 0..s {
                        let (x, y) = place(col * s + sx, row as i32 * s + sy);
                        let color = self.color;
                        self.put_pixel(x, y, color);
                    }
                }
            }
        }
    }

    // State.

    pub fn set_line_style(&mut self, style: i32, pattern: u16, thickness: i32) {
        self.line_style = (style, pattern);
        self.line_pattern = match style {
            4 => pattern,
            s => LINE_PATTERNS.get(s as usize).copied().unwrap_or(0xFFFF),
        };
        self.thickness = thickness;
    }

    pub fn set_fill_style(&mut self, pattern: i32, color: u8) {
        if let Some(p) = FILL_PATTERNS.get(pattern as usize) {
            self.fill_pattern = *p;
            self.fill_style = pattern;
        }
        self.fill_color = color & 15;
    }

    pub fn clear_view_port(&mut self) {
        let (left, top, right, bottom) = self.view;
        for y in top.max(0)..=bottom.min(HEIGHT as i32 - 1) {
            for x in left.max(0)..=right.min(WIDTH as i32 - 1) {
                self.pixels[y as usize * WIDTH + x as usize] = 0;
            }
        }
        self.cp = (0, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_visual_page_is_shown() {
        let mut s = Screen::new();
        s.set_active_page(1);
        s.put_pixel(0, 0, 15);
        let mut out = vec![0; WIDTH * HEIGHT];
        s.render(&mut out);
        assert_eq!(out[0], 0, "page 0 is on show");
        s.visual_page = 1;
        s.render(&mut out);
        assert_eq!(out[0], 0xFFFFFF, "page 1 is on show");
        s.set_active_page(0);
        s.render(&mut out);
        assert_eq!(out[0], 0xFFFFFF, "still page 1 while drawing on page 0");
    }

    #[test]
    fn rgb_palette_recolours() {
        let mut s = Screen::new();
        s.put_pixel(0, 0, 4); // red, EGA colour 4
        s.dac[4] = dac_rgb(63, 32, 0);
        let mut out = vec![0; WIDTH * HEIGHT];
        s.render(&mut out);
        assert_eq!(out[0], 0xFF8100);
    }

    #[test]
    fn palette_colours() {
        assert_eq!(ega_rgb(DEFAULT_PALETTE[0]), 0x000000);
        assert_eq!(ega_rgb(DEFAULT_PALETTE[1]), 0x0000AA);
        assert_eq!(ega_rgb(DEFAULT_PALETTE[6]), 0xAA5500); // brown
        assert_eq!(ega_rgb(DEFAULT_PALETTE[14]), 0xFFFF55);
        assert_eq!(ega_rgb(DEFAULT_PALETTE[15]), 0xFFFFFF);
    }

    #[test]
    fn images_in_tp_layout() {
        let mut s = Screen::new();
        s.put_pixel(0, 0, 9); // 1001: planes 3 and 0
        s.put_pixel(9, 1, 6); // 0110: planes 2 and 1
        let image = s.get_image(0, 0, 9, 1);
        assert_eq!(image.len(), image_bytes(10, 2));
        assert_eq!(image[..6], [9, 0, 1, 0, 0, 0]);
        // Row 0: planes 3, 2, 1, 0, two bytes each; pixel 0 is bit 7.
        assert_eq!(image[6..14], [0x80, 0, 0, 0, 0, 0, 0x80, 0]);
        // Row 1: pixel 9 is bit 6 of the second byte.
        assert_eq!(image[14..22], [0, 0, 0, 0x40, 0, 0x40, 0, 0]);
        s.put_image(20, 20, &image, 0);
        assert_eq!((s.get_pixel(20, 20), s.get_pixel(29, 21)), (9, 6));
        s.put_image(20, 20, &image, 1); // XORPut twice clears it
        assert_eq!((s.get_pixel(20, 20), s.get_pixel(29, 21)), (0, 0));
        s.put_image(20, 20, &image, 4); // NotPut
        assert_eq!((s.get_pixel(20, 20), s.get_pixel(21, 20)), (6, 15));
    }

    #[test]
    fn lines_reach_both_ends() {
        let mut s = Screen::new();
        s.color = 4;
        s.line(10, 10, 20, 15);
        assert_eq!(s.get_pixel(10, 10), 4);
        assert_eq!(s.get_pixel(20, 15), 4);
        assert_eq!(s.get_pixel(15, 20), 0);
    }

    #[test]
    fn circle_and_fill() {
        let mut s = Screen::new();
        s.color = 2;
        s.ellipse(100, 100, 0, 360, 20, 20);
        assert_eq!(s.get_pixel(120, 100), 2);
        assert_eq!(s.get_pixel(100, 80), 2);
        assert_eq!(s.get_pixel(100, 100), 0);
        s.set_fill_style(1, 3);
        s.flood_fill(100, 100, 2);
        assert_eq!(s.get_pixel(100, 100), 3);
        assert_eq!(s.get_pixel(130, 100), 0);
    }

    #[test]
    fn text_justification() {
        let mut s = Screen::new();
        s.justify = (1, 1); // centred both ways
        s.out_text_xy(100, 100, b"I");
        // `I` is drawn in the 8x8 cell from (96, 96) to (103, 103).
        let lit: Vec<(i32, i32)> =
            (90..110).flat_map(|y| (90..110).map(move |x| (x, y))).filter(|&(x, y)| s.get_pixel(x, y) != 0).collect();
        assert!(lit.iter().all(|&(x, y)| (96..104).contains(&x) && (96..104).contains(&y)), "{lit:?}");
        assert!(!lit.is_empty());
    }
}
