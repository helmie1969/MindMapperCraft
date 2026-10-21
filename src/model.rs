//! Datenmodell: MindMap, Themen (Knoten) und deren Formatierung.

use crate::fonts::DEFAULT_FONT;
use egui::Color32;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type NodeId = u64;

/// Farbpalette für die Hauptzweige (wie bei Xmind bekommt jeder Hauptzweig eine eigene Farbe).
pub const BRANCH_PALETTE: [Color32; 6] = [
    Color32::from_rgb(0xE8, 0x54, 0x4E),
    Color32::from_rgb(0xF2, 0xA9, 0x3B),
    Color32::from_rgb(0x4C, 0xAF, 0x7A),
    Color32::from_rgb(0x3E, 0x8E, 0xD0),
    Color32::from_rgb(0x8E, 0x6C, 0xC9),
    Color32::from_rgb(0xE0, 0x6C, 0x9F),
];

/// Die verfügbaren MindMap-Typen (Strukturen).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapKind {
    MindMap,
    LogicRight,
    LogicLeft,
    OrgChart,
    TreeChart,
    Timeline,
}

impl MapKind {
    pub const ALL: [MapKind; 6] = [
        MapKind::MindMap,
        MapKind::LogicRight,
        MapKind::LogicLeft,
        MapKind::OrgChart,
        MapKind::TreeChart,
        MapKind::Timeline,
    ];

    pub fn label(self) -> &'static str {
        match self {
            MapKind::MindMap => "Mindmap",
            MapKind::LogicRight => "Logik rechts",
            MapKind::LogicLeft => "Logik links",
            MapKind::OrgChart => "Organigramm",
            MapKind::TreeChart => "Baum",
            MapKind::Timeline => "Zeitleiste",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            MapKind::MindMap => "Klassische Mindmap: Hauptthemen verteilen sich links und rechts",
            MapKind::LogicRight => "Logikdiagramm: alle Zweige wachsen nach rechts",
            MapKind::LogicLeft => "Logikdiagramm: alle Zweige wachsen nach links",
            MapKind::OrgChart => "Organigramm: Hierarchie von oben nach unten",
            MapKind::TreeChart => "Baumdiagramm: eingerückte Gliederung nach unten",
            MapKind::Timeline => "Zeitleiste: Hauptthemen entlang einer horizontalen Achse",
        }
    }
}

/// Rahmenform eines Themas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorderShape {
    Rectangle,
    Rounded,
    Capsule,
    Ellipse,
    Underline,
    None,
}

impl BorderShape {
    pub const ALL: [BorderShape; 6] = [
        BorderShape::Rectangle,
        BorderShape::Rounded,
        BorderShape::Capsule,
        BorderShape::Ellipse,
        BorderShape::Underline,
        BorderShape::None,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BorderShape::Rectangle => "Rechteck",
            BorderShape::Rounded => "Abgerundetes Rechteck",
            BorderShape::Capsule => "Kapsel",
            BorderShape::Ellipse => "Ellipse",
            BorderShape::Underline => "Unterstrichen",
            BorderShape::None => "Ohne Rahmen",
        }
    }
}

/// Form der Verbindungslinie vom Elternthema zu diesem Thema.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineStyle {
    Curve,
    Straight,
    Elbow,
}

impl LineStyle {
    pub const ALL: [LineStyle; 3] = [LineStyle::Curve, LineStyle::Straight, LineStyle::Elbow];

    pub fn label(self) -> &'static str {
        match self {
            LineStyle::Curve => "Kurve",
            LineStyle::Straight => "Gerade",
            LineStyle::Elbow => "Winkel",
        }
    }
}

/// Formatierung eines Themas: Rahmen, Schrift und eingehende Verbindungslinie.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeStyle {
    // Rahmen
    pub shape: BorderShape,
    pub fill: Color32,
    pub border_color: Color32,
    pub border_width: f32,
    // Schrift
    pub font_family: String,
    pub font_size: f32,
    pub bold: bool,
    pub text_color: Color32,
    // Verbindungslinie
    pub line_style: LineStyle,
    pub line_color: Color32,
    pub line_width: f32,
}

