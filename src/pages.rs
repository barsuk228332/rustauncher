use std::path::Path;
use std::sync::atomic::Ordering;

use egui::{
    vec2, Align, Label, Layout, Margin, RichText, Rounding, ScrollArea, Sense, Stroke, TextEdit,
    Ui,
};

use crate::config;
use crate::icons::{self, Icon};
use crate::runners::{self, RunnerKind};
use crate::theme;
use crate::widgets::{self, BtnKind};
use crate::{
    pick_file, split_lines, App, Confirm, ConfirmAction, Page, PrefixForm,
};

// ---------------------------------------------------------------------------
// Боковая навигация
// ---------------------------------------------------------------------------

pub(crate) fn nav(app: &mut App, ui: &mut Ui) {
    let logo = app.assets.logo.clone();
    ui.horizontal(|ui| {
        ui.add(egui::Image::new(&logo).fit_to_exact_size(vec2(34.0, 34.0)));
        ui.add_space(10.0);
        ui.vertical(|ui| {
            ui.label(RichText::new("rustauncher").size(16.0).strong());
            ui.label(
                RichText::new("Wine / Proton")
                    .size(10.5)
                    .color(theme::FAINT),
            );
        });
    });

    ui.add_space(theme::SP_XL);

    for page in Page::ALL {
        if widgets::nav_item(ui, page.icon(), page.title(), app.page == page).clicked() {
            app.page = page;
            if page == Page::Prefixes
                && app.selected_prefix.is_none()
                && !app.cfg.prefixes.is_empty()
            {
                app.selected_prefix = Some(0);
            }
        }
        ui.add_space(2.0);
    }

    ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
        let runner = app
            .cfg
            .active_runner
            .clone()
            .unwrap_or_else(|| "Системный Wine".to_string());
        egui::Frame::none()
            .fill(theme::CARD)
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(12.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.with_layout(Layout::top_down(Align::Min), |ui| {
                    ui.horizontal(|ui| {
                        icons::icon(ui, Icon::Bolt, 12.0, theme::ACCENT);
                        ui.label(
                            RichText::new("АКТИВНЫЙ РАННЕР")
                                .size(9.5)
                                .strong()
                                .color(theme::FAINT),
                        );
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(runner).size(12.5).color(theme::TEXT));
                    ui.label(
                        RichText::new(format!("системный: {}", app.sysver))
                            .size(10.0)
                            .color(theme::FAINT),
                    );
                });
            });
    });
}

// ---------------------------------------------------------------------------
// Приложения
// ---------------------------------------------------------------------------

struct AppRow {
    prefix_idx: usize,
    app_idx: usize,
    name: String,
    exe: String,
    prefix: String,
    winver: String,
}

pub(crate) fn apps(app: &mut App, ui: &mut Ui) {
    widgets::page_header(
        ui,
        "Приложения",
        "Все программы, привязанные к префиксам",
        |ui| {
            if widgets::button(ui, Some(Icon::Plus), "Добавить", BtnKind::Primary).clicked() {
                open_add_app(app);
            }
        },
    );
    ui.add_space(theme::SP_L);

    let mut rows: Vec<AppRow> = Vec::new();
    for (pi, prefix) in app.cfg.prefixes.iter().enumerate() {
        for (ai, a) in prefix.apps.iter().enumerate() {
            rows.push(AppRow {
                prefix_idx: pi,
                app_idx: ai,
                name: a.name.clone(),
                exe: a.exe.clone(),
                prefix: prefix.name.clone(),
                winver: prefix.windows_version.clone(),
            });
        }
    }
    rows.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let total = rows.len();
    let query = app.app_search.trim().to_lowercase();
    if !query.is_empty() {
        rows.retain(|r| {
            r.name.to_lowercase().contains(&query)
                || r.exe.to_lowercase().contains(&query)
                || r.prefix.to_lowercase().contains(&query)
        });
    }

    ui.horizontal(|ui| {
        icons::icon(ui, Icon::Search, 15.0, theme::FAINT);
        ui.add(
            TextEdit::singleline(&mut app.app_search)
                .hint_text("Поиск по приложению, exe или префиксу")
                .desired_width(340.0),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if total > 0 {
                ui.label(
                    RichText::new(format!("из {total}"))
                        .size(12.0)
                        .color(theme::FAINT),
                );
            }
            ui.label(
                RichText::new(format!("{}", rows.len()))
                    .size(12.0)
                    .color(theme::FAINT),
            );
        });
    });
    ui.add_space(theme::SP_L);

    if total == 0 {
        let image = app.assets.empty_apps.clone();
        widgets::empty_state(
            ui,
            &image,
            "Пока нет приложений",
            "Добавь .exe или просто перетащи файл в окно.",
            |ui| {
                if widgets::button(ui, Some(Icon::Plus), "Добавить приложение", BtnKind::Primary)
                    .clicked()
                {
                    open_add_app(app);
                }
            },
        );
        return;
    }

    if rows.is_empty() {
        let image = app.assets.empty_search.clone();
        widgets::empty_state(
            ui,
            &image,
            "Ничего не найдено",
            "Попробуй другой запрос или сбрось поиск.",
            |ui| {
                if widgets::button(ui, None, "Сбросить поиск", BtnKind::Secondary).clicked() {
                    app.app_search.clear();
                }
            },
        );
        return;
    }

    let avail = ui.available_width();
    let gap = 14.0;
    let min_w = 310.0;
    let cols = (((avail + gap) / (min_w + gap)).floor() as usize).max(1);
    let card_w = (avail - gap * (cols as f32 - 1.0)) / cols as f32;

    ui.spacing_mut().item_spacing.x = gap;
    for chunk in rows.chunks(cols) {
        ui.horizontal_top(|ui| {
            for row in chunk {
                ui.allocate_ui_with_layout(
                    vec2(card_w, 0.0),
                    Layout::top_down(Align::Min),
                    |ui| {
                        ui.set_width(card_w);
                        app_card(app, ui, row, card_w);
                    },
                );
            }
        });
        ui.add_space(gap);
    }
}

