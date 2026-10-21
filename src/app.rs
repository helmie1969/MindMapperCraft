//! Hauptfenster: Werkzeugleiste, Zeichenfläche, Tastatur-Workflow, Rückgängig und Dateien.

use crate::fonts::FontCatalog;
use crate::layout::{self, Layout};
use crate::model::{MindMap, NodeId};
use crate::render;
use egui::{vec2, Event, Key, KeyboardShortcut, Modifiers, Pos2, Rect, RichText, Sense, Vec2};
use std::path::{Path, PathBuf};

const FILE_EXTENSION: &str = "mmc";
const MAX_UNDO: usize = 200;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 4.0;

/// Tasten, die vor egui abgefangen werden, damit Tab/Enter nicht den Fokus wechseln.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyAction {
    Tab,
    Enter,
    Escape,
}

/// Ein Thema wird gerade direkt auf der Zeichenfläche bearbeitet.
pub(crate) struct EditState {
    pub id: NodeId,
    pub buffer: String,
    /// Text, der bei Abbruch (Esc) oder leerer Eingabe verwendet wird.
    pub fallback: String,
    pub just_started: bool,
}

pub struct MindMapApp {
    pub(crate) map: MindMap,
    pub(crate) selected: Option<NodeId>,
    pub(crate) editing: Option<EditState>,
    pub(crate) fonts: FontCatalog,
    /// Formatänderungen in der Seitenleiste auch auf alle Unterthemen übertragen.
    pub(crate) apply_to_branch: bool,
    /// Verhindert, dass jeder Frame eines Slider-Zugs einen eigenen Undo-Schritt erzeugt.
    style_undo_open: bool,
    pan: Vec2,
    zoom: f32,
    canvas_rect: Rect,
    undo_stack: Vec<MindMap>,
    redo_stack: Vec<MindMap>,
    pending_keys: Vec<KeyAction>,
    last_layout: Layout,
    scroll_to_selection: bool,
    file_path: Option<PathBuf>,
    dirty: bool,
    window_title: String,
    status: Option<String>,
}

fn edit_widget_id(id: NodeId) -> egui::Id {
    egui::Id::new(("mmc_topic_edit", id))
}

