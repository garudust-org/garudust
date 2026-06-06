//! egui prototype — a pure-native Rust desktop UI (no webview/WASM/JS) with the
//! Garudust agent embedded in-process. Full feature parity with the Leptos web
//! UI: Chat (streaming, model picker, New chat, Stop), Status, Config (selects +
//! routing editor + key hints), and write-only masked Secrets with delete.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod config_io;

use eframe::egui;

/// Load a Thai-capable fallback font so Thai text renders (egui's default font
/// is Latin-only — a webview gets this free from the OS).
fn setup_fonts(ctx: &egui::Context) {
    // macOS Thai font; harmless no-op on other platforms.
    const THAI: &str = "/System/Library/Fonts/Supplemental/Ayuthaya.ttf";
    let Ok(bytes) = std::fs::read(THAI) else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("thai".to_owned(), egui::FontData::from_owned(bytes).into());
    for fam in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(fam)
            .or_default()
            .push("thai".to_owned());
    }
    ctx.set_fonts(fonts);
}

fn main() -> eframe::Result<()> {
    let t0 = std::time::Instant::now();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([920.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Garudust",
        options,
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::dark());
            egui_extras::install_image_loaders(&cc.egui_ctx);
            setup_fonts(&cc.egui_ctx);
            // Restore saved appearance prefs (default: dark, smallest size).
            let dark = cc
                .storage
                .and_then(|s| eframe::get_value(s, "dark"))
                .unwrap_or(true);
            let font_level = cc
                .storage
                .and_then(|s| eframe::get_value(s, "font_level"))
                .unwrap_or(0);
            let backend = backend::Backend::spawn(cc.egui_ctx.clone());
            let boot_ms = t0.elapsed().as_millis();
            Ok(Box::new(app::App::new(backend, boot_ms, dark, font_level)))
        }),
    )
}