fn open_add_app(app: &mut App) {
    if app.cfg.prefixes.is_empty() {
        app.logln("Сначала создай префикс — открою диалог");
        app.page = Page::Prefixes;
        app.prefix_dialog = Some(PrefixForm::new());
        return;
    }
    let prefix = app
        .selected_prefix
        .filter(|i| *i < app.cfg.prefixes.len())
        .unwrap_or(0);
    app.open_app_dialog(prefix, None);
}

fn app_card(app: &mut App, ui: &mut Ui, row: &AppRow, width: f32) {
    widgets::card(ui, |ui| {
        ui.set_width(width - 2.0 * theme::SP_L);
        ui.set_min_height(110.0);

        ui.horizontal(|ui| {
            widgets::icon_tile(ui, Icon::Wine, 38.0);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.add(
                    Label::new(RichText::new(&row.name).size(15.0).strong()).truncate(),
                );
                ui.horizontal(|ui| {
                    widgets::pill_accent(ui, &row.prefix);
                    widgets::pill_muted(ui, &row.winver);
                });
            });
        });

        ui.add_space(theme::SP_S);
        ui.add(
            Label::new(
                RichText::new(exe_name(&row.exe))
                    .monospace()
                    .size(11.0)
                    .color(theme::FAINT),
            )
            .truncate(),
        );
        ui.add_space(theme::SP_M);

        ui.horizontal(|ui| {
            if widgets::button_h(ui, Some(Icon::Play), "Запустить", BtnKind::Primary, 30.0)
                .clicked()
            {
                app.launch_app(row.prefix_idx, row.app_idx);
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, Icon::Trash, "Удалить приложение", BtnKind::Danger)
                    .clicked()
                {
                    app.confirm = Some(Confirm {
                        title: "Удалить приложение?".to_string(),
                        text: format!(
                            "«{}» будет убрано из префикса «{}». Файлы на диске останутся.",
                            row.name, row.prefix
                        ),
                        confirm_label: "Удалить".to_string(),
                        danger: true,
                        delete_folder: false,
                        action: ConfirmAction::DeleteApp(row.prefix_idx, row.app_idx),
                    });
                }
                if widgets::icon_button(ui, Icon::Edit, "Изменить", BtnKind::Secondary)
                    .clicked()
                {
                    app.open_app_dialog(row.prefix_idx, Some(row.app_idx));
                }
            });
        });
    });
}

fn exe_name(exe: &str) -> String {
    Path::new(exe)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(exe)
        .to_string()
}

// ---------------------------------------------------------------------------
// Префиксы
// ---------------------------------------------------------------------------

