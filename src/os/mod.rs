pub mod windows;

pub fn apply_window_effects(config: &crate::config::BanquoConfig, frame: &mut eframe::Frame) {
    #[cfg(target_os = "windows")]
    windows::apply_effects(config, frame);
    // Ignore on other OSes for now
}
