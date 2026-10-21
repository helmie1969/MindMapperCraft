// Im Release-Build kein Konsolenfenster unter Windows öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod fonts;
mod layout;
mod model;
mod render;
mod sidebar;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("MindMapperCraft")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "MindMapperCraft",
        options,
        Box::new(|cc| Ok(Box::new(app::MindMapApp::new(cc)))),
    )
}