pub(crate) fn prefixes(app: &mut App, ui: &mut Ui) {
    widgets::page_header(
        ui,
        "Префиксы",
        "Отдельные окружения Wine для разных игр",
        |ui| {
            if widgets::button(ui, Some(Icon::Plus), "Новый префикс", BtnKind::Primary).clicked() {
                app.prefix_dialog = Some(PrefixForm::new());
            }
        },
    );
    ui.add_space(theme::SP_L);

    if app.cfg.prefixes.is_empty() {
        let image = app.assets.empty_prefixes.clone();
        widgets::empty_state(
            ui,
            &image,
            "Нет префиксов",
            "Создай первый префикс — wineboot подготовит окружение.",
            |ui| {
                if widgets::button(ui, Some(Icon::Plus), "Создать префикс", BtnKind::Primary)
                    .clicked()
                {
                    app.prefix_dialog = Some(PrefixForm::new());
                }
            },
        );
        return;
    }

    if app.selected_prefix.map(|i| i >= app.cfg.prefixes.len()).unwrap_or(true) {
        app.selected_prefix = Some(0);
    }

    let list_w = 300.0;
    let avail_h = ui.available_height();
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            vec2(list_w, avail_h),
            Layout::top_down(Align::Min),
            |ui| {
                ui.set_width(list_w);
                ScrollArea::vertical()
                    .id_salt("prefix-list")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(list_w - 10.0);
                        for i in 0..app.cfg.prefixes.len() {
                            let selected = app.selected_prefix == Some(i);
                            let name = app.cfg.prefixes[i].name.clone();
                            let path = app.cfg.prefixes[i].path.clone();
                            let winver = app.cfg.prefixes[i].windows_version.clone();
                            let runner = prefix_runner_label(app, i);
                            let apps_count = app.cfg.prefixes[i].apps.len();
                            let (resp, _) = widgets::selectable_card(
                                ui,
                                ui.id().with(("prefix-card", i)),
                                selected,
                                |ui| {
                                    ui.horizontal(|ui| {
                                        ui.vertical(|ui| {
                                            ui.add(
                                                Label::new(
                                                    RichText::new(&name).size(14.0).strong(),
                                                )
                                                .truncate(),
                                            );
                                            ui.add(
                                                Label::new(
                                                    RichText::new(short_path(&path))
                                                        .monospace()
                                                        .size(10.0)
                                                        .color(theme::FAINT),
                                                )
                                                .truncate(),
                                            );
                                        });
                                        ui.with_layout(
                                            Layout::right_to_left(Align::Center),
                                            |ui| {
                                                widgets::pill_muted(ui, &winver);
                                            },
                                        );
                                    });
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        widgets::pill_muted(ui, &runner);
                                        widgets::pill_muted(ui, &format!("{apps_count} прил."));
                                    });
                                },
                            );
                            if resp.clicked() {
                                app.selected_prefix = Some(i);
                            }
                            ui.add_space(theme::SP_S);
                        }
                    });
            },
        );
        ui.add_space(theme::SP_L);
        ui.vertical(|ui| {
            match app.selected_prefix {
                Some(i) if i < app.cfg.prefixes.len() => prefix_detail(app, ui, i),
                _ => {
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            RichText::new("Выбери префикс слева")
                                .size(13.0)
                                .color(theme::FAINT),
                        );
                    });
                }
            }
        });
    });
}

fn prefix_runner_label(app: &App, i: usize) -> String {
    match &app.cfg.prefixes[i].runner {
        Some(name) => name.clone(),
        None => app
            .cfg
            .active_runner
            .clone()
            .unwrap_or_else(|| "Системный Wine".to_string()),
    }
}

fn short_path(path: &str) -> String {
    if let Some(home) = dirs::home_dir() {
        let home = home.to_string_lossy();
        if let Some(rest) = path.strip_prefix(home.as_ref()) {
            return format!("~{rest}");
        }
    }
    path.to_string()
}