impl NodeStyle {
    /// Standardstil abhängig von der Ebene (0 = zentrales Thema, 1 = Hauptthema, 2+ = Unterthema).
    pub fn for_depth(depth: usize, branch: Color32) -> Self {
        match depth {
            0 => NodeStyle {
                shape: BorderShape::Rounded,
                fill: Color32::from_rgb(0x1F, 0x3A, 0x5F),
                border_color: Color32::from_rgb(0x1F, 0x3A, 0x5F),
                border_width: 0.0,
                font_family: DEFAULT_FONT.to_owned(),
                font_size: 22.0,
                bold: true,
                text_color: Color32::WHITE,
                line_style: LineStyle::Curve,
                line_color: Color32::from_rgb(0x55, 0x6B, 0x82),
                line_width: 2.5,
            },
            1 => NodeStyle {
                shape: BorderShape::Rounded,
                fill: mix(branch, Color32::WHITE, 0.82),
                border_color: branch,
                border_width: 1.5,
                font_family: DEFAULT_FONT.to_owned(),
                font_size: 16.0,
                bold: false,
                text_color: Color32::from_rgb(0x22, 0x22, 0x22),
                line_style: LineStyle::Curve,
                line_color: branch,
                line_width: 2.5,
            },
            _ => NodeStyle {
                shape: BorderShape::Underline,
                fill: Color32::TRANSPARENT,
                border_color: branch,
                border_width: 1.5,
                font_family: DEFAULT_FONT.to_owned(),
                font_size: 14.0,
                bold: false,
                text_color: Color32::from_rgb(0x33, 0x33, 0x33),
                line_style: LineStyle::Curve,
                line_color: branch,
                line_width: 1.5,
            },
        }
    }

