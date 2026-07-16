use crate::theme::TextureKind;
use rand::RngExt;

/// Should the Face regenerate its cached background texture?
///
/// Pure cache decision: regenerate exactly when the resolved kind differs from
/// the cached one (`None` cached = nothing processed yet). Regenerating a
/// `Flat` kind drops the texture (generate returns `None`).
pub fn needs_texture_regen(cached: Option<TextureKind>, resolved: TextureKind) -> bool {
    cached != Some(resolved)
}

/// Generate the procedural background for a texture kind. `Flat` themes have
/// no texture (`None`); the Face paints their solid background fill instead.
pub fn generate(kind: TextureKind, width: usize, height: usize) -> Option<egui::ColorImage> {
    match kind {
        TextureKind::Flat => None,
        TextureKind::Blanco => Some(generate_blanco_texture(width, height)),
        TextureKind::Concrete => Some(generate_concrete_texture(width, height)),
        TextureKind::ConcreteDark => Some(generate_concrete_dark_texture(width, height)),
        TextureKind::Primordial => Some(generate_primordial_texture(width, height)),
    }
}

fn generate_blanco_texture(width: usize, height: usize) -> egui::ColorImage {
    let mut img =
        egui::ColorImage::new([width, height], vec![egui::Color32::WHITE; width * height]);
    let mut rng = rand::rng();

    // Base dots - more scarce
    for _ in 0..(width * height / 4000) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);

        let size = rng.random_range(1..=2); // smaller dots
        for dy in 0..size {
            for dx in 0..size {
                if x + dx < width && y + dy < height {
                    img.pixels[(y + dy) * width + (x + dx)] = egui::Color32::from_rgb(40, 40, 40);
                }
            }
        }
    }
    img
}

fn generate_concrete_texture(width: usize, height: usize) -> egui::ColorImage {
    let base_color = egui::Color32::from_rgb(130, 130, 130);
    let mut img = egui::ColorImage::new([width, height], vec![base_color; width * height]);
    let mut rng = rand::rng();

    // Base dots - more scarce
    for _ in 0..(width * height / 3000) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);

        let size = rng.random_range(1..=2); // smaller dots
        let color_choice = rng.random_range(0..3);
        let color = match color_choice {
            0 => egui::Color32::from_rgb(40, 40, 40),   // Blackish
            1 => egui::Color32::from_rgb(200, 120, 60), // Orange/Rust
            _ => egui::Color32::from_rgb(120, 90, 70),  // Brown
        };

        for dy in 0..size {
            for dx in 0..size {
                if x + dx < width && y + dy < height {
                    img.pixels[(y + dy) * width + (x + dx)] = color;
                }
            }
        }
    }

    // Add subtle noise to the base gray to give it texture
    for pixel in img.pixels.iter_mut() {
        if *pixel == base_color {
            let noise = rng.random_range(-10..=10);
            let r = (130_i32 + noise).clamp(0, 255) as u8;
            *pixel = egui::Color32::from_rgb(r, r, r);
        }
    }

    img
}

fn generate_primordial_texture(width: usize, height: usize) -> egui::ColorImage {
    // 80% opacity black background
    let base_color = egui::Color32::from_black_alpha(204);
    let mut img = egui::ColorImage::new([width, height], vec![base_color; width * height]);
    let mut rng = rand::rng();

    // Base dots - scarce
    for _ in 0..(width * height / 4000) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);

        let size = rng.random_range(1..=2); // smaller dots
        let color = egui::Color32::from_rgba_premultiplied(255, 0, 0, 204); // Red matching 80% alpha

        for dy in 0..size {
            for dx in 0..size {
                if x + dx < width && y + dy < height {
                    img.pixels[(y + dy) * width + (x + dx)] = color;
                }
            }
        }
    }
    img
}

fn generate_concrete_dark_texture(width: usize, height: usize) -> egui::ColorImage {
    let base_color = egui::Color32::from_rgb(20, 20, 20);
    let mut img = egui::ColorImage::new([width, height], vec![base_color; width * height]);
    let mut rng = rand::rng();

    // Base dots - more scarce
    for _ in 0..(width * height / 3000) {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);

        let size = rng.random_range(1..=2); // smaller dots
        let color_choice = rng.random_range(0..3);
        let color = match color_choice {
            0 => egui::Color32::from_rgb(0, 0, 0),    // Blackish
            1 => egui::Color32::from_rgb(80, 50, 30), // Orange/Rust (darker)
            _ => egui::Color32::from_rgb(40, 30, 25), // Brown (darker)
        };

        for dy in 0..size {
            for dx in 0..size {
                if x + dx < width && y + dy < height {
                    img.pixels[(y + dy) * width + (x + dx)] = color;
                }
            }
        }
    }

    // Add subtle noise to the base gray to give it texture
    for pixel in img.pixels.iter_mut() {
        if *pixel == base_color {
            let noise = rng.random_range(-5..=5);
            let r = (20_i32 + noise).clamp(0, 255) as u8;
            *pixel = egui::Color32::from_rgb(r, r, r);
        }
    }

    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_none_kind() {
        assert!(generate(TextureKind::Flat, 64, 64).is_none());
    }

    #[test]
    fn test_generate_textured_kinds_dimensions() {
        let kinds = [
            (TextureKind::Blanco, egui::Color32::WHITE),
            (
                TextureKind::Concrete,
                egui::Color32::from_rgb(130, 130, 130),
            ),
            (
                TextureKind::ConcreteDark,
                egui::Color32::from_rgb(20, 20, 20),
            ),
            (
                TextureKind::Primordial,
                egui::Color32::from_black_alpha(204),
            ),
        ];
        for (kind, base) in kinds {
            let img = generate(kind, 64, 64).expect("textured kind yields an image");
            assert_eq!(img.size, [64, 64], "{kind:?} size");
            // The base color survives somewhere (dots/noise never cover 100%).
            // Concrete kinds add +/- noise to the base, so accept near-base too.
            let near = |p: &egui::Color32| {
                let d = |a: u8, b: u8| a.abs_diff(b);
                d(p.r(), base.r()) <= 10 && d(p.g(), base.g()) <= 10 && d(p.b(), base.b()) <= 10
            };
            assert!(img.pixels.iter().any(near), "{kind:?} keeps its base color");
        }
    }

    #[test]
    fn test_texture_regen_decision() {
        assert!(needs_texture_regen(None, TextureKind::Blanco));
        assert!(!needs_texture_regen(
            Some(TextureKind::Blanco),
            TextureKind::Blanco
        ));
        assert!(needs_texture_regen(
            Some(TextureKind::Blanco),
            TextureKind::Concrete
        ));
        // Switching to a Flat theme must drop the texture (regen -> None).
        assert!(needs_texture_regen(
            Some(TextureKind::Concrete),
            TextureKind::Flat
        ));
        assert!(!needs_texture_regen(
            Some(TextureKind::Flat),
            TextureKind::Flat
        ));
    }
}
