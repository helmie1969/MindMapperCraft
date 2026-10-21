//! Layout-Algorithmen für die verschiedenen MindMap-Typen.
//!
//! Alle Positionen werden in Weltkoordinaten berechnet (zentrales Thema im Ursprung,
//! Zoomstufe 1). Der Canvas rechnet sie anschließend in Bildschirmkoordinaten um.

use crate::fonts::FontCatalog;
use crate::model::{BorderShape, MapKind, MindMap, NodeId};
use egui::{pos2, vec2, Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;

/// Horizontaler Abstand zwischen den Ebenen (Mindmap, Logikdiagramm).
const H_GAP: f32 = 44.0;
/// Vertikaler Abstand zwischen Geschwistern (Mindmap, Logikdiagramm).
const V_GAP: f32 = 12.0;
/// Abstände im Organigramm.
const ORG_H_GAP: f32 = 18.0;
const ORG_V_GAP: f32 = 44.0;
/// Einrückung und Zeilenabstand im Baumdiagramm.
pub const TREE_INDENT: f32 = 30.0;
const TREE_V_GAP: f32 = 10.0;
/// Abstand der Hauptthemen auf der Zeitleiste.
const TIMELINE_GAP: f32 = 48.0;
const MIN_WIDTH: f32 = 48.0;

/// Wie eine Verbindungslinie an die beiden Themen andockt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// Seitlich (links/rechts) – Mindmap und Logikdiagramm.
    Horizontal,
    /// Unten → oben – Organigramm.
    Vertical,
    /// Vom eingerückten Punkt unter dem Elternthema zur linken Seite des Kindes – Baumdiagramm.
    Tree,
    /// Achsensegment zwischen aufeinanderfolgenden Hauptthemen – Zeitleiste.
    Timeline,
}

#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

#[derive(Default)]
pub struct Layout {
    pub rects: HashMap<NodeId, Rect>,
    pub edges: Vec<Edge>,
}

impl Layout {
    pub fn bounds(&self) -> Option<Rect> {
        self.rects.values().copied().reduce(|a, b| a.union(b))
    }
}

/// Innenabstand zwischen Text und Rahmen.
pub fn padding(shape: BorderShape) -> Vec2 {
    match shape {
        BorderShape::Rectangle | BorderShape::Rounded => vec2(14.0, 8.0),
        BorderShape::Capsule => vec2(20.0, 8.0),
        BorderShape::Ellipse => vec2(26.0, 14.0),
        BorderShape::Underline | BorderShape::None => vec2(6.0, 4.0),
    }
}

pub fn compute(map: &MindMap, ctx: &egui::Context, fonts: &FontCatalog) -> Layout {
    let mut b = Builder {
        map,
        sizes: measure(map, ctx, fonts),
        heights: HashMap::new(),
        widths: HashMap::new(),
        out: Layout::default(),
    };

    let root = map.root;
    let root_rect = Rect::from_center_size(Pos2::ZERO, b.sizes[&root]);
    b.out.rects.insert(root, root_rect);
    let kids = &map.nodes[&root].children;

    match map.kind {
        MapKind::MindMap => {
            // Im Uhrzeigersinn: erste Hälfte rechts (oben → unten), Rest links (unten → oben).
            let split = kids.len().div_ceil(2);
            let right = &kids[..split];
            let left: Vec<NodeId> = kids[split..].iter().rev().copied().collect();
            b.children_horizontal(root, root_rect, right, 1.0);
            b.children_horizontal(root, root_rect, &left, -1.0);
        }
        MapKind::LogicRight => b.children_horizontal(root, root_rect, kids, 1.0),
        MapKind::LogicLeft => b.children_horizontal(root, root_rect, kids, -1.0),
        MapKind::OrgChart => b.children_vertical(root, root_rect, kids),
        MapKind::TreeChart => {
            b.children_tree(root, root_rect, kids);
        }
        MapKind::Timeline => b.timeline(root, root_rect, kids),
    }

    b.out
}

/// Größe jedes Themas in Weltkoordinaten (Text + Innenabstand).
fn measure(map: &MindMap, ctx: &egui::Context, fonts: &FontCatalog) -> HashMap<NodeId, Vec2> {
    ctx.fonts(|f| {
        map.nodes
            .values()
            .map(|node| {
                let text = if node.text.is_empty() { " " } else { node.text.as_str() };
                let galley = f.layout_no_wrap(
                    text.to_owned(),
                    fonts.font_id(&node.style, 1.0),
                    Color32::WHITE,
                );
                let mut size = galley.size() + padding(node.style.shape) * 2.0;
                size.x = size.x.max(MIN_WIDTH);
                (node.id, size)
            })
            .collect()
    })
}

struct Builder<'a> {
    map: &'a MindMap,
    sizes: HashMap<NodeId, Vec2>,
    /// Höhe der Teilbäume (für horizontale Layouts).
    heights: HashMap<NodeId, f32>,
    /// Breite der Teilbäume (für das Organigramm).
    widths: HashMap<NodeId, f32>,
    out: Layout,
}