fn prefix_detail(app: &mut App, ui: &mut Ui, i: usize) {
    let folder_path = app.cfg.prefixes[i].path.clone();
    let mut apply_ver: Option<String> = None;
    let mut tool: Option<&'static str> = None;
    let mut pick_and_run = false;
    let mut kill = false;
    let mut open_folder = false;
    let mut request_delete = false;

    let height = ui.available_height();
    ScrollArea::vertical()
        .id_salt(("prefix-detail", i))
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            let name = app.cfg.prefixes[i].name.clone();
            let path = app.cfg.prefixes[i].path.clone();
            let winver = app.cfg.prefixes[i].windows_version.clone();
            let runner = prefix_runner_label(app, i);
            let apps_count = app.cfg.prefixes[i].apps.len();

            ui.horizontal(|ui| {
                ui.label(RichText::new(&name).size(20.0).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(
                        ui,
                        Icon::Trash,
                        "Удалить префикс",
                        BtnKind::Danger,
                    )
                    .clicked()
                    {
                        request_delete = true;
                    }
                    if widgets::button(
                        ui,
                        Some(Icon::Folder),
                        "Открыть папку",
                        BtnKind::Secondary,
                    )
                    .clicked()
                    {
                        open_folder = true;
                    }
                });
            });
            ui.add(
                Label::new(
                    RichText::new(short_path(&path))
                        .monospace()
                        .size(11.0)
                        .color(theme::FAINT),
                )
                .truncate(),
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                widgets::pill_accent(ui, &winver);
                widgets::pill_muted(ui, &runner);
                widgets::pill_muted(ui, &format!("{apps_count} прил."));
            });
            ui.add_space(theme::SP_L);

            // ---- Основное -------------------------------------------------
            widgets::card(ui, |ui| {
                widgets::section_title(ui, Icon::Wine, "ОСНОВНОЕ");
                ui.add_space(theme::SP_XS);
                egui::Grid::new(("prefix-main", i))
                    .num_columns(2)
                    .spacing([theme::SP_M, theme::SP_S])
                    .show(ui, |ui| {
                        widgets::field_label(ui, "Имя");
                        if ui
                            .add(
                                TextEdit::singleline(&mut app.cfg.prefixes[i].name)
                                    .desired_width(280.0),
                            )
                            .changed()
                        {
                            app.dirty = true;
                        }
                        ui.end_row();

                        widgets::field_label(ui, "Раннер");
                        let cur = app.cfg.prefixes[i].runner.clone();
                        let label = cur
                            .clone()
                            .unwrap_or_else(|| "Как в глобальных".to_string());
                        egui::ComboBox::from_id_salt(("prefix-runner", i))
                            .selected_text(label)
                            .width(240.0)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(cur.is_none(), "Как в глобальных")
                                    .clicked()
                                {
                                    app.cfg.prefixes[i].runner = None;
                                    app.dirty = true;
                                }
                                for r in &app.runners {
                                    let sel = cur.as_deref() == Some(r.name.as_str());
                                    if ui.selectable_label(sel, r.name.as_str()).clicked() {
                                        app.cfg.prefixes[i].runner = Some(r.name.clone());
                                        app.dirty = true;
                                    }
                                }
                            });
                        ui.end_row();

                        widgets::field_label(ui, "Windows");
                        ui.horizontal(|ui| {
                            let ver = app.cfg.prefixes[i].windows_version.clone();
                            let mut chosen = ver.clone();
                            egui::ComboBox::from_id_salt(("prefix-winver", i))
                                .selected_text(ver.as_str())
                                .width(110.0)
                                .show_ui(ui, |ui| {
                                    for v in ["win11", "win10", "win8", "win7"] {
                                        ui.selectable_value(
                                            &mut chosen,
                                            v.to_string(),
                                            v,
                                        );
                                    }
                                });
                            if chosen != ver {
                                app.cfg.prefixes[i].windows_version = chosen.clone();
                                app.dirty = true;
                                apply_ver = Some(chosen);
                            }
                            if widgets::button(
                                ui,
                                Some(Icon::Check),
                                "Применить",
                                BtnKind::Secondary,
                            )
                            .clicked()
                            {
                                apply_ver =
                                    Some(app.cfg.prefixes[i].windows_version.clone());
                            }
                        });
                        ui.end_row();
                    });
            });
            ui.add_space(theme::SP_M);

            // ---- Окружение ------------------------------------------------
            widgets::card(ui, |ui| {
                widgets::section_title(ui, Icon::Sliders, "ОКРУЖЕНИЕ");
                ui.add_space(theme::SP_XS);
                widgets::field_label(ui, "DLL overrides — по одному на строку");
                let mut ov = app.cfg.prefixes[i].dll_overrides.join("\n");
                if ui
                    .add(
                        TextEdit::multiline(&mut ov)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("version=n,b"),
                    )
                    .changed()
                {
                    app.cfg.prefixes[i].dll_overrides = split_lines(&ov);
                    app.dirty = true;
                }
                ui.add_space(theme::SP_S);
                widgets::field_label(ui, "Переменные окружения — KEY=VALUE");
                let mut env = app.cfg.prefixes[i].env.join("\n");
                if ui
                    .add(
                        TextEdit::multiline(&mut env)
                            .desired_rows(3)
                            .desired_width(f32::INFINITY)
                            .hint_text("DXVK_HUD=fps"),
                    )
                    .changed()
                {
                    app.cfg.prefixes[i].env = split_lines(&env);
                    app.dirty = true;
                }
            });
            ui.add_space(theme::SP_M);

            // ---- Инструменты ---------------------------------------------
            widgets::card(ui, |ui| {
                widgets::section_title(ui, Icon::Bolt, "ИНСТРУМЕНТЫ");
                ui.add_space(theme::SP_XS);
                ui.horizontal_wrapped(|ui| {
                    if widgets::button(ui, Some(Icon::Sliders), "winecfg", BtnKind::Secondary)
                        .clicked()
                    {
                        tool = Some("winecfg");
                    }
                    if widgets::button(ui, Some(Icon::Edit), "regedit", BtnKind::Secondary)
                        .clicked()
                    {
                        tool = Some("regedit");
                    }
                    if widgets::button(ui, Some(Icon::Folder), "explorer", BtnKind::Secondary)
                        .clicked()
                    {
                        tool = Some("explorer");
                    }
                    if widgets::button(ui, Some(Icon::Play), "Запустить exe…", BtnKind::Secondary)
                        .clicked()
                    {
                        pick_and_run = true;
                    }
                    if widgets::button(ui, Some(Icon::Stop), "Остановить", BtnKind::Danger)
                        .clicked()
                    {
                        kill = true;
                    }
                });
            });
            ui.add_space(theme::SP_M);

            // ---- Приложения ----------------------------------------------
            widgets::card(ui, |ui| {
                ui.horizontal(|ui| {
                    icons::icon(ui, Icon::Apps, 14.0, theme::FAINT);
                    ui.label(
                        RichText::new("ПРИЛОЖЕНИЯ")
                            .size(11.0)
                            .strong()
                            .color(theme::FAINT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        widgets::pill_muted(ui, &format!("{apps_count}"));
                    });
                });
                ui.add_space(theme::SP_XS);

                if apps_count == 0 {
                    widgets::hint(
                        ui,
                        "Пока пусто — добавь exe ниже или перетащи файл в окно.",
                    );
                }

                for j in 0..apps_count {
                    let (app_name, app_exe) = {
                        let a = &app.cfg.prefixes[i].apps[j];
                        (a.name.clone(), a.exe.clone())
                    };
                    egui::Frame::none()
                        .fill(theme::INPUT)
                        .rounding(Rounding::same(theme::RADIUS_SM))
                        .inner_margin(Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                icons::icon(ui, Icon::Wine, 14.0, theme::MUTED);
                                ui.vertical(|ui| {
                                    ui.add(
                                        Label::new(
                                            RichText::new(&app_name).size(13.0).strong(),
                                        )
                                        .truncate(),
                                    );
                                    ui.add(
                                        Label::new(
                                            RichText::new(exe_name(&app_exe))
                                                .monospace()
                                                .size(10.0)
                                                .color(theme::FAINT),
                                        )
                                        .truncate(),
                                    );
                                });
                                ui.with_layout(
                                    Layout::right_to_left(Align::Center),
                                    |ui| {
                                        if widgets::icon_button(
                                            ui,
                                            Icon::Trash,
                                            "Убрать",
                                            BtnKind::Danger,
                                        )
                                        .clicked()
                                        {
                                            app.confirm = Some(Confirm {
                                                title: "Удалить приложение?".to_string(),
                                                text: format!(
                                                    "«{app_name}» будет убрано из префикса «{name}». Файлы на диске останутся."
                                                ),
                                                confirm_label: "Удалить".to_string(),
                                                danger: true,
                                                delete_folder: false,
                                                action: ConfirmAction::DeleteApp(i, j),
                                            });
                                        }
                                        if widgets::icon_button(
                                            ui,
                                            Icon::Edit,
                                            "Изменить",
                                            BtnKind::Secondary,
                                        )
                                        .clicked()
                                        {
                                            app.open_app_dialog(i, Some(j));
                                        }
                                        if widgets::button_h(
                                            ui,
                                            Some(Icon::Play),
                                            "Запустить",
                                            BtnKind::Primary,
                                            28.0,
                                        )
                                        .clicked()
                                        {
                                            app.launch_app(i, j);
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(6.0);
                }

                ui.add_space(theme::SP_S);
                if widgets::button(
                    ui,
                    Some(Icon::Plus),
                    "Добавить приложение",
                    BtnKind::Secondary,
                )
                .clicked()
                {
                    app.open_app_dialog(i, None);
                }
            });
            ui.add_space(theme::SP_M);

            // ---- Опасная зона ---------------------------------------------
            widgets::card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Удаление префикса")
                                .size(13.5)
                                .strong()
                                .color(theme::TEXT),
                        );
                        widgets::hint(
                            ui,
                            "Запись убирается из списка. Папку можно удалить вместе с префиксом.",
                        );
                    });
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if widgets::button(
                            ui,
                            Some(Icon::Trash),
                            "Удалить префикс",
                            BtnKind::Danger,
                        )
                        .clicked()
                        {
                            request_delete = true;
                        }
                    });
                });
            });
            ui.add_space(theme::SP_S);
        });

    if open_folder {
        app.open_path(Path::new(&folder_path));
    }
    if let Some(ver) = apply_ver {
        let prefix = app.cfg.prefixes[i].clone();
        app.apply_winver(prefix, ver);
    }
    if let Some(t) = tool {
        let prefix = app.cfg.prefixes[i].clone();
        let global = app.cfg.active_runner.clone();
        match crate::wine::spawn_tool(
            &prefix,
            global.as_deref(),
            t,
            app.log.clone(),
            app.ctx.clone(),
        ) {
            Ok(()) => app.logln(format!("{t} запущен")),
            Err(e) => app.logln(format!("Не смог запустить {t}: {e}")),
        }
    }
    if pick_and_run {
        if let Some(exe) = pick_file("") {
            let prefix = app.cfg.prefixes[i].clone();
            let global = app.cfg.active_runner.clone();
            if let Err(e) = crate::wine::run_exe(
                &prefix,
                global.as_deref(),
                &exe,
                app.log.clone(),
                app.ctx.clone(),
            ) {
                app.logln(format!("Не смог запустить {exe}: {e}"));
            } else {
                app.logln(format!("Запущен {exe}"));
            }
        }
    }
    if kill {
        let prefix = app.cfg.prefixes[i].clone();
        let global = app.cfg.active_runner.clone();
        match crate::wine::kill_prefix(&prefix, global.as_deref()) {
            Ok(()) => app.logln("Процессы префикса остановлены"),
            Err(e) => app.logln(format!("Не смог остановить процессы: {e}")),
        }
    }
    if request_delete {
        let name = app.cfg.prefixes[i].name.clone();
        app.confirm = Some(Confirm {
            title: "Удалить префикс?".to_string(),
            text: format!("«{name}» будет убран из списка."),
            confirm_label: "Удалить".to_string(),
            danger: true,
            delete_folder: false,
            action: ConfirmAction::DeletePrefix(i),
        });
    }
}

