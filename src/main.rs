mod app;
mod i18n;
mod snap;
mod x11_helper;
mod xrandr;

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use eframe::egui;

fn main() -> eframe::Result<()> {
    // Hintergrund-Thread der die Trennlinien-Overlays aus Cinnamon-Taskbar/Pager entfernt
    let stop_hider = Arc::new(AtomicBool::new(false));
    x11_helper::spawn_taskbar_hider("parting-divider".to_string(), stop_hider.clone());

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([980.0, 640.0])
        .with_min_inner_size([700.0, 480.0])
        .with_title("Parting — Virtual Monitor Splitter");
    if let Some(icon) = load_icon() {
        viewport = viewport.with_icon(icon);
    }

    let opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    let result = eframe::run_native(
        "Parting",
        opts,
        Box::new(|_cc| Ok(Box::new(app::PartingApp::new()))),
    );

    stop_hider.store(true, std::sync::atomic::Ordering::Relaxed);
    result
}

fn load_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}