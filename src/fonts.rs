//! Lädt Systemschriften, damit in der Seitenleiste echte Schriftarten wählbar sind.

use crate::model::NodeStyle;
use egui::{FontData, FontDefinitions, FontFamily, FontId};
use std::collections::HashSet;
use std::path::PathBuf;

/// Standardschrift für neue Themen (fällt auf die egui-Schrift zurück, falls nicht installiert).
pub const DEFAULT_FONT: &str = "Segoe UI";

/// (Anzeigename, Datei normal, Datei fett) – Dateien im Windows-Schriftenordner.
const CANDIDATES: &[(&str, &str, &str)] = &[
    ("Segoe UI", "segoeui.ttf", "segoeuib.ttf"),
    ("Arial", "arial.ttf", "arialbd.ttf"),
    ("Calibri", "calibri.ttf", "calibrib.ttf"),
    ("Verdana", "verdana.ttf", "verdanab.ttf"),
    ("Georgia", "georgia.ttf", "georgiab.ttf"),
    ("Times New Roman", "times.ttf", "timesbd.ttf"),
    ("Consolas", "consola.ttf", "consolab.ttf"),
    ("Comic Sans MS", "comic.ttf", "comicbd.ttf"),
];

pub struct FontCatalog {
    /// Namen der erfolgreich geladenen Schriftfamilien.
    pub names: Vec<String>,
    bold: HashSet<String>,
}

impl FontCatalog {
    pub fn install(ctx: &egui::Context) -> Self {
        let dir = std::env::var_os("WINDIR")
            .map(|w| PathBuf::from(w).join("Fonts"))
            .unwrap_or_else(|| PathBuf::from(r"C:\Windows\Fonts"));

        let mut defs = FontDefinitions::default();
        // Fehlende Zeichen (z. B. Emojis) fallen auf die eingebauten egui-Schriften zurück.
        let fallback = defs.families[&FontFamily::Proportional].clone();

        let mut names = Vec::new();
        let mut bold = HashSet::new();

        let register = |defs: &mut FontDefinitions, key: String, file: &str| -> bool {
            match std::fs::read(dir.join(file)) {
                Ok(bytes) => {
                    defs.font_data.insert(key.clone(), FontData::from_owned(bytes));
                    let mut family = vec![key.clone()];
                    family.extend(fallback.iter().cloned());
                    defs.families.insert(FontFamily::Name(key.into()), family);
                    true
                }
                Err(_) => false,
            }
        };

        for (name, regular, bold_file) in CANDIDATES {
            if register(&mut defs, (*name).to_owned(), regular) {
                names.push((*name).to_owned());
                if register(&mut defs, format!("{name} Bold"), bold_file) {
                    bold.insert((*name).to_owned());
                }
            }
        }

        // Die Bedienoberfläche selbst ebenfalls in Segoe UI darstellen, falls vorhanden.
        if names.iter().any(|n| n == DEFAULT_FONT) {
            defs.families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, DEFAULT_FONT.to_owned());
        }

        ctx.set_fonts(defs);
        FontCatalog { names, bold }
    }

    pub fn has_bold(&self, name: &str) -> bool {
        self.bold.contains(name)
    }

    /// Löst einen Schriftnamen auf. Unbekannte Namen (z. B. aus einer Datei von einem anderen
    /// Rechner) werden auf die Standardschrift abgebildet, da egui bei unbekannten Familien abbricht.
    pub fn family(&self, name: &str, bold: bool) -> FontFamily {
        if !self.names.iter().any(|n| n == name) {
            return FontFamily::Proportional;
        }
        if bold && self.has_bold(name) {
            FontFamily::Name(format!("{name} Bold").into())
        } else {
            FontFamily::Name(name.into())
        }
    }

    pub fn font_id(&self, style: &NodeStyle, zoom: f32) -> FontId {
        FontId::new(
            (style.font_size * zoom).max(1.0),
            self.family(&style.font_family, style.bold),
        )
    }
}