// ---------------------------------------------------------------------------
// Раннеры
// ---------------------------------------------------------------------------

pub(crate) fn runners(app: &mut App, ui: &mut Ui) {
    widgets::page_header(
        ui,
        "Раннеры",
        "Wine-GE и Proton-GE прямо с GitHub",
        |ui| {
            if widgets::button(ui, Some(Icon::Refresh), "Обновить", BtnKind::Ghost).clicked() {
                app.runners = runners::scan();
                app.logln(format!("Раннеров найдено: {}", app.runners.len()));
            }
        },
    );
    ui.add_space(theme::SP_L);

    let mut start: Option<RunnerKind> = None;
    let mut cancel = false;
    let mut dismiss = false;

    widgets::card(ui, |ui| {
        widgets::section_title(ui, Icon::Download, "ЗАГРУЗИТЬ РАННЕР");
        ui.add_space(theme::SP_XS);
        ui.label(
            RichText::new(
                "Берётся последний релиз с GitHub и распаковывается в папку раннеров.",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
        ui.add_space(theme::SP_S);
        ui.horizontal(|ui| {
            if widgets::button(ui, Some(Icon::Download), "wine-ge", BtnKind::Primary).clicked() {
                start = Some(RunnerKind::WineGe);
            }
            if widgets::button(ui, Some(Icon::Download), "Proton-GE", BtnKind::Primary)
                .clicked()
            {
                start = Some(RunnerKind::ProtonGe);
            }
        });

        if let Some(dl) = &app.dl {
            ui.add_space(theme::SP_M);
            let frac = match dl.total {
                Some(t) if t > 0 => (dl.got as f32 / t as f32).clamp(0.0, 1.0),
                _ => 0.0,
            };
            let text = match dl.total {
                Some(t) => format!(
                    "{} / {}",
                    runners::human_bytes(dl.got),
                    runners::human_bytes(t)
                ),
                None => runners::human_bytes(dl.got),
            };
            ui.add(
                egui::ProgressBar::new(frac)
                    .desired_width(f32::INFINITY)
                    .fill(theme::ACCENT)
                    .text(text),
            );
            ui.add_space(theme::SP_XS);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(if dl.finished {
                        "Готово"
                    } else {
                        dl.label.as_str()
                    })
                    .size(11.0)
                    .color(theme::FAINT),
                );
                if dl.speed > 0.0 && !dl.finished {
                    ui.label(
                        RichText::new(runners::human_speed(dl.speed))
                            .size(11.0)
                            .color(theme::FAINT),
                    );
                }
                if let (Some(total), false) = (dl.total, dl.finished) {
                    if dl.speed > 0.0 {
                        let left = total.saturating_sub(dl.got);
                        let eta = (left as f64 / dl.speed).round() as u64;
                        ui.label(
                            RichText::new(format!(
                                "осталось ~{}",
                                runners::human_eta(eta)
                            ))
                            .size(11.0)
                            .color(theme::FAINT),
                        );
                    }
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if dl.finished {
                        if widgets::button_h(ui, None, "Скрыть", BtnKind::Ghost, 26.0)
                            .clicked()
                        {
                            dismiss = true;
                        }
                    } else if widgets::button_h(
                        ui,
                        Some(Icon::Close),
                        "Отменить",
                        BtnKind::Danger,
                        26.0,
                    )
                    .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        }
    });

    if let Some(kind) = start {
        app.start_download(kind);
    }
    if cancel {
        if let Some(dl) = &app.dl {
            dl.cancel.store(true, Ordering::Relaxed);
        }
        app.logln("Отмена загрузки…");
    }
    if dismiss {
        app.dl = None;
    }

    ui.add_space(theme::SP_L);
    ui.horizontal(|ui| {
        widgets::section_title(ui, Icon::Bolt, "УСТАНОВЛЕННЫЕ");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            widgets::pill_muted(ui, &format!("{}", app.runners.len()));
        });
    });
    ui.add_space(theme::SP_S);

    if app.runners.is_empty() {
        widgets::card(ui, |ui| {
            ui.label(
                RichText::new("Свои раннеры не установлены")
                    .size(13.5)
                    .strong(),
            );
            widgets::hint(
                ui,
                "Используется системный Wine. Можно скачать wine-ge или Proton-GE выше.",
            );
        });
        return;
    }

    let names: Vec<String> = app.runners.iter().map(|r| r.name.clone()).collect();
    for name in names {
        let Some(runner) = app.runners.iter().find(|r| r.name == name).cloned() else {
            continue;
        };
        let active = app.cfg.active_runner.as_deref() == Some(runner.name.as_str());
        let mut pick = false;
        let mut remove = false;
        let mut open = false;

        widgets::card(ui, |ui| {
            ui.horizontal(|ui| {
                widgets::icon_tile(ui, Icon::Bolt, 36.0);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&runner.name)
                                .size(14.0)
                                .strong()
                                .color(theme::TEXT),
                        );
                        if active {
                            widgets::pill_accent(ui, "по умолчанию");
                        }
                    });
                    ui.add(
                        Label::new(
                            RichText::new(short_path(&runner.path.to_string_lossy()))
                                .monospace()
                                .size(10.0)
                                .color(theme::FAINT),
                        )
                        .truncate(),
                    );
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        widgets::pill_muted(ui, runner.kind);
                        widgets::pill_muted(ui, &runner.version);
                    });
                });
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if widgets::icon_button(ui, Icon::Trash, "Удалить раннер", BtnKind::Danger)
                        .clicked()
                    {
                        remove = true;
                    }
                    if widgets::icon_button(ui, Icon::Folder, "Открыть папку", BtnKind::Ghost)
                        .clicked()
                    {
                        open = true;
                    }
                    if !active
                        && widgets::button_h(
                            ui,
                            Some(Icon::Check),
                            "Выбрать",
                            BtnKind::Secondary,
                            28.0,
                        )
                        .clicked()
                    {
                        pick = true;
                    }
                });
            });
        });
        ui.add_space(theme::SP_S);

        if pick {
            app.cfg.active_runner = Some(runner.name.clone());
            app.dirty = true;
            app.logln(format!("Активный раннер: {}", runner.name));
            app.save();
        }
        if open {
            app.open_path(&runner.path);
        }
        if remove {
            app.confirm = Some(Confirm {
                title: "Удалить раннер?".to_string(),
                text: format!(
                    "Папка «{}» будет удалена с диска. Префиксы, использующие этот раннер, нужно будет переназначить.",
                    runner.name
                ),
                confirm_label: "Удалить".to_string(),
                danger: true,
                delete_folder: false,
                action: ConfirmAction::DeleteRunner(runner.name.clone()),
            });
        }
    }

    ui.add_space(theme::SP_M);
    ui.horizontal(|ui| {
        widgets::hint(ui, "Раннер по умолчанию меняется в «Настройках».");
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::button(ui, Some(Icon::Sliders), "Настройки", BtnKind::Ghost).clicked() {
                app.page = Page::Settings;
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Настройки
// ---------------------------------------------------------------------------

pub(crate) fn settings(app: &mut App, ui: &mut Ui) {
    widgets::page_header(
        ui,
        "Настройки",
        "Глобальный раннер, масштаб и пути к данным",
        |_ui| {},
    );
    ui.add_space(theme::SP_L);

    widgets::card(ui, |ui| {
        widgets::section_title(ui, Icon::Bolt, "РАННЕР ПО УМОЛЧАНИЮ");
        ui.add_space(theme::SP_XS);
        ui.label(
            RichText::new(
                "Используется для всех префиксов, где не выбран собственный раннер.",
            )
            .size(12.0)
            .color(theme::MUTED),
        );
        ui.add_space(theme::SP_S);
        let current = app.cfg.active_runner.clone();
        let label = current
            .clone()
            .unwrap_or_else(|| "Системный Wine".to_string());
        egui::ComboBox::from_id_salt("global-runner-settings")
            .selected_text(label)
            .width(300.0)
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(current.is_none(), "Системный Wine")
                    .clicked()
                {
                    app.cfg.active_runner = None;
                    app.dirty = true;
                }
                for r in &app.runners {
                    let sel = current.as_deref() == Some(r.name.as_str());
                    if ui.selectable_label(sel, r.name.as_str()).clicked() {
                        app.cfg.active_runner = Some(r.name.clone());
                        app.dirty = true;
                    }
                }
            });
    });
    ui.add_space(theme::SP_M);

    widgets::card(ui, |ui| {
        widgets::section_title(ui, Icon::Sliders, "ИНТЕРФЕЙС");
        ui.add_space(theme::SP_XS);
        ui.horizontal(|ui| {
            widgets::field_label(ui, "Масштаб");
            if widgets::button(ui, None, "−", BtnKind::Secondary).clicked() {
                set_zoom(app, app.ctx.zoom_factor() - 0.1);
            }
            let zoom = (app.ctx.zoom_factor() * 100.0).round() as i32;
            ui.add_sized(
                vec2(52.0, 24.0),
                Label::new(
                    RichText::new(format!("{zoom}%"))
                        .size(12.0)
                        .color(theme::TEXT),
                ),
            );
            if widgets::button(ui, None, "+", BtnKind::Secondary).clicked() {
                set_zoom(app, app.ctx.zoom_factor() + 0.1);
            }
            if widgets::button(ui, None, "Сбросить", BtnKind::Ghost).clicked() {
                set_zoom(app, 1.0);
            }
        });
        widgets::hint(ui, "Масштабируется весь интерфейс, включая шрифты.");
    });
    ui.add_space(theme::SP_M);

    widgets::card(ui, |ui| {
        widgets::section_title(ui, Icon::Folder, "РАСПОЛОЖЕНИЕ ДАННЫХ");
        ui.add_space(theme::SP_XS);
        let cfg_path = config::config_path();
        path_row(
            app,
            ui,
            "Конфиг",
            &cfg_path.to_string_lossy(),
            cfg_path.parent().unwrap_or(Path::new(".")),
        );
        path_row(
            app,
            ui,
            "Раннеры",
            &runners::runners_dir().to_string_lossy(),
            &runners::runners_dir(),
        );
        path_row(
            app,
            ui,
            "Префиксы",
            &runners::prefixes_dir().to_string_lossy(),
            &runners::prefixes_dir(),
        );
    });
    ui.add_space(theme::SP_M);

    widgets::card(ui, |ui| {
        widgets::section_title(ui, Icon::Info, "О СИСТЕМЕ");
        ui.add_space(theme::SP_XS);
        ui.label(
            RichText::new(format!("rustauncher {}", env!("CARGO_PKG_VERSION")))
                .size(13.0)
                .strong(),
        );
        ui.label(
            RichText::new(format!("Системный Wine: {}", app.sysver))
                .size(12.0)
                .color(theme::MUTED),
        );
        ui.label(
            RichText::new(format!(
                "Активный раннер: {}",
                app.cfg
                    .active_runner
                    .clone()
                    .unwrap_or_else(|| "Системный Wine".to_string())
            ))
            .size(12.0)
            .color(theme::MUTED),
        );
    });
}

fn path_row(app: &mut App, ui: &mut Ui, title: &str, path: &str, dir: &Path) {
    let mut open = false;
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).size(12.0).color(theme::MUTED));
            ui.add(
                Label::new(
                    RichText::new(short_path(path))
                        .monospace()
                        .size(11.0)
                        .color(theme::FAINT),
                )
                .truncate(),
            );
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if widgets::icon_button(ui, Icon::Folder, "Открыть папку", BtnKind::Ghost).clicked() {
                open = true;
            }
        });
    });
    ui.add_space(6.0);
    if open {
        app.open_dir(dir);
    }
}

