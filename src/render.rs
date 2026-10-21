//! Zeichnen von Themen, Verbindungslinien und den Vorschaubildern der MindMap-Typen.

use crate::layout::{EdgeKind, TREE_INDENT};
use crate::model::{BorderShape, LineStyle, MapKind, NodeStyle};
use egui::epaint::CubicBezierShape;
use egui::{pos2, vec2, Color32, Galley, Painter, Pos2, Rect, Shape, Stroke};
use std::sync::Arc;

pub const SELECTION_COLOR: Color32 = Color32::from_rgb(0x2F, 0x80, 0xED);

/// Zeichnet ein Thema. `galley` ist `None`, während das Thema bearbeitet wird
/// (dann zeichnet das Textfeld den Text).
pub fn draw_node(
    painter: &Painter,
    rect: Rect,
    style: &NodeStyle,
    galley: Option<Arc<Galley>>,
    zoom: f32,
    selected: bool,
) {
    let stroke = Stroke::new(style.border_width * zoom, style.border_color);
    match style.shape {
        BorderShape::Rectangle => {
            painter.rect(rect, 0.0, style.fill, stroke);
        }
        BorderShape::Rounded => {
            painter.rect(rect, 6.0 * zoom, style.fill, stroke);
        }
        BorderShape::Capsule => {
            painter.rect(rect, rect.height() / 2.0, style.fill, stroke);
        }
        BorderShape::Ellipse => {
            painter.add(Shape::convex_polygon(ellipse_points(rect, 64), style.fill, stroke));
        }
        BorderShape::Underline => {
            if style.fill.a() > 0 {
                painter.rect_filled(rect, 3.0 * zoom, style.fill);
            }
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], stroke);
        }
        BorderShape::None => {
            if style.fill.a() > 0 {
                painter.rect_filled(rect, 3.0 * zoom, style.fill);
            }
        }
    }

    if let Some(galley) = galley {
        let pos = rect.center() - galley.size() / 2.0;
        painter.galley(pos, galley, style.text_color);
    }

    if selected {
        painter.rect_stroke(
            rect.expand(4.0 * zoom.max(0.5)),
            6.0 * zoom,
            Stroke::new(2.0_f32, SELECTION_COLOR),
        );
    }
}

fn ellipse_points(rect: Rect, segments: usize) -> Vec<Pos2> {
    let c = rect.center();
    let (rx, ry) = (rect.width() / 2.0, rect.height() / 2.0);
    (0..segments)
        .map(|i| {
            let a = i as f32 / segments as f32 * std::f32::consts::TAU;
            pos2(c.x + rx * a.cos(), c.y + ry * a.sin())
        })
        .collect()
}

#[derive(Clone, Copy)]
enum Orientation {
    /// Linie verläuft hauptsächlich horizontal.
    Horizontal,
    /// Linie verläuft hauptsächlich vertikal.
    Vertical,
    /// Erst nach unten, dann nach rechts (Baumdiagramm).
    Corner,
}

