use egui::{
    Color32, FontFamily, FontId, Margin, Rounding, Stroke, Style, TextStyle, Vec2, Visuals,
};

// ---- Палитра -------------------------------------------------------------

/// Основной фон окна — графит.
pub const BG: Color32 = Color32::from_rgb(0x13, 0x14, 0x17);
/// Панели навигации и нижняя панель.
pub const PANEL: Color32 = Color32::from_rgb(0x18, 0x19, 0x1D);
/// Карточки.
pub const CARD: Color32 = Color32::from_rgb(0x20, 0x22, 0x27);
pub const CARD_HOVER: Color32 = Color32::from_rgb(0x26, 0x29, 0x2F);
/// Поля ввода и «утопленные» поверхности.
pub const INPUT: Color32 = Color32::from_rgb(0x16, 0x17, 0x1B);

pub const BORDER: Color32 = Color32::from_rgb(0x2C, 0x2F, 0x37);
pub const BORDER_SOFT: Color32 = Color32::from_rgb(0x23, 0x25, 0x2B);

pub const TEXT: Color32 = Color32::from_rgb(0xE7, 0xE8, 0xEC);
pub const MUTED: Color32 = Color32::from_rgb(0x9B, 0x9E, 0xA6);
pub const FAINT: Color32 = Color32::from_rgb(0x6C, 0x6F, 0x78);

/// Приглушённый оранжевый акцент.
pub const ACCENT: Color32 = Color32::from_rgb(0xD1, 0x84, 0x49);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xE0, 0x95, 0x58);
/// Полупрозрачная подложка акцента (выделение, иконки-плитки).
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0x3A, 0x2A, 0x1D);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(0x6B, 0x48, 0x2C);

pub const DANGER: Color32 = Color32::from_rgb(0xCB, 0x6D, 0x5D);
pub const DANGER_SOFT: Color32 = Color32::from_rgb(0x3A, 0x23, 0x21);

// ---- Метрики -------------------------------------------------------------

pub const RADIUS_CARD: f32 = 12.0;
pub const RADIUS_DIALOG: f32 = 16.0;
pub const RADIUS_BTN: f32 = 8.0;
pub const RADIUS_SM: f32 = 6.0;

pub const SP_XS: f32 = 4.0;
pub const SP_S: f32 = 8.0;
pub const SP_M: f32 = 12.0;
pub const SP_L: f32 = 16.0;
pub const SP_XL: f32 = 24.0;

pub const NAV_WIDTH: f32 = 220.0;

pub fn apply(ctx: &egui::Context) {
    let mut style = Style::default();
    let mut v = Visuals::dark();

    v.dark_mode = true;
    v.override_text_color = Some(TEXT);
    v.panel_fill = BG;
    v.window_fill = PANEL;
    v.extreme_bg_color = INPUT;
    v.faint_bg_color = Color32::from_rgb(0x1D, 0x1F, 0x24);
    v.code_bg_color = INPUT;
    v.window_stroke = Stroke::new(1.0_f32, BORDER);
    v.window_rounding = Rounding::same(RADIUS_DIALOG);
    v.menu_rounding = Rounding::same(RADIUS_CARD);
    v.hyperlink_color = ACCENT;
    v.warn_fg_color = ACCENT_HOVER;
    v.error_fg_color = DANGER;
    v.striped = false;
    v.window_highlight_topmost = false;
    v.window_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 10.0),
        blur: 36.0,
        spread: 0.0,
        color: Color32::from_black_alpha(160),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: Vec2::new(0.0, 6.0),
        blur: 20.0,
        spread: 0.0,
        color: Color32::from_black_alpha(130),
    };

    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);

    let w = &mut v.widgets;
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = PANEL;
    w.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER_SOFT);
    w.noninteractive.fg_stroke = Stroke::new(1.0_f32, MUTED);

    w.inactive.bg_fill = CARD;
    w.inactive.weak_bg_fill = CARD;
    w.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);

    w.hovered.bg_fill = CARD_HOVER;
    w.hovered.weak_bg_fill = CARD_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(0x3A, 0x3E, 0x47));
    w.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);

    w.active.bg_fill = ACCENT_SOFT;
    w.active.weak_bg_fill = ACCENT_SOFT;
    w.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    w.active.fg_stroke = Stroke::new(1.0_f32, TEXT);

    w.open.bg_fill = CARD;
    w.open.weak_bg_fill = CARD;
    w.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    w.open.fg_stroke = Stroke::new(1.0_f32, TEXT);

    for ws in [
        &mut w.noninteractive,
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        ws.rounding = Rounding::same(RADIUS_BTN);
    }
    w.noninteractive.rounding = Rounding::same(RADIUS_SM);

    style.visuals = v;
    style.spacing.item_spacing = Vec2::new(SP_S, SP_S);
    style.spacing.button_padding = Vec2::new(SP_M, 7.0);
    style.spacing.interact_size = Vec2::new(40.0, 32.0);
    style.spacing.menu_margin = Margin::same(SP_S);
    style.spacing.window_margin = Margin::same(SP_L);
    style.spacing.indent = SP_L;
    style.spacing.scroll.bar_width = 9.0;
    style.spacing.scroll.bar_inner_margin = 3.0;
    style.spacing.scroll.floating = false;

    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(24.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Body,
        FontId::new(14.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style
        .text_styles
        .insert(TextStyle::Small, FontId::new(11.5, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(12.0, FontFamily::Monospace),
    );

    ctx.set_style(style);
}

/// Рамка карточки.
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(RADIUS_CARD))
        .inner_margin(Margin::same(SP_L))
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 2.0),
            blur: 10.0,
            spread: 0.0,
            color: Color32::from_black_alpha(45),
        })
}

/// Рамка диалогового окна.
pub fn dialog_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(PANEL)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(RADIUS_DIALOG))
        .inner_margin(Margin::same(SP_XL))
        .shadow(egui::epaint::Shadow {
            offset: Vec2::new(0.0, 14.0),
            blur: 44.0,
            spread: 0.0,
            color: Color32::from_black_alpha(170),
        })
}