    /// Übernimmt nur die Eigenschaften, die sich zwischen `before` und `after` geändert haben.
    /// So kann eine Änderung auf einen ganzen Zweig übertragen werden, ohne dass
    /// z. B. die unterschiedlichen Schriftgrößen der Ebenen verloren gehen.
    pub fn apply_changes(&mut self, before: &NodeStyle, after: &NodeStyle) {
        macro_rules! sync {
            ($($field:ident),* $(,)?) => {
                $( if before.$field != after.$field { self.$field = after.$field.clone(); } )*
            };
        }
        sync!(
            shape,
            fill,
            border_color,
            border_width,
            font_family,
            font_size,
            bold,
            text_color,
            line_style,
            line_color,
            line_width,
        );
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub text: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub style: NodeStyle,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MindMap {
    pub nodes: BTreeMap<NodeId, Node>,
    pub root: NodeId,
    pub next_id: NodeId,
    pub kind: MapKind,
    pub background: Color32,
}

impl MindMap {
    pub fn new(title: &str) -> Self {
        let root = Node {
            id: 0,
            text: title.to_owned(),
            parent: None,
            children: Vec::new(),
            style: NodeStyle::for_depth(0, BRANCH_PALETTE[0]),
        };
        MindMap {
            nodes: BTreeMap::from([(0, root)]),
            root: 0,
            next_id: 1,
            kind: MapKind::MindMap,
            background: Color32::from_rgb(0xF7, 0xF8, 0xFA),
        }
    }

    /// Startvorlage wie in Xmind: zentrales Thema mit vier Hauptthemen.
    pub fn sample() -> Self {
        let mut map = MindMap::new("Zentrales Thema");
        for i in 1..=4 {
            map.add_child(map.root, format!("Hauptthema {i}"));
        }
        map
    }

    pub fn is_valid(&self) -> bool {
        self.nodes.contains_key(&self.root)
            && self.nodes.values().all(|n| {
                n.children.iter().all(|c| self.nodes.contains_key(c))
                    && n.parent.map_or(n.id == self.root, |p| self.nodes.contains_key(&p))
            })
    }

    pub fn depth(&self, id: NodeId) -> usize {
        let mut depth = 0;
        let mut current = self.nodes.get(&id).and_then(|n| n.parent);
        while let Some(p) = current {
            depth += 1;
            current = self.nodes.get(&p).and_then(|n| n.parent);
        }
        depth
    }

    pub fn default_label(depth: usize) -> &'static str {
        match depth {
            0 => "Zentrales Thema",
            1 => "Hauptthema",
            _ => "Unterthema",
        }
    }

    /// Farbe des Hauptzweigs, zu dem `id` gehört.
    pub fn branch_color(&self, id: NodeId) -> Color32 {
        let mut current = id;
        while let Some(parent) = self.nodes.get(&current).and_then(|n| n.parent) {
            if parent == self.root {
                let index = self.nodes[&self.root]
                    .children
                    .iter()
                    .position(|c| *c == current)
                    .unwrap_or(0);
                return BRANCH_PALETTE[index % BRANCH_PALETTE.len()];
            }
            current = parent;
        }
        BRANCH_PALETTE[0]
    }

    pub fn default_style(&self, id: NodeId) -> NodeStyle {
        NodeStyle::for_depth(self.depth(id), self.branch_color(id))
    }

    pub fn add_child(&mut self, parent: NodeId, text: impl Into<String>) -> NodeId {
        let index = self.nodes[&parent].children.len();
        self.insert_child_at(parent, index, text.into())
    }

    /// Fügt ein Geschwisterthema direkt hinter `id` ein. Für das zentrale Thema `None`.
    pub fn add_sibling(&mut self, id: NodeId) -> Option<NodeId> {
        let parent = self.nodes.get(&id)?.parent?;
        let index = self.nodes[&parent].children.iter().position(|c| *c == id)? + 1;
        let label = Self::default_label(self.depth(id));
        Some(self.insert_child_at(parent, index, label.to_owned()))
    }

    fn insert_child_at(&mut self, parent: NodeId, index: usize, text: String) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;

        let parent_node = self.nodes.get_mut(&parent).expect("Elternthema existiert");
        let index = index.min(parent_node.children.len());
        parent_node.children.insert(index, id);

        self.nodes.insert(
            id,
            Node {
                id,
                text,
                parent: Some(parent),
                children: Vec::new(),
                style: NodeStyle::for_depth(1, BRANCH_PALETTE[0]),
            },
        );

        // Hauptthemen bekommen ihre Palettenfarbe, tiefere Themen erben die Linienfarbe des Elternthemas.
        let depth = self.depth(id);
        let branch = if depth <= 1 {
            self.branch_color(id)
        } else {
            self.nodes[&parent].style.line_color
        };
        self.nodes.get_mut(&id).unwrap().style = NodeStyle::for_depth(depth, branch);
        id
    }

    /// Entfernt ein Thema samt Unterthemen. Liefert das Thema, das danach ausgewählt werden soll.
    pub fn remove(&mut self, id: NodeId) -> Option<NodeId> {
        if id == self.root {
            return None;
        }
        let parent = self.nodes.get(&id)?.parent?;
        let siblings = &mut self.nodes.get_mut(&parent)?.children;
        let index = siblings.iter().position(|c| *c == id)?;
        siblings.remove(index);

        let next = siblings
            .get(index)
            .or_else(|| index.checked_sub(1).and_then(|i| siblings.get(i)))
            .copied()
            .unwrap_or(parent);

        for d in self.descendants(id) {
            self.nodes.remove(&d);
        }
        self.nodes.remove(&id);
        Some(next)
    }

    /// Alle Nachfahren (ohne `id` selbst).
    pub fn descendants(&self, id: NodeId) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut stack: Vec<NodeId> = self.nodes.get(&id).map(|n| n.children.clone()).unwrap_or_default();
        while let Some(n) = stack.pop() {
            out.push(n);
            if let Some(node) = self.nodes.get(&n) {
                stack.extend(node.children.iter().copied());
            }
        }
        out
    }

    /// Setzt alle Themen auf den Standardstil ihrer Ebene zurück.
    pub fn reset_styles(&mut self) {
        let ids: Vec<NodeId> = self.nodes.keys().copied().collect();
        for id in ids {
            let style = self.default_style(id);
            self.nodes.get_mut(&id).unwrap().style = style;
        }
    }
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}
