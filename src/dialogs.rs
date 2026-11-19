use egui::{Align, Color32, Context, Id, Layout, RichText, TextEdit};

use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, BtnKind};
use crate::{file_stem, pick_dir, pick_file, App, ConfirmAction};

/// Показывает модальные диалоги и затемнение фона.
pub(crate) fn show(app: &mut App, ctx: &Context) {
    let modal =
        app.prefix_dialog.is_some() || app.app_dialog.is_some() || app.confirm.is_some();
    if !modal {
        return;
    }

    let mut close_all = false;
    egui::Area::new(Id::new("modal-dim"))
        .order(egui::Order::Middle)
        .fixed_pos(egui::Pos2::ZERO)
        .show(ctx, |ui| {
            let screen = ctx.screen_rect();
            let resp = ui.allocate_rect(screen, egui::Sense::click());
            ui.painter()
                .rect_filled(screen, 0.0, Color32::from_black_alpha(150));
            if resp.clicked() {
                close_all = true;
            }
        });

    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        close_all = true;
    }

    if close_all {
        app.prefix_dialog = None;
        app.app_dialog = None;
        app.confirm = None;
        return;
    }

    if app.prefix_dialog.is_some() {
        prefix_dialog(app, ctx);
    }
    if app.app_dialog.is_some() {
        app_dialog(app, ctx);
    }
    if app.confirm.is_some() {
        confirm_dialog(app, ctx);
    }
}

fn window<R>(
    ctx: &Context,
    id: &str,
    title: &str,
    width: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> Option<(R, bool)> {
    let mut close = false;
    let inner = egui::Window::new(title)
        .id(Id::new(id))
        .title_bar(false)
        .collapsible(false)
        .resizable(false)
        .movable(false)
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .min_width(width)
        .max_width(width)
        .frame(theme::dialog_frame())
        .show(ctx, |ui| {
            ui.set_width(width - 2.0 * theme::SP_XL);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .size(18.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(ui, Icon::Close, "Закрыть", BtnKind::Ghost).clicked()
                    {
                        close = true;
                    }
                });
            });
            ui.add_space(theme::SP_M);
            add(ui)
        });
    inner.and_then(|i| i.inner.map(|r| (r, close)))
}

fn prefix_dialog(app: &mut App, ctx: &Context) {
    let Some(mut form) = app.prefix_dialog.clone() else {
        return;
    };
    let mut create = false;
    let mut cancel = false;

    let result = window(ctx, "dlg-prefix", "Новый префикс", 460.0, |ui| {
        widgets::field_label(ui, "Имя");
        ui.add(
            TextEdit::singleline(&mut form.name)
                .hint_text("tld")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(theme::SP_S);

        widgets::field_label(ui, "Папка");
        ui.add(
            TextEdit::singleline(&mut form.dir)
                .hint_text("пусто = ~/.local/share/rustauncher/prefixes/<имя>")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(theme::SP_S);

        widgets::field_label(ui, "Версия Windows");
        egui::ComboBox::from_id_salt("new-winver")
            .selected_text(form.winver.as_str())
            .width(140.0)
            .show_ui(ui, |ui| {
                for v in ["win11", "win10", "win8", "win7"] {
                    ui.selectable_value(&mut form.winver, v.to_string(), v);
                }
            });

        ui.add_space(theme::SP_M);
        widgets::hint(
            ui,
            "wineboot подготовит окружение в фоне. Это может занять около минуты.",
        );
        ui.add_space(theme::SP_L);

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button(ui, Some(Icon::Check), "Создать", BtnKind::Primary).clicked() {
                create = true;
            }
            if widgets::button(ui, None, "Отмена", BtnKind::Ghost).clicked() {
                cancel = true;
            }
        });
    });

    let closed = result.map(|(_, c)| c).unwrap_or(true);
    if create {
        app.prefix_dialog = Some(form);
        app.create_prefix_from_dialog();
    } else if cancel || closed {
        app.prefix_dialog = None;
    }
}

