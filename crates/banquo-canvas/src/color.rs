//! 8-bit RGBA colour.

use serde::{Deserialize, Serialize};

/// An 8-bit-per-channel RGBA colour, straight (non-premultiplied) alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Rgba8 {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba8 {
    /// A colour from all four channels.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// An opaque colour (`a = 255`).
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_is_opaque() {
        assert_eq!(Rgba8::rgb(1, 2, 3), Rgba8::new(1, 2, 3, 255));
    }

    #[test]
    fn round_trips_through_json() {
        let c = Rgba8::new(10, 20, 30, 40);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Rgba8>(&json).unwrap(), c);
    }
}
