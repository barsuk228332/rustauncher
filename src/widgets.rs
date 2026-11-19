use egui::{
    vec2, Align, Color32, CursorIcon, FontId, Id, InnerResponse, Layout, Response, Rounding,
    Sense, Stroke, Ui,
};

use crate::icons::{self, Icon};
use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BtnKind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

fn colors(kind: BtnKind, hovered: bool) -> (Color32, Color32, Color32, Color32) {
    // (fill, border, text, icon)
    match (kind, hovered) {
        (BtnKind::Primary, false) => (
            theme::ACCENT,
            theme::ACCENT,
            Color32::from_rgb(0x1B, 0x14, 0x0E),
            Color32::from_rgb(0x1B, 0x14, 0x0E),
        ),
        (BtnKind::Primary, true) => (
            theme::ACCENT_HOVER,
            theme::ACCENT_HOVER,
            Color32::from_rgb(0x1B, 0x14, 0x0E),
            Color32::from_rgb(0x1B, 0x14, 0x0E),
        ),
        (BtnKind::Secondary, false) => (
            theme::CARD,
            theme::BORDER,
            theme::TEXT,
            theme::MUTED,
        ),
        (BtnKind::Secondary, true) => (
            theme::CARD_HOVER,
            theme::ACCENT_DIM,
            theme::TEXT,
            theme::ACCENT,
        ),
        (BtnKind::Ghost, false) => (
            Color32::TRANSPARENT,
            Color32::TRANSPARENT,
            theme::MUTED,
            theme::MUTED,
        ),
        (BtnKind::Ghost, true) => (
            theme::CARD,
            theme::BORDER_SOFT,
            theme::TEXT,
            theme::TEXT,
        ),
        (BtnKind::Danger, false) => (
            theme::DANGER_SOFT,
            Color32::from_rgb(0x4A, 0x2A, 0x27),
            theme::DANGER,
            theme::DANGER,
        ),
        (BtnKind::Danger, true) => (
            Color32::from_rgb(0x46, 0x28, 0x25),
            theme::DANGER,
            Color32::from_rgb(0xE2, 0x8A, 0x79),
            Color32::from_rgb(0xE2, 0x8A, 0x79),
        ),
    }
}

pub fn button(ui: &mut Ui, icon: Option<Icon>, label: &str, kind: BtnKind) -> Response {
    button_h(ui, icon, label, kind, 32.0)
}

pub fn button_h(
    ui: &mut Ui,
    icon: Option<Icon>,
    label: &str,
    kind: BtnKind,
    height: f32,
) -> Response {
    let (fill, border, text_color, icon_color) = colors(kind, false);
    let font = FontId::proportional(if height < 30.0 { 12.0 } else { 13.0 });
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font, text_color));

    let pad_x = if height < 30.0 { 10.0 } else { 12.0 };
    let icon_w = if icon.is_some() { 15.0 + 7.0 } else { 0.0 };
    let width = (galley.size().x + pad_x * 2.0 + icon_w).max(height);
    let (rect, resp) = ui.allocate_exact_size(vec2(width, height), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hovered = resp.hovered();
    let (fill, border, _, _) = if hovered {
        let c = colors(kind, true);
        (c.0, c.1, c.2, c.3)
    } else {
        (fill, border, text_color, icon_color)
    };
    let (text_color, icon_color) = if hovered {
        let c = colors(kind, true);
        (c.2, c.3)
    } else {
        (text_color, icon_color)
    };

    let painter = ui.painter();
    let rounding = Rounding::same(theme::RADIUS_BTN);
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, rounding, fill);
    }
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, border));
    }

    let mut x = rect.left() + pad_x;
    if let Some(ic) = icon {
        icons::icon_at(
            painter,
            egui::pos2(x + 7.5, rect.center().y),
            15.0,
            ic,
            icon_color,
        );
        x += 15.0 + 7.0;
    }
    painter.galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        text_color,
    );

    resp
}

pub fn icon_button(ui: &mut Ui, icon: Icon, tooltip: &str, kind: BtnKind) -> Response {
    icon_button_size(ui, icon, 30.0, tooltip, kind)
}

pub fn icon_button_size(
    ui: &mut Ui,
    icon: Icon,
    size: f32,
    tooltip: &str,
    kind: BtnKind,
) -> Response {
    let (fill, border, _, icon_color) = colors(kind, false);
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hovered = resp.hovered();
    let (fill, border, icon_color) = if hovered {
        let c = colors(kind, true);
        (c.0, c.1, c.3)
    } else {
        (fill, border, icon_color)
    };

    let painter = ui.painter();
    let rounding = Rounding::same(theme::RADIUS_BTN);
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, rounding, fill);
    }
    if border != Color32::TRANSPARENT {
        painter.rect_stroke(rect, rounding, Stroke::new(1.0_f32, border));
    }
    icons::icon_at(
        painter,
        rect.center(),
        size * 0.5,
        icon,
        icon_color,
    );

    let resp = resp.on_hover_text(tooltip);
    resp
}