impl MindMapApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let fonts = FontCatalog::install(&cc.egui_ctx);
        let map = MindMap::sample();
        Self {
            selected: Some(map.root),
            map,
            editing: None,
            fonts,
            apply_to_branch: false,
            style_undo_open: false,
            pan: Vec2::ZERO,
            zoom: 1.0,
            canvas_rect: Rect::NOTHING,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            pending_keys: Vec::new(),
            last_layout: Layout::default(),
            scroll_to_selection: false,
            file_path: None,
            dirty: false,
            window_title: String::new(),
            status: None,
        }
    }

    // ---------------------------------------------------------------- Rückgängig

    pub(crate) fn push_undo(&mut self) {
        self.undo_stack.push(self.map.clone());
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
    }

    /// Ein Undo-Schritt für eine zusammenhängende Formatänderung (z. B. Slider ziehen).
    pub(crate) fn push_style_undo(&mut self) {
        if !self.style_undo_open {
            self.push_undo();
            self.style_undo_open = true;
        }
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_stack.pop() {
            self.redo_stack.push(std::mem::replace(&mut self.map, previous));
            self.after_map_swap();
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(std::mem::replace(&mut self.map, next));
            self.after_map_swap();
        }
    }

    fn after_map_swap(&mut self) {
        self.editing = None;
        self.dirty = true;
        if !self.selected.is_some_and(|id| self.map.nodes.contains_key(&id)) {
            self.selected = Some(self.map.root);
        }
    }

    // ---------------------------------------------------------------- Themen bearbeiten

    fn add_child(&mut self, ctx: &egui::Context, parent: NodeId) {
        self.finish_edit(ctx, false);
        if !self.map.nodes.contains_key(&parent) {
            return;
        }
        self.push_undo();
        let label = MindMap::default_label(self.map.depth(parent) + 1);
        let id = self.map.add_child(parent, label);
        self.select_new(id);
    }

    fn add_sibling(&mut self, ctx: &egui::Context, id: NodeId) {
        self.finish_edit(ctx, false);
        if id == self.map.root {
            // Das zentrale Thema hat keine Geschwister – wie bei Xmind entsteht ein Hauptthema.
            self.add_child(ctx, id);
            return;
        }
        self.push_undo();
        if let Some(new_id) = self.map.add_sibling(id) {
            self.select_new(new_id);
        }
    }

    /// Neues Thema auswählen und sofort in den Bearbeitungsmodus wechseln.
    fn select_new(&mut self, id: NodeId) {
        let label = self.map.nodes[&id].text.clone();
        self.selected = Some(id);
        self.scroll_to_selection = true;
        // Leerer Puffer: der Platzhalter zeigt den Standardtext, Tippen ersetzt ihn direkt.
        self.editing = Some(EditState {
            id,
            buffer: String::new(),
            fallback: label,
            just_started: true,
        });
    }

    fn begin_edit(&mut self, id: NodeId, initial: Option<String>) {
        let Some(node) = self.map.nodes.get(&id) else { return };
        let original = node.text.clone();
        self.push_undo();
        self.selected = Some(id);
        self.scroll_to_selection = true;
        self.editing = Some(EditState {
            id,
            buffer: initial.unwrap_or_else(|| original.clone()),
            fallback: original,
            just_started: true,
        });
    }

    /// Beendet die Bearbeitung. `cancel` stellt den vorherigen Text wieder her.
    fn finish_edit(&mut self, ctx: &egui::Context, cancel: bool) {
        let Some(edit) = self.editing.take() else { return };
        ctx.memory_mut(|m| m.surrender_focus(edit_widget_id(edit.id)));
        let typed = edit.buffer.trim();
        let text = if cancel || typed.is_empty() {
            edit.fallback
        } else {
            typed.to_owned()
        };
        if let Some(node) = self.map.nodes.get_mut(&edit.id) {
            node.text = text;
        }
    }

    fn delete_selected(&mut self, ctx: &egui::Context) {
        self.finish_edit(ctx, true);
        let Some(id) = self.selected else { return };
        if id == self.map.root {
            self.status = Some("Das zentrale Thema kann nicht gelöscht werden.".into());
            return;
        }
        self.push_undo();
        self.selected = self.map.remove(id);
    }

    /// Auswahl per Pfeiltaste: nächstgelegenes Thema in der gewünschten Richtung.
    fn navigate(&mut self, dir: Vec2) {
        let Some(current) = self.selected.and_then(|id| self.last_layout.rects.get(&id)) else {
            self.selected = Some(self.map.root);
            return;
        };
        let from = current.center();
        let best = self
            .last_layout
            .rects
            .iter()
            .filter(|(id, _)| Some(**id) != self.selected)
            .filter_map(|(id, rect)| {
                let d = rect.center() - from;
                let along = d.dot(dir);
                if along <= 1.0 {
                    return None;
                }
                let across = (d - dir * along).length();
                Some((*id, along + across * 2.0))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((id, _)) = best {
            self.selected = Some(id);
            self.scroll_to_selection = true;
        }
    }

    // ---------------------------------------------------------------- Tastatur

    fn process_key_actions(&mut self, ctx: &egui::Context) {
        for action in std::mem::take(&mut self.pending_keys) {
            match action {
                KeyAction::Tab => {
                    let target = self.editing.as_ref().map(|e| e.id).or(self.selected);
                    if let Some(id) = target {
                        self.add_child(ctx, id);
                    }
                }
                KeyAction::Enter => {
                    if self.editing.is_some() {
                        self.finish_edit(ctx, false);
                    } else if let Some(id) = self.selected {
                        self.add_sibling(ctx, id);
                    }
                }
                KeyAction::Escape => self.finish_edit(ctx, true),
            }
        }
    }

    fn text_field_focused(ctx: &egui::Context) -> bool {
        let focused = ctx.memory(|m| m.focused());
        focused.is_some_and(|id| egui::TextEdit::load_state(ctx, id).is_some())
    }

    /// Tastenkürzel, die nur gelten, wenn kein Textfeld aktiv ist.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.editing.is_some() || Self::text_field_focused(ctx) {
            return;
        }

        let shortcut = |mods: Modifiers, key: Key| {
            ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(mods, key)))
        };
        if shortcut(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z) || shortcut(Modifiers::COMMAND, Key::Y) {
            self.redo();
        }
        if shortcut(Modifiers::COMMAND, Key::Z) {
            self.undo();
        }
        if shortcut(Modifiers::COMMAND | Modifiers::SHIFT, Key::S) {
            self.save_as();
        }
        if shortcut(Modifiers::COMMAND, Key::S) {
            self.save();
        }
        if shortcut(Modifiers::COMMAND, Key::O) {
            self.open_file();
        }
        if shortcut(Modifiers::COMMAND, Key::N) {
            self.new_map();
        }

        let (delete, f2, insert, typed) = ctx.input(|i| {
            let typed = i.events.iter().find_map(|e| match e {
                Event::Text(t) => Some(t.clone()),
                _ => None,
            });
            (
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::F2),
                i.key_pressed(Key::Insert),
                typed,
            )
        });
        let arrows = ctx.input(|i| {
            [
                (Key::ArrowLeft, vec2(-1.0, 0.0)),
                (Key::ArrowRight, vec2(1.0, 0.0)),
                (Key::ArrowUp, vec2(0.0, -1.0)),
                (Key::ArrowDown, vec2(0.0, 1.0)),
            ]
            .into_iter()
            .find(|(k, _)| i.key_pressed(*k))
            .map(|(_, d)| d)
        });

        if delete {
            self.delete_selected(ctx);
        } else if let Some(dir) = arrows {
            self.navigate(dir);
        } else if let Some(id) = self.selected {
            if insert {
                self.add_child(ctx, id);
            } else if f2 {
                self.begin_edit(id, None);
            } else if let Some(text) = typed {
                // Leertaste bearbeitet den vorhandenen Text, jedes andere Zeichen ersetzt ihn (wie Xmind).
                if text.trim().is_empty() {
                    self.begin_edit(id, None);
                } else {
                    self.begin_edit(id, Some(text));
                }
            }
        }
    }

    // ---------------------------------------------------------------- Ansicht

    fn zoom_at(&mut self, pointer: Pos2, factor: f32) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let origin = self.canvas_rect.center() + self.pan;
        let world = (pointer - origin) / self.zoom;
        self.pan = pointer - world * new_zoom - self.canvas_rect.center();
        self.zoom = new_zoom;
    }

    fn center_map(&mut self) {
        if let Some(bounds) = self.last_layout.bounds() {
            self.pan = -bounds.center().to_vec2() * self.zoom;
        }
    }

    // ---------------------------------------------------------------- Dateien

    fn new_map(&mut self) {
        self.map = MindMap::sample();
        self.selected = Some(self.map.root);
        self.reset_document_state(None);
    }

    fn reset_document_state(&mut self, path: Option<PathBuf>) {
        self.editing = None;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.file_path = path;
        self.dirty = false;
        self.pan = Vec2::ZERO;
        self.zoom = 1.0;
    }

    fn open_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("MindMapperCraft", &[FILE_EXTENSION])
            .pick_file()
        else {
            return;
        };
        let result = std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|s| serde_json::from_str::<MindMap>(&s).map_err(|e| e.to_string()))
            .and_then(|m| {
                if m.is_valid() {
                    Ok(m)
                } else {
                    Err("Datei enthält eine ungültige Struktur".into())
                }
            });
        match result {
            Ok(map) => {
                self.map = map;
                self.selected = Some(self.map.root);
                self.status = Some(format!("Geöffnet: {}", path.display()));
                self.reset_document_state(Some(path));
            }
            Err(e) => self.status = Some(format!("Fehler beim Öffnen: {e}")),
        }
    }

    fn save(&mut self) {
        match self.file_path.clone() {
            Some(path) => self.write_file(&path),
            None => self.save_as(),
        }
    }

    fn save_as(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("MindMapperCraft", &[FILE_EXTENSION])
            .set_file_name(format!("mindmap.{FILE_EXTENSION}"))
            .save_file()
        {
            self.write_file(&path);
        }
    }

    fn write_file(&mut self, path: &Path) {
        let result = serde_json::to_string_pretty(&self.map)
            .map_err(|e| e.to_string())
            .and_then(|json| std::fs::write(path, json).map_err(|e| e.to_string()));
        match result {
            Ok(()) => {
                self.file_path = Some(path.to_path_buf());
                self.dirty = false;
                self.status = Some(format!("Gespeichert: {}", path.display()));
            }
            Err(e) => self.status = Some(format!("Fehler beim Speichern: {e}")),
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let name = self
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unbenannt".into());
        let title = format!("{}{} – MindMapperCraft", name, if self.dirty { " *" } else { "" });
        if title != self.window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.window_title = title;
        }
    }

    // ---------------------------------------------------------------- Oberfläche

    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        egui::menu::bar(ui, |ui| {
            ui.menu_button("Datei", |ui| {
                if ui.button("Neu            Strg+N").clicked() {
                    ui.close_menu();
                    self.new_map();
                }
                if ui.button("Öffnen …       Strg+O").clicked() {
                    ui.close_menu();
                    self.open_file();
                }
                if ui.button("Speichern      Strg+S").clicked() {
                    ui.close_menu();
                    self.save();
                }
                if ui.button("Speichern unter …").clicked() {
                    ui.close_menu();
                    self.save_as();
                }
                ui.separator();
                if ui.button("Beenden").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("Bearbeiten", |ui| {
                if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Rückgängig    Strg+Z")).clicked() {
                    ui.close_menu();
                    self.undo();
                }
                if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Wiederholen   Strg+Y")).clicked() {
                    ui.close_menu();
                    self.redo();
                }
            });
            ui.menu_button("Ansicht", |ui| {
                let center = self.canvas_rect.center();
                if ui.button("Vergrößern").clicked() {
                    self.zoom_at(center, 1.25);
                }
                if ui.button("Verkleinern").clicked() {
                    self.zoom_at(center, 0.8);
                }
                if ui.button("Originalgröße (100 %)").clicked() {
                    self.zoom_at(center, 1.0 / self.zoom);
                }
                if ui.button("Karte zentrieren").clicked() {
                    ui.close_menu();
                    self.center_map();
                }
            });

            ui.separator();

            let selected = self.selected;
            let has_selection = selected.is_some();
            if ui
                .add_enabled(has_selection, egui::Button::new("Unterthema"))
                .on_hover_text("Unterthema einfügen (Tab)")
                .clicked()
            {
                self.add_child(&ctx, selected.unwrap());
            }
            if ui
                .add_enabled(has_selection, egui::Button::new("Thema"))
                .on_hover_text("Thema auf gleicher Ebene einfügen (Enter)")
                .clicked()
            {
                self.add_sibling(&ctx, selected.unwrap());
            }
            if ui
                .add_enabled(selected.is_some_and(|id| id != self.map.root), egui::Button::new("Löschen"))
                .on_hover_text("Thema mit allen Unterthemen löschen (Entf)")
                .clicked()
            {
                self.delete_selected(&ctx);
            }

            ui.separator();

            if ui.add_enabled(!self.undo_stack.is_empty(), egui::Button::new("Rückgängig")).clicked() {
                self.undo();
            }
            if ui.add_enabled(!self.redo_stack.is_empty(), egui::Button::new("Wiederholen")).clicked() {
                self.redo();
            }

            ui.separator();

            let center = self.canvas_rect.center();
            if ui.button(" − ").on_hover_text("Verkleinern").clicked() {
                self.zoom_at(center, 0.8);
            }
            if ui.button(format!("{:.0} %", self.zoom * 100.0)).on_hover_text("Auf 100 % setzen").clicked() {
                self.zoom_at(center, 1.0 / self.zoom);
            }
            if ui.button(" + ").on_hover_text("Vergrößern").clicked() {
                self.zoom_at(center, 1.25);
            }
            if ui.button("Zentrieren").clicked() {
                self.center_map();
            }
        });
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(
                    "Tab: Unterthema  ·  Enter: Thema  ·  F2 / Doppelklick / Tippen: Text  ·  \
                     Entf: Löschen  ·  Pfeiltasten: Auswahl  ·  Strg+Mausrad: Zoom  ·  Ziehen: Verschieben",
                )
                .small()
                .weak(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(status) = &self.status {
                    ui.label(RichText::new(status).small());
                }
            });
        });
    }

    fn canvas(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        self.canvas_rect = rect;

        // Zoom (Strg+Mausrad) und Verschieben (Mausrad / Ziehen).
        if response.hovered() {
            let (zoom_delta, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
            if zoom_delta != 1.0 {
                if let Some(pointer) = response.hover_pos() {
                    self.zoom_at(pointer, zoom_delta);
                }
            } else {
                self.pan += scroll;
            }
        }
        if response.dragged() {
            self.pan += response.drag_delta();
        }

        // Während der Bearbeitung wächst das Thema live mit dem eingegebenen Text.
        if let Some(edit) = &self.editing {
            if let Some(node) = self.map.nodes.get_mut(&edit.id) {
                if node.text != edit.buffer {
                    node.text.clone_from(&edit.buffer);
                }
            }
        }

        let layout = layout::compute(&self.map, &ctx, &self.fonts);
        let origin = rect.center() + self.pan;
        let zoom = self.zoom;
        let to_screen = move |r: Rect| {
            Rect::from_min_max(origin + r.min.to_vec2() * zoom, origin + r.max.to_vec2() * zoom)
        };

        let painter = ui.painter_at(rect);

        for edge in &layout.edges {
            let (Some(from), Some(to)) = (layout.rects.get(&edge.from), layout.rects.get(&edge.to)) else {
                continue;
            };
            let from_node = &self.map.nodes[&edge.from];
            let to_node = &self.map.nodes[&edge.to];
            render::draw_edge(
                &painter,
                edge.kind,
                to_screen(*from),
                from_node.style.shape,
                to_screen(*to),
                to_node.style.shape,
                &to_node.style,
                zoom,
            );
        }

        let editing_id = self.editing.as_ref().map(|e| e.id);
        for (id, node) in &self.map.nodes {
            let Some(world) = layout.rects.get(id) else { continue };
            let screen = to_screen(*world);
            if !rect.expand(40.0).intersects(screen) {
                continue;
            }
            let galley = (editing_id != Some(*id)).then(|| {
                painter.layout_no_wrap(
                    node.text.clone(),
                    self.fonts.font_id(&node.style, zoom),
                    node.style.text_color,
                )
            });
            render::draw_node(&painter, screen, &node.style, galley, zoom, self.selected == Some(*id));
        }

        // Mausinteraktion: Klick wählt aus, Doppelklick bearbeitet.
        let hit = |pos: Pos2| {
            layout
                .rects
                .iter()
                .find(|(_, r)| to_screen(**r).contains(pos))
                .map(|(id, _)| *id)
        };
        if response.clicked() {
            if let Some(focused) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(focused));
            }
            let target = response.interact_pointer_pos().and_then(hit);
            if editing_id.is_some() && editing_id != target {
                self.finish_edit(&ctx, false);
            }
            self.selected = target;
        }
        if response.double_clicked() {
            if let Some(id) = response.interact_pointer_pos().and_then(hit) {
                if self.editing.is_none() {
                    self.begin_edit(id, None);
                }
            }
        }

        // Textfeld direkt über dem Thema.
        let mut lost_focus = false;
        if let Some(edit) = &mut self.editing {
            if let (Some(world), Some(node)) = (layout.rects.get(&edit.id), self.map.nodes.get(&edit.id)) {
                let screen = to_screen(*world);
                let inner = screen.shrink2(layout::padding(node.style.shape) * zoom);
                let text_edit = egui::TextEdit::singleline(&mut edit.buffer)
                    .id(edit_widget_id(edit.id))
                    .font(self.fonts.font_id(&node.style, zoom))
                    .text_color(node.style.text_color)
                    .hint_text(edit.fallback.clone())
                    .frame(false)
                    .margin(vec2(0.0, 0.0))
                    .desired_width(inner.width().max(60.0 * zoom));
                let resp = ui.put(inner.expand2(vec2(2.0, 0.0)), text_edit);
                if edit.just_started {
                    resp.request_focus();
                    edit.just_started = false;
                } else if resp.lost_focus() {
                    lost_focus = true;
                }
            }
        }
        if lost_focus {
            self.finish_edit(&ctx, false);
        }

        // Neu angelegte oder per Tastatur gewählte Themen in den sichtbaren Bereich holen.
        if self.scroll_to_selection {
            self.scroll_to_selection = false;
            if let Some(world) = self.selected.and_then(|id| layout.rects.get(&id)) {
                let r = to_screen(*world);
                let view = rect.shrink(40.0);
                let mut delta = Vec2::ZERO;
                if r.left() < view.left() {
                    delta.x = view.left() - r.left();
                } else if r.right() > view.right() {
                    delta.x = view.right() - r.right();
                }
                if r.top() < view.top() {
                    delta.y = view.top() - r.top();
                } else if r.bottom() > view.bottom() {
                    delta.y = view.bottom() - r.bottom();
                }
                if delta != Vec2::ZERO {
                    self.pan += delta;
                    ctx.request_repaint();
                }
            }
        }

        self.last_layout = layout;
    }
}