impl Builder<'_> {
    fn place(&mut self, id: NodeId, rect: Rect) {
        self.out.rects.insert(id, rect);
    }

    fn edge(&mut self, from: NodeId, to: NodeId, kind: EdgeKind) {
        self.out.edges.push(Edge { from, to, kind });
    }

    // ---------- Mindmap / Logikdiagramm ----------

    fn subtree_height(&mut self, id: NodeId) -> f32 {
        if let Some(h) = self.heights.get(&id) {
            return *h;
        }
        let map = self.map;
        let stacked = self.stack_height(&map.nodes[&id].children);
        let h = self.sizes[&id].y.max(stacked);
        self.heights.insert(id, h);
        h
    }

    fn stack_height(&mut self, kids: &[NodeId]) -> f32 {
        if kids.is_empty() {
            return 0.0;
        }
        let sum: f32 = kids.iter().map(|c| self.subtree_height(*c)).sum();
        sum + V_GAP * (kids.len() - 1) as f32
    }

    /// Ordnet `kids` neben `parent_rect` an. `dir` = 1 für rechts, -1 für links.
    fn children_horizontal(&mut self, parent: NodeId, parent_rect: Rect, kids: &[NodeId], dir: f32) {
        let map = self.map;
        let mut y = parent_rect.center().y - self.stack_height(kids) / 2.0;
        let x = if dir > 0.0 {
            parent_rect.right() + H_GAP
        } else {
            parent_rect.left() - H_GAP
        };

        for &child in kids {
            let h = self.subtree_height(child);
            let size = self.sizes[&child];
            let top = y + (h - size.y) / 2.0;
            let rect = if dir > 0.0 {
                Rect::from_min_size(pos2(x, top), size)
            } else {
                Rect::from_min_size(pos2(x - size.x, top), size)
            };
            self.place(child, rect);
            self.edge(parent, child, EdgeKind::Horizontal);
            self.children_horizontal(child, rect, &map.nodes[&child].children, dir);
            y += h + V_GAP;
        }
    }

    // ---------- Organigramm ----------

    fn subtree_width(&mut self, id: NodeId) -> f32 {
        if let Some(w) = self.widths.get(&id) {
            return *w;
        }
        let map = self.map;
        let row = self.row_width(&map.nodes[&id].children);
        let w = self.sizes[&id].x.max(row);
        self.widths.insert(id, w);
        w
    }

    fn row_width(&mut self, kids: &[NodeId]) -> f32 {
        if kids.is_empty() {
            return 0.0;
        }
        let sum: f32 = kids.iter().map(|c| self.subtree_width(*c)).sum();
        sum + ORG_H_GAP * (kids.len() - 1) as f32
    }

    fn children_vertical(&mut self, parent: NodeId, parent_rect: Rect, kids: &[NodeId]) {
        let map = self.map;
        let mut x = parent_rect.center().x - self.row_width(kids) / 2.0;
        let y = parent_rect.bottom() + ORG_V_GAP;

        for &child in kids {
            let w = self.subtree_width(child);
            let size = self.sizes[&child];
            let rect = Rect::from_min_size(pos2(x + (w - size.x) / 2.0, y), size);
            self.place(child, rect);
            self.edge(parent, child, EdgeKind::Vertical);
            self.children_vertical(child, rect, &map.nodes[&child].children);
            x += w + ORG_H_GAP;
        }
    }

    // ---------- Baumdiagramm ----------

    /// Ordnet `kids` eingerückt untereinander an und liefert die Unterkante des Teilbaums.
    fn children_tree(&mut self, parent: NodeId, parent_rect: Rect, kids: &[NodeId]) -> f32 {
        let map = self.map;
        let x = parent_rect.left() + TREE_INDENT;
        let mut y = parent_rect.bottom() + TREE_V_GAP;
        let mut bottom = parent_rect.bottom();

        for &child in kids {
            let rect = Rect::from_min_size(pos2(x, y), self.sizes[&child]);
            self.place(child, rect);
            self.edge(parent, child, EdgeKind::Tree);
            bottom = self.children_tree(child, rect, &map.nodes[&child].children);
            y = bottom + TREE_V_GAP;
        }
        bottom
    }

    // ---------- Zeitleiste ----------

    fn timeline(&mut self, root: NodeId, root_rect: Rect, kids: &[NodeId]) {
        let map = self.map;
        let mut x = root_rect.right() + TIMELINE_GAP;
        let mut previous = root;

        for &child in kids {
            let size = self.sizes[&child];
            let rect = Rect::from_min_size(pos2(x, -size.y / 2.0), size);
            self.place(child, rect);
            self.edge(previous, child, EdgeKind::Timeline);
            self.children_tree(child, rect, &map.nodes[&child].children);

            let right = map
                .descendants(child)
                .iter()
                .filter_map(|d| self.out.rects.get(d))
                .map(|r| r.right())
                .fold(rect.right(), f32::max);
            x = right + TIMELINE_GAP;
            previous = child;
        }
    }
}
