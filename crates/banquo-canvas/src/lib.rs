#![forbid(unsafe_code)]
//! `banquo-canvas`: a CPU canvas with pygame Surface and Rect semantics.
//!
//! The shared data contracts are [`Rgba8`], [`IRect`] and [`Surface`]. The
//! crate has no GPU dependency, so headless rendering runs anywhere.

mod color;
mod rect;
mod surface;

pub mod crt;
pub mod headless;
pub mod raster;
pub mod text;

pub use color::Rgba8;
pub use rect::IRect;
pub use surface::Surface;
