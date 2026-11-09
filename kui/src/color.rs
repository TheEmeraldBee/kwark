/// An sRGB color with alpha
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self::rgb(0x000000);
    pub const WHITE: Self = Self::rgb(0xFFFFFF);
    pub const TRANSPARENT: Self = Self::rgba(0x00000000);

    /// Returns an opaque color from `0xRRGGBB`
    pub const fn rgb(hex: u32) -> Self {
        Self::rgba((hex << 8) | 0xFF)
    }

    /// Returns a color from `0xRRGGBBAA`
    pub const fn rgba(hex: u32) -> Self {
        let [r, g, b, a] = hex.to_be_bytes();
        Self { r, g, b, a }
    }

    /// Returns the color with its alpha scaled (multiplied) by `opacity` (0 to 1)
    pub fn opacity(self, opacity: f32) -> Self {
        let a = (self.a as f32 * opacity.clamp(0.0, 1.0)).round() as u8;
        Self { a, ..self }
    }
}

impl From<Color> for vello::peniko::Color {
    fn from(c: Color) -> Self {
        Self::from_rgba8(c.r, c.g, c.b, c.a)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn hex_splits_into_channels() {
        assert_eq!(
            Color::rgba(0x11223344),
            Color {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0x44
            }
        );
        assert_eq!(Color::rgb(0x112233).a, 0xFF);
    }

    #[test]
    fn opacity_scales_alpha() {
        assert_eq!(Color::WHITE.opacity(0.5).a, 128);
        assert_eq!(Color::WHITE.opacity(2.0).a, 255);
    }
}
