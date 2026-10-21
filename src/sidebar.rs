//! Rechte Seitenleiste: MindMap-Typ wählen und das ausgewählte Thema formatieren.

use crate::app::MindMapApp;
use crate::model::{BorderShape, LineStyle, MapKind};
use crate::render;
use egui::{vec2, Align2, CollapsingHeader, ComboBox, FontId, Grid, Rect, RichText, Sense, Slider, Stroke};

impl MindMapApp {
    pub(crate) fn sidebar(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(6.0);
            ui.heading("Format");
            ui.add_space(4.0);

            CollapsingHeader::new(RichText::new("Struktur").strong())
                .default_open(true)
                .show(ui, |ui| self.structure_picker(ui));

            ui.separator();
            self.topic_format(ui);

            ui.separator();
            CollapsingHeader::new(RichText::new("Karte").strong())
                .default_open(false)
                .show(ui, |ui| self.map_settings(ui));
        });
    }

    /// Kacheln mit Vorschaubild für jeden MindMap-Typ.
    fn structure_picker(&mut self, ui: &mut egui::Ui) {
        let tile = vec2(124.0, 74.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for kind in MapKind::ALL {
                let selected = self.map.kind == kind;
                let (rect, response) = ui.allocate_exact_size(tile, Sense::click());
                let visuals = ui.visuals().clone();

                let (bg, border) = if selected {
                    (visuals.selection.bg_fill.gamma_multiply(0.35), Stroke::new(2.0_f32, render::SELECTION_COLOR))
                } else if response.hovered() {
                    (visuals.widgets.hovered.weak_bg_fill, visuals.widgets.hovered.bg_stroke)
                } else {
                    (visuals.extreme_bg_color, visuals.widgets.noninteractive.bg_stroke)
                };
                let painter = ui.painter();
                painter.rect(rect, 6.0, bg, border);

                let thumb = Rect::from_min_size(rect.min + vec2(10.0, 8.0), vec2(rect.width() - 20.0, 40.0));
                let color = if selected { render::SELECTION_COLOR } else { visuals.text_color() };
                render::draw_thumbnail(painter, thumb, kind, color);
                painter.text(
                    rect.center_bottom() - vec2(0.0, 12.0),
                    Align2::CENTER_CENTER,
                    kind.label(),
                    FontId::proportional(12.5),
                    visuals.text_color(),
                );

                let response = response.on_hover_text(kind.description());
                if response.clicked() && !selected {
                    self.push_undo();
                    self.map.kind = kind;
                }
            }
        });
    }

    /// Formatierung des ausgewählten Themas: Rahmen, Schrift, Verbindungslinie.
    fn topic_format(&mut self, ui: &mut egui::Ui) {
        let Some(id) = self.selected.filter(|id| self.map.nodes.contains_key(id)) else {
            ui.add_space(4.0);
            ui.label(RichText::new("Kein Thema ausgewählt").strong());
            ui.label(RichText::new("Klicke auf ein Thema, um Rahmen, Schrift und Verbindungslinie zu formatieren.").weak());
            return;
        };

        let is_root = id == self.map.root;
        let node = &self.map.nodes[&id];
        let before = node.style.clone();
        let mut style = before.clone();

        let mut title: String = node.text.chars().take(32).collect();
        if node.text.chars().count() > 32 {
            title.push('…');
        }
        ui.add_space(4.0);
        ui.label(RichText::new(format!("Thema: {title}")).strong());
        ui.checkbox(&mut self.apply_to_branch, "Auch auf alle Unterthemen anwenden");
        ui.add_space(4.0);

        CollapsingHeader::new(RichText::new("Rahmen").strong())
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("format_border").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Form");
                    ComboBox::new("border_shape", "")
                        .selected_text(style.shape.label())
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for shape in BorderShape::ALL {
                                ui.selectable_value(&mut style.shape, shape, shape.label());
                            }
                        });
                    ui.end_row();

                    ui.label("Füllfarbe");
                    ui.color_edit_button_srgba(&mut style.fill);
                    ui.end_row();

                    ui.label("Rahmenfarbe");
                    ui.color_edit_button_srgba(&mut style.border_color);
                    ui.end_row();

                    ui.label("Rahmenstärke");
                    ui.add(Slider::new(&mut style.border_width, 0.0..=8.0).step_by(0.5).suffix(" px"));
                    ui.end_row();
                });
            });

        let fonts = &self.fonts;
        CollapsingHeader::new(RichText::new("Schrift").strong())
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("format_font").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Schriftart");
                    let current = if fonts.names.contains(&style.font_family) {
                        style.font_family.clone()
                    } else {
                        "Standard".to_owned()
                    };
                    ComboBox::new("font_family", "")
                        .selected_text(current)
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut style.font_family, String::new(), "Standard");
                            for name in &fonts.names {
                                ui.selectable_value(&mut style.font_family, name.clone(), name.as_str());
                            }
                        });
                    ui.end_row();

                    ui.label("Größe");
                    ui.add(Slider::new(&mut style.font_size, 8.0..=72.0).step_by(1.0).suffix(" pt"));
                    ui.end_row();

                    ui.label("Schnitt");
                    ui.add_enabled(fonts.has_bold(&style.font_family), egui::Checkbox::new(&mut style.bold, "Fett"));
                    ui.end_row();

                    ui.label("Textfarbe");
                    ui.color_edit_button_srgba(&mut style.text_color);
                    ui.end_row();
                });
            });

        CollapsingHeader::new(RichText::new("Verbindungslinie").strong())
            .default_open(true)
            .show(ui, |ui| {
                if is_root && !self.apply_to_branch {
                    ui.label(
                        RichText::new(
                            "Das zentrale Thema hat keine eingehende Linie. Aktiviere \
                             „Auch auf alle Unterthemen anwenden“, um alle Linien zu ändern.",
                        )
                        .weak()
                        .small(),
                    );
                }
                Grid::new("format_line").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                    ui.label("Linienform");
                    ComboBox::new("line_style", "")
                        .selected_text(style.line_style.label())
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for line in LineStyle::ALL {
                                ui.selectable_value(&mut style.line_style, line, line.label());
                            }
                        });
                    ui.end_row();

                    ui.label("Linienfarbe");
                    ui.color_edit_button_srgba(&mut style.line_color);
                    ui.end_row();

                    ui.label("Linienstärke");
                    ui.add(Slider::new(&mut style.line_width, 0.0..=10.0).step_by(0.5).suffix(" px"));
                    ui.end_row();
                });
            });

        ui.add_space(6.0);
        if ui.button("Standardformat wiederherstellen").clicked() {
            style = self.map.default_style(id);
        }

        if style != before {
            self.push_style_undo();
            let descendants = if self.apply_to_branch {
                self.map.descendants(id)
            } else {
                Vec::new()
            };
            for d in descendants {
                if let Some(n) = self.map.nodes.get_mut(&d) {
                    n.style.apply_changes(&before, &style);
                }
            }
            self.map.nodes.get_mut(&id).unwrap().style = style;
        }
    }

    fn map_settings(&mut self, ui: &mut egui::Ui) {
        Grid::new("format_map").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Hintergrund");
            let mut background = self.map.background;
            if ui.color_edit_button_srgba(&mut background).changed() {
                self.push_style_undo();
                self.map.background = background;
            }
            ui.end_row();

            ui.label("Themen");
            ui.label(self.map.nodes.len().to_string());
            ui.end_row();
        });
        ui.add_space(4.0);
        if ui.button("Alle Formate zurücksetzen").clicked() {
            self.push_undo();
            self.map.reset_styles();
        }
    }
}