/// Zeichnet die Verbindungslinie zwischen zwei Themen (in Bildschirmkoordinaten).
/// Die Formatierung stammt vom Zielthema (`style`).
#[allow(clippy::too_many_arguments)]
pub fn draw_edge(
    painter: &Painter,
    kind: EdgeKind,
    from: Rect,
    from_shape: BorderShape,
    to: Rect,
    to_shape: BorderShape,
    style: &NodeStyle,
    zoom: f32,
) {
    let stroke = Stroke::new(style.line_width * zoom, style.line_color);
    if stroke.width <= 0.0 || style.line_color.a() == 0 {
        return;
    }

    // Unterstrichene Themen docken – wie bei Xmind – an der Unterlinie an.
    let side_y = |r: Rect, shape: BorderShape| {
        if shape == BorderShape::Underline {
            r.bottom()
        } else {
            r.center().y
        }
    };

    let (a, b, orientation) = match kind {
        EdgeKind::Horizontal | EdgeKind::Timeline => {
            let (fx, tx) = if to.center().x >= from.center().x {
                (from.right(), to.left())
            } else {
                (from.left(), to.right())
            };
            (
                pos2(fx, side_y(from, from_shape)),
                pos2(tx, side_y(to, to_shape)),
                Orientation::Horizontal,
            )
        }
        EdgeKind::Vertical => (from.center_bottom(), to.center_top(), Orientation::Vertical),
        EdgeKind::Tree => (
            pos2(from.left() + TREE_INDENT * zoom / 2.0, from.bottom()),
            pos2(to.left(), side_y(to, to_shape)),
            Orientation::Corner,
        ),
    };

    match style.line_style {
        LineStyle::Straight => {
            painter.line_segment([a, b], stroke);
        }
        LineStyle::Curve => {
            let (c1, c2) = match orientation {
                Orientation::Horizontal => {
                    let mx = a.x + (b.x - a.x) * 0.5;
                    (pos2(mx, a.y), pos2(mx, b.y))
                }
                Orientation::Vertical => {
                    let my = a.y + (b.y - a.y) * 0.5;
                    (pos2(a.x, my), pos2(b.x, my))
                }
                Orientation::Corner => (
                    pos2(a.x, a.y + (b.y - a.y) * 0.8),
                    pos2(a.x + (b.x - a.x) * 0.2, b.y),
                ),
            };
            painter.add(CubicBezierShape::from_points_stroke(
                [a, c1, c2, b],
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        LineStyle::Elbow => {
            let points = match orientation {
                Orientation::Horizontal => {
                    let mx = a.x + (b.x - a.x) * 0.5;
                    vec![a, pos2(mx, a.y), pos2(mx, b.y), b]
                }
                Orientation::Vertical => {
                    let my = a.y + (b.y - a.y) * 0.5;
                    vec![a, pos2(a.x, my), pos2(b.x, my), b]
                }
                Orientation::Corner => vec![a, pos2(a.x, b.y), b],
            };
            painter.add(Shape::line(points, stroke));
        }
    }
}

/// Kleines schematisches Vorschaubild eines MindMap-Typs für die Seitenleiste.
pub fn draw_thumbnail(painter: &Painter, r: Rect, kind: MapKind, color: Color32) {
    let mirror = kind == MapKind::LogicLeft;
    let at = |x: f32, y: f32| {
        let x = if mirror { 1.0 - x } else { x };
        pos2(r.left() + x * r.width(), r.top() + y * r.height())
    };
    let stroke = Stroke::new(1.3_f32, color);
    let node = |x: f32, y: f32, w: f32, h: f32, filled: bool| {
        let rect = Rect::from_center_size(at(x, y), vec2(w * r.width(), h * r.height()));
        if filled {
            painter.rect_filled(rect, 2.0, color);
        } else {
            painter.rect_stroke(rect, 2.0, stroke);
        }
    };
    let line = |pts: &[(f32, f32)]| {
        painter.add(Shape::line(pts.iter().map(|&(x, y)| at(x, y)).collect(), stroke));
    };

    match kind {
        MapKind::MindMap => {
            for (x, y) in [(0.84, 0.2), (0.84, 0.8), (0.16, 0.2), (0.16, 0.8)] {
                line(&[(0.5, 0.5), (x, y)]);
                node(x, y, 0.2, 0.2, false);
            }
            node(0.5, 0.5, 0.26, 0.28, true);
        }
        MapKind::LogicRight | MapKind::LogicLeft => {
            for y in [0.15, 0.5, 0.85] {
                line(&[(0.27, 0.5), (0.45, 0.5), (0.45, y), (0.6, y)]);
                node(0.76, y, 0.3, 0.2, false);
            }
            node(0.15, 0.5, 0.24, 0.28, true);
        }
        MapKind::OrgChart => {
            for x in [0.18, 0.5, 0.82] {
                line(&[(0.5, 0.3), (0.5, 0.5), (x, 0.5), (x, 0.68)]);
                node(x, 0.8, 0.24, 0.22, false);
            }
            node(0.5, 0.17, 0.3, 0.26, true);
        }
        MapKind::TreeChart => {
            line(&[(0.12, 0.28), (0.12, 0.86)]);
            for y in [0.44, 0.65, 0.86] {
                line(&[(0.12, y), (0.28, y)]);
                node(0.48, y, 0.4, 0.16, false);
            }
            node(0.22, 0.14, 0.32, 0.24, true);
        }
        MapKind::Timeline => {
            line(&[(0.22, 0.4), (0.98, 0.4)]);
            for x in [0.46, 0.78] {
                node(x, 0.4, 0.22, 0.22, false);
                line(&[(x - 0.07, 0.51), (x - 0.07, 0.82), (x - 0.02, 0.82)]);
                node(x + 0.06, 0.82, 0.14, 0.14, false);
            }
            node(0.12, 0.4, 0.2, 0.28, true);
        }
    }
}
