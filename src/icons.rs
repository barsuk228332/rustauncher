use egui::{
    epaint::PathShape, vec2, Color32, Painter, Pos2, Rect, Response, Rounding, Sense, Shape,
    Stroke, Ui,
};

/// Векторные иконки в стиле линейных SVG-наборов.
/// Рисуются средствами egui — без растровых ассетов и зависимостей.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    Apps,
    Layers,
    Bolt,
    Sliders,
    Plus,
    Play,
    Edit,
    Trash,
    Folder,
    Close,
    Refresh,
    Download,
    Stop,
    Search,
    ChevronDown,
    ChevronRight,
    Check,
    Wine,
    Info,
}

pub fn icon(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint(ui.painter(), rect, icon, color);
    resp
}

pub fn icon_at(painter: &Painter, center: Pos2, size: f32, icon: Icon, color: Color32) {
    let rect = Rect::from_center_size(center, vec2(size, size));
    paint(painter, rect, icon, color);
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let sw = (rect.width() * 0.095).clamp(1.25, 2.1);
    let stroke = Stroke::new(sw, color);
    let pstroke = egui::epaint::PathStroke::new(sw, color);
    let p = |x: f32, y: f32| rect.min + vec2(x * rect.width(), y * rect.height());
    let r = |x0: f32, y0: f32, x1: f32, y1: f32| Rect::from_min_max(p(x0, y0), p(x1, y1));
    let line = |pts: Vec<Pos2>| painter.add(Shape::line(pts, pstroke.clone()));
    let closed = |pts: Vec<Pos2>| painter.add(Shape::closed_line(pts, pstroke.clone()));
    let filled = |pts: Vec<Pos2>, fill: Color32| {
        painter.add(Shape::Path(PathShape {
            points: pts,
            closed: true,
            fill,
            stroke: egui::epaint::PathStroke::NONE,
        }));
    };
    let dot = |x: f32, y: f32, rad: f32| {
        painter.add(Shape::circle_filled(
            p(x, y),
            rect.width() * rad,
            color,
        ));
    };
    let round = |rad: f32| Rounding::same(rect.width() * rad);

    match icon {
        Icon::Apps => {
            let a = r(0.12, 0.12, 0.46, 0.46);
            let b = r(0.54, 0.12, 0.88, 0.46);
            let c = r(0.12, 0.54, 0.46, 0.88);
            let d = r(0.54, 0.54, 0.88, 0.88);
            for sq in [a, b, c, d] {
                painter.rect_filled(sq, round(0.07), color);
            }
        }
        Icon::Layers => {
            closed(vec![
                p(0.5, 0.1),
                p(0.9, 0.32),
                p(0.5, 0.54),
                p(0.1, 0.32),
            ]);
            line(vec![p(0.11, 0.49), p(0.5, 0.7), p(0.89, 0.49)]);
            line(vec![p(0.11, 0.66), p(0.5, 0.87), p(0.89, 0.66)]);
        }
        Icon::Bolt => {
            filled(
                vec![
                    p(0.6, 0.06),
                    p(0.24, 0.55),
                    p(0.47, 0.55),
                    p(0.4, 0.94),
                    p(0.78, 0.43),
                    p(0.54, 0.43),
                ],
                color,
            );
        }
        Icon::Sliders => {
            for (y, x) in [(0.26, 0.36), (0.5, 0.66), (0.74, 0.46)] {
                line(vec![p(0.12, y), p(0.88, y)]);
                painter.add(Shape::circle_filled(p(x, y), rect.width() * 0.085, color));
            }
        }
        Icon::Plus => {
            line(vec![p(0.5, 0.13), p(0.5, 0.87)]);
            line(vec![p(0.13, 0.5), p(0.87, 0.5)]);
        }
        Icon::Play => {
            filled(vec![p(0.32, 0.14), p(0.85, 0.5), p(0.32, 0.86)], color);
        }
        Icon::Edit => {
            closed(vec![
                p(0.16, 0.84),
                p(0.22, 0.58),
                p(0.68, 0.12),
                p(0.88, 0.32),
                p(0.42, 0.78),
            ]);
            line(vec![p(0.22, 0.58), p(0.42, 0.78)]);
        }
        Icon::Trash => {
            line(vec![p(0.14, 0.26), p(0.86, 0.26)]);
            line(vec![p(0.38, 0.26), p(0.38, 0.13), p(0.62, 0.13), p(0.62, 0.26)]);
            closed(vec![
                p(0.21, 0.26),
                p(0.26, 0.9),
                p(0.74, 0.9),
                p(0.79, 0.26),
            ]);
            line(vec![p(0.42, 0.4), p(0.44, 0.78)]);
            line(vec![p(0.58, 0.4), p(0.56, 0.78)]);
        }
        Icon::Folder => {
            closed(vec![
                p(0.1, 0.28),
                p(0.42, 0.28),
                p(0.5, 0.4),
                p(0.9, 0.4),
                p(0.9, 0.84),
                p(0.1, 0.84),
            ]);
        }
        Icon::Close => {
            line(vec![p(0.24, 0.24), p(0.76, 0.76)]);
            line(vec![p(0.76, 0.24), p(0.24, 0.76)]);
        }
        Icon::Refresh => {
            let center = p(0.5, 0.5);
            let radius = rect.width() * 0.33;
            let start = 0.65_f32;
            let end = std::f32::consts::TAU - 0.65;
            let mut pts = Vec::new();
            for i in 0..=30 {
                let t = start + (end - start) * (i as f32 / 30.0);
                pts.push(center + vec2(t.cos(), t.sin()) * radius);
            }
            line(pts);
            let tip = center + vec2(end.cos(), end.sin()) * radius;
            let dir = vec2(-end.sin(), end.cos());
            let radial = vec2(end.cos(), end.sin());
            let s = rect.width();
            line(vec![tip + dir * (0.02 * s), tip, tip + radial * (0.16 * s)]);
            line(vec![
                tip + dir * (0.15 * s) + radial * (0.04 * s),
                tip,
            ]);
        }
        Icon::Download => {
            line(vec![p(0.5, 0.1), p(0.5, 0.62)]);
            line(vec![p(0.3, 0.44), p(0.5, 0.64), p(0.7, 0.44)]);
            line(vec![p(0.15, 0.84), p(0.85, 0.84)]);
        }
        Icon::Stop => {
            painter.rect_filled(r(0.24, 0.24, 0.76, 0.76), round(0.12), color);
        }
        Icon::Search => {
            painter.add(Shape::circle_stroke(
                p(0.42, 0.42),
                rect.width() * 0.27,
                stroke,
            ));
            line(vec![p(0.63, 0.63), p(0.87, 0.87)]);
        }
        Icon::ChevronDown => {
            line(vec![p(0.22, 0.4), p(0.5, 0.68), p(0.78, 0.4)]);
        }
        Icon::ChevronRight => {
            line(vec![p(0.4, 0.22), p(0.68, 0.5), p(0.4, 0.78)]);
        }
        Icon::Check => {
            line(vec![p(0.16, 0.54), p(0.4, 0.78), p(0.86, 0.24)]);
        }
        Icon::Wine => {
            closed(vec![
                p(0.4, 0.1),
                p(0.6, 0.1),
                p(0.6, 0.3),
                p(0.78, 0.54),
                p(0.78, 0.88),
                p(0.22, 0.88),
                p(0.22, 0.54),
                p(0.4, 0.3),
            ]);
            line(vec![p(0.4, 0.1), p(0.6, 0.1)]);
            line(vec![p(0.27, 0.66), p(0.73, 0.66)]);
        }
        Icon::Info => {
            painter.add(Shape::circle_stroke(
                p(0.5, 0.5),
                rect.width() * 0.36,
                stroke,
            ));
            line(vec![p(0.5, 0.44), p(0.5, 0.68)]);
            dot(0.5, 0.31, 0.045);
        }
    }
}