impl eframe::App for MindMapApp {
    /// Tab, Enter und Esc werden abgefangen, bevor egui sie für die Fokus-Navigation nutzt.
    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        let editing = self.editing.is_some();
        if !editing && Self::text_field_focused(ctx) {
            // Ein Eingabefeld in der Seitenleiste ist aktiv – Tasten normal durchlassen.
            return;
        }
        let pending = &mut self.pending_keys;
        raw_input.events.retain(|event| {
            let Event::Key { key, pressed, modifiers, .. } = event else {
                return true;
            };
            let action = match key {
                Key::Tab if !modifiers.shift => Some(KeyAction::Tab),
                Key::Enter if !modifiers.shift && !modifiers.command => Some(KeyAction::Enter),
                Key::Escape if editing => Some(KeyAction::Escape),
                _ => None,
            };
            match action {
                Some(action) => {
                    if *pressed {
                        pending.push(action);
                    }
                    false
                }
                None => true,
            }
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_key_actions(ctx);
        self.handle_shortcuts(ctx);

        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| self.toolbar(ui));
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| self.status_bar(ui));
        egui::SidePanel::right("format_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(250.0)
            .show(ctx, |ui| self.sidebar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(self.map.background))
            .show(ctx, |ui| self.canvas(ui));

        if !ctx.input(|i| i.pointer.any_down()) {
            self.style_undo_open = false;
        }
        self.update_title(ctx);
    }
}