pub fn nav_item(ui: &mut Ui, icon: Icon, label: &str, selected: bool) -> Response {
    let height = 40.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let hovered = resp.hovered();
    let painter = ui.painter();
    let rounding = Rounding::same(theme::RADIUS_BTN);

    if selected {
        painter.rect_filled(rect, rounding, theme::ACCENT_SOFT);
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left() + 2.0, rect.center().y - 9.0),
            vec2(3.0, 18.0),
        );
        painter.rect_filled(bar, Rounding::same(2.0), theme::ACCENT);
    } else if hovered {
        painter.rect_filled(rect, rounding, theme::CARD);
    }

    let icon_color = if selected {
        theme::ACCENT
    } else if hovered {
        theme::TEXT
    } else {
        theme::MUTED
    };
    icons::icon_at(
        painter,
        egui::pos2(rect.left() + 22.0, rect.center().y),
        17.0,
        icon,
        icon_color,
    );
    painter.text(
        egui::pos2(rect.left() + 44.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        if selected { theme::TEXT } else { theme::MUTED },
    );

    resp
}

pub fn icon_tile(ui: &mut Ui, icon: Icon, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(size * 0.28), theme::ACCENT_SOFT);
    icons::icon_at(
        ui.painter(),
        rect.center(),
        size * 0.5,
        icon,
        theme::ACCENT,
    );
}

pub fn pill(ui: &mut Ui, text: &str, fg: Color32, bg: Color32) -> Response {
    let font = FontId::proportional(11.0);
    let galley = ui.fonts(|f| f.layout_no_wrap(text.to_owned(), font, fg));
    let size = vec2(galley.size().x + 14.0, galley.size().y + 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(size.y / 2.0), bg);
    painter.galley(
        egui::pos2(
            rect.center().x - galley.size().x / 2.0,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        fg,
    );
    resp
}

pub fn pill_accent(ui: &mut Ui, text: &str) -> Response {
    pill(ui, text, theme::ACCENT_HOVER, theme::ACCENT_SOFT)
}

pub fn pill_muted(ui: &mut Ui, text: &str) -> Response {
    pill(
        ui,
        text,
        theme::MUTED,
        Color32::from_rgb(0x25, 0x27, 0x2D),
    )
}

pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    theme::card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    })
}

pub fn selectable_card<R>(
    ui: &mut Ui,
    id: Id,
    selected: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> (Response, R) {
    let frame = theme::card_frame()
        .fill(if selected { theme::ACCENT_SOFT } else { theme::CARD })
        .stroke(Stroke::new(1.0_f32,
            if selected {
                theme::ACCENT_DIM
            } else {
                theme::BORDER
            },
        ))
        .inner_margin(egui::Margin::same(theme::SP_M));
    let inner = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let resp = ui
        .interact(inner.response.rect, id, Sense::click())
        .on_hover_cursor(CursorIcon::PointingHand);
    if resp.hovered() && !selected {
        ui.painter().rect_filled(
            inner.response.rect,
            Rounding::same(theme::RADIUS_CARD),
            Color32::from_white_alpha(4),
        );
    }
    (resp, inner.inner)
}

pub fn section_title(ui: &mut Ui, icon: Icon, title: &str) {
    ui.horizontal(|ui| {
        icons::icon(ui, icon, 14.0, theme::FAINT);
        ui.label(
            egui::RichText::new(title)
                .size(11.0)
                .strong()
                .color(theme::FAINT),
        );
    });
    ui.add_space(theme::SP_XS);
}

pub fn page_header(
    ui: &mut Ui,
    title: &str,
    subtitle: &str,
    actions: impl FnOnce(&mut Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new(title)
                    .size(24.0)
                    .strong()
                    .color(theme::TEXT),
            );
            ui.label(egui::RichText::new(subtitle).size(13.0).color(theme::MUTED));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            actions(ui);
        });
    });
}

pub fn hint(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(11.5).color(theme::FAINT));
}

pub fn field_label(ui: &mut Ui, text: &str) {
    ui.label(egui::RichText::new(text).size(12.0).color(theme::MUTED));
}

pub fn empty_state(
    ui: &mut Ui,
    icon: Icon,
    title: &str,
    text: &str,
    action: impl FnOnce(&mut Ui),
) {
    ui.vertical_centered(|ui| {
        ui.add_space(theme::SP_XL);
        let (rect, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
        ui.painter()
            .rect_filled(rect, Rounding::same(16.0), theme::CARD);
        icons::icon_at(ui.painter(), rect.center(), 26.0, icon, theme::FAINT);
        ui.add_space(theme::SP_M);
        ui.label(
            egui::RichText::new(title)
                .size(16.0)
                .strong()
                .color(theme::TEXT),
        );
        ui.label(
            egui::RichText::new(text)
                .size(12.5)
                .color(theme::MUTED),
        );
        ui.add_space(theme::SP_M);
        ui.vertical_centered(action);
        ui.add_space(theme::SP_XL);
    });
}