fn set_zoom(app: &mut App, zoom: f32) {
    app.ctx.set_zoom_factor(zoom.clamp(0.7, 1.6));
}

// ---------------------------------------------------------------------------
// Панель событий
// ---------------------------------------------------------------------------

pub(crate) fn event_bar(app: &mut App, ui: &mut Ui) {
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0_f32, theme::BORDER_SOFT),
    );

    let mut clear = false;
    let mut toggle = false;
    let busy = app.busy();

    ui.horizontal(|ui| {
        let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        ui.painter().circle_filled(
            dot.center(),
            3.5,
            if busy { theme::ACCENT } else { theme::FAINT },
        );

        let text = app
            .last_log()
            .unwrap_or_else(|| "Событий пока нет".to_string());
        let width = (ui.available_width() - 320.0).max(120.0);
        ui.add_sized(
            vec2(width, 18.0),
            Label::new(RichText::new(text).size(12.0).color(theme::MUTED)).truncate(),
        );

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if app.log_expanded {
                "Свернуть"
            } else {
                "Журнал"
            };
            let icon = if app.log_expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            if widgets::button_h(ui, Some(icon), label, BtnKind::Ghost, 26.0).clicked() {
                toggle = true;
            }
            if widgets::button_h(ui, None, "Очистить", BtnKind::Ghost, 26.0).clicked() {
                clear = true;
            }
            ui.label(
                RichText::new(format!("{}", app.log_len()))
                    .size(11.0)
                    .color(theme::FAINT),
            );
            if busy {
                ui.spinner();
            }
        });
    });

    if clear {
        if let Ok(mut l) = app.log.lock() {
            l.clear();
        }
    }
    if toggle {
        app.log_expanded = !app.log_expanded;
    }

    if app.log_expanded {
        ui.add_space(theme::SP_XS);
        ScrollArea::vertical()
            .id_salt("event-log")
            .max_height(150.0)
            .auto_shrink([false, false])
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let text = app.log_text();
                ui.add(
                    Label::new(
                        RichText::new(text)
                            .monospace()
                            .size(11.5)
                            .color(theme::MUTED),
                    )
                    .wrap(),
                );
            });
    }
}