fn app_dialog(app: &mut App, ctx: &Context) {
    let Some(mut form) = app.app_dialog.clone() else {
        return;
    };
    let prefix_names: Vec<String> = app
        .cfg
        .prefixes
        .iter()
        .map(|p| p.name.clone())
        .collect();
    let title = if form.index.is_some() {
        "Настройка приложения"
    } else {
        "Новое приложение"
    };
    let mut save = false;
    let mut cancel = false;

    let result = window(ctx, "dlg-app", title, 580.0, |ui| {
        egui::Grid::new("app-form-grid")
            .num_columns(2)
            .spacing([theme::SP_M, theme::SP_S])
            .show(ui, |ui| {
                widgets::field_label(ui, "Префикс");
                let previous = form.prefix;
                egui::ComboBox::from_id_salt("app-prefix")
                    .selected_text(
                        prefix_names
                            .get(form.prefix)
                            .cloned()
                            .unwrap_or_else(|| "—".to_string()),
                    )
                    .width(260.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in prefix_names.iter().enumerate() {
                            ui.selectable_value(&mut form.prefix, i, name);
                        }
                    });
                if form.prefix != previous {
                    // При переносе в другой префикс создаём новую запись,
                    // а не перезаписываем приложение по старому индексу.
                    form.index = None;
                }
                ui.end_row();

                widgets::field_label(ui, "Имя");
                ui.add(
                    TextEdit::singleline(&mut form.name)
                        .hint_text("Название в списке")
                        .desired_width(360.0),
                );
                ui.end_row();

                widgets::field_label(ui, "Exe");
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut form.exe)
                            .hint_text("/path/to/game.exe")
                            .desired_width(420.0),
                    );
                    if widgets::button(ui, Some(Icon::Folder), "Обзор", BtnKind::Secondary)
                        .clicked()
                    {
                        if let Some(path) = pick_file("") {
                            if form.name.trim().is_empty() {
                                form.name = file_stem(&path);
                            }
                            form.exe = path;
                        }
                    }
                });
                ui.end_row();

                widgets::field_label(ui, "Рабочая папка");
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::singleline(&mut form.workdir)
                            .hint_text("пусто = папка exe")
                            .desired_width(420.0),
                    );
                    if widgets::button(ui, Some(Icon::Folder), "Обзор", BtnKind::Secondary)
                        .clicked()
                    {
                        if let Some(path) = pick_dir("") {
                            form.workdir = path;
                        }
                    }
                });
                ui.end_row();

                widgets::field_label(ui, "Аргументы");
                ui.add(
                    TextEdit::singleline(&mut form.args)
                        .hint_text("-windowed")
                        .desired_width(420.0),
                );
                ui.end_row();

                widgets::field_label(ui, "Env");
                ui.add(
                    TextEdit::multiline(&mut form.env)
                        .desired_rows(2)
                        .desired_width(420.0)
                        .hint_text("KEY=VALUE"),
                );
                ui.end_row();

                widgets::field_label(ui, "DLL overrides");
                ui.add(
                    TextEdit::multiline(&mut form.dll)
                        .desired_rows(2)
                        .desired_width(420.0)
                        .hint_text("version=n,b"),
                );
                ui.end_row();
            });

        if !form.exe.trim().is_empty() && !std::path::Path::new(form.exe.trim()).exists() {
            ui.add_space(theme::SP_XS);
            ui.label(
                RichText::new("Файл по этому пути не найден")
                    .size(11.0)
                    .color(theme::ACCENT_HOVER),
            );
        }

        ui.add_space(theme::SP_L);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button(ui, Some(Icon::Check), "Сохранить", BtnKind::Primary).clicked() {
                save = true;
            }
            if widgets::button(ui, None, "Отмена", BtnKind::Ghost).clicked() {
                cancel = true;
            }
        });
    });

    let closed = result.map(|(_, c)| c).unwrap_or(true);
    if save {
        app.app_dialog = Some(form);
        app.save_app_dialog();
    } else if cancel || closed {
        app.app_dialog = None;
    }
}

fn confirm_dialog(app: &mut App, ctx: &Context) {
    let Some(mut confirm) = app.confirm.clone() else {
        return;
    };
    let title = confirm.title.clone();
    let text = confirm.text.clone();
    let confirm_label = confirm.confirm_label.clone();
    let danger = confirm.danger;
    let is_prefix = matches!(confirm.action, ConfirmAction::DeletePrefix(_));
    let mut yes = false;
    let mut no = false;

    let result = window(ctx, "dlg-confirm", &title, 440.0, |ui| {
        ui.label(RichText::new(text).size(12.5).color(theme::MUTED));
        if is_prefix {
            ui.add_space(theme::SP_M);
            ui.checkbox(&mut confirm.delete_folder, "Удалить и папку префикса");
        }
        ui.add_space(theme::SP_L);
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let kind = if danger {
                BtnKind::Danger
            } else {
                BtnKind::Primary
            };
            if widgets::button(ui, Some(Icon::Trash), &confirm_label, kind).clicked() {
                yes = true;
            }
            if widgets::button(ui, None, "Отмена", BtnKind::Ghost).clicked() {
                no = true;
            }
        });
    });

    let closed = result.map(|(_, c)| c).unwrap_or(true);
    if yes {
        app.confirm = Some(confirm);
        app.apply_confirm();
    } else if no || closed {
        app.confirm = None;
    }
}
