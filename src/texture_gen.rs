use rand::RngExt;

pub fn generate_blanco_texture(width: usize, height: usize) -> egui::ColorImage {
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

pub fn generate_concrete_texture(width: usize, height: usize) -> egui::ColorImage {
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

pub fn generate_primordial_texture(width: usize, height: usize) -> egui::ColorImage {
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
