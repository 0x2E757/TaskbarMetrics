//! The app logo, rasterized for one icon size at a time: an accent tile with a thin
//! white pulse across it. Plain std, so the build script embeds the same pixels in
//! the executables that the window draws at run time.

/// The logo laid out on the pixel grid of one icon size: the tile's edges cover
/// whole pixels and the pulse is centred on a pixel row.
struct IconShape {
    size: f64,
    inset: f64,
    radius: f64,
    half_stroke: f64,
    pulse: [(f64, f64); 7],
}
impl IconShape {
    /// The pulse in 16-unit space; half its stroke gives round caps and joins.
    const PULSE: [(f64, f64); 7] = [
        (3.0, 8.5),
        (5.5, 8.5),
        (6.75, 5.25),
        (8.75, 11.75),
        (10.25, 7.5),
        (11.25, 8.5),
        (13.0, 8.5),
    ];
    const HALF_STROKE: f64 = 0.55;
    /// The pulse is a quarter thinner than a whole-pixel stroke.
    const THINNING: f64 = 0.75;
    /// Width at both ends of the pulse, as a share of its full width.
    const END: f64 = 1.0 / 3.0;
    fn new(size: u32) -> Self {
        let size = size as f64;
        let scale = size / 16.0;
        let stroke = (2.0 * Self::HALF_STROKE * scale).round().max(1.0) * Self::THINNING;
        let snap = |v: f64| v.floor() + 0.5;
        Self {
            size,
            inset: scale.round(),
            radius: 3.5 * scale,
            half_stroke: stroke / 2.0,
            pulse: Self::PULSE.map(|(x, y)| (snap(x * scale), snap(y * scale))),
        }
    }
    /// Distance from `point` to the segment `a`–`b`, and where along it the nearest point is.
    fn distance((x, y): (f64, f64), (ax, ay): (f64, f64), (bx, by): (f64, f64)) -> (f64, f64) {
        let (dx, dy) = (bx - ax, by - ay);
        let t = (((x - ax) * dx + (y - ay) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
        ((x - ax - t * dx).hypot(y - ay - t * dy), t)
    }
    /// Half width at `t` along segment `index`: the first and the last runs taper
    /// from full width to `END` at the outer ends.
    fn half_width(&self, index: usize, t: f64) -> f64 {
        let taper = |share: f64| Self::END + (1.0 - Self::END) * share;
        self.half_stroke
            * match index {
                0 => taper(t),
                i if i == self.pulse.len() - 2 => taper(1.0 - t),
                _ => 1.0,
            }
    }
    /// Whether the pixel-space point is on the tile and on the pulse.
    fn coverage(&self, x: f64, y: f64) -> (bool, bool) {
        let middle = self.size / 2.0;
        let corner = middle - self.inset - self.radius;
        let (dx, dy) = ((x - middle).abs() - corner, (y - middle).abs() - corner);
        let tile = if dx > 0.0 && dy > 0.0 {
            dx.hypot(dy) <= self.radius
        } else {
            dx <= self.radius && dy <= self.radius
        };
        let line = self.pulse.windows(2).enumerate().any(|(index, pair)| {
            let (distance, t) = Self::distance((x, y), pair[0], pair[1]);
            distance <= self.half_width(index, t)
        });
        (tile, line)
    }
}

pub struct AppIcon;
impl AppIcon {
    /// `size`×`size` pixels, rows top-down, as 0xAARRGGBB with straight alpha.
    pub fn pixels(size: u32) -> Vec<u32> {
        let shape = IconShape::new(size);
        const SAMPLES: u32 = 8;
        let total = (SAMPLES * SAMPLES) as f64;
        let mut pixels = Vec::with_capacity((size * size) as usize);
        for py in 0..size {
            for px in 0..size {
                let (mut tile, mut line) = (0.0, 0.0);
                for sy in 0..SAMPLES {
                    for sx in 0..SAMPLES {
                        let x = px as f64 + (sx as f64 + 0.5) / SAMPLES as f64;
                        let y = py as f64 + (sy as f64 + 0.5) / SAMPLES as f64;
                        if let (true, on_line) = shape.coverage(x, y) {
                            tile += 1.0;
                            if on_line {
                                line += 1.0;
                            }
                        }
                    }
                }
                let white = if tile > 0.0 { line / tile } else { 0.0 };
                let channel = |accent: f64| (accent + (255.0 - accent) * white).round() as u32;
                let alpha = (tile / total * 255.0).round() as u32;
                pixels.push(
                    (alpha << 24) | (channel(0.0) << 16) | (channel(95.0) << 8) | channel(184.0),
                );
            }
        }
        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Share of the pixel covered by the tile (`tile`) or the pulse.
    fn share(shape: &IconShape, px: u32, py: u32, tile: bool) -> f64 {
        let hits = (0..64)
            .filter(|i| {
                let x = px as f64 + (*i % 8) as f64 / 8.0 + 1.0 / 16.0;
                let y = py as f64 + (*i / 8) as f64 / 8.0 + 1.0 / 16.0;
                let (t, l) = shape.coverage(x, y);
                if tile {
                    t
                } else {
                    l
                }
            })
            .count();
        hits as f64 / 64.0
    }
    #[test]
    fn tile_edges_cover_whole_pixels_and_the_pulse_tapers_to_its_ends() {
        for size in [16, 20, 24, 30, 32, 36, 40, 48, 64, 96, 128, 256] {
            let shape = IconShape::new(size);
            // Painted height of a pixel column of the pulse.
            let width =
                |column: u32| -> f64 { (0..size).map(|py| share(&shape, column, py, false)).sum() };
            let end = width(shape.pulse[0].0 as u32);
            let full = width(shape.pulse[1].0 as u32 - 1);
            let stroke = 2.0 * shape.half_stroke;
            assert!(
                end < stroke / 2.0 && end < full,
                "{size} px: end {end}, next to the bend {full}, stroke {stroke}"
            );
            for py in 0..size {
                let edge = share(&shape, py, size / 2, true);
                assert!(
                    edge == 0.0 || edge == 1.0,
                    "{size} px: tile column {py} is {edge}"
                );
            }
        }
    }
    #[test]
    fn pixels_fill_the_square_and_leave_the_corners_clear() {
        let pixels = AppIcon::pixels(24);
        assert_eq!(pixels.len(), 24 * 24);
        assert_eq!(pixels[0] >> 24, 0);
        assert_eq!(pixels[12 * 24 + 3] >> 24, 255);
    }
}
