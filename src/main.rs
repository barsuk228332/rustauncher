mod config;
mod dialogs;
mod icons;
mod pages;
mod runners;
mod theme;
mod widgets;
mod wine;

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Instant;

use config::{AppEntry, Config, Prefix};
use icons::Icon;
use runners::{DlMsg, Runner, RunnerKind};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 820.0])
            .with_min_inner_size([980.0, 620.0]),
        ..Default::default()
    };

    eframe::run_native(
        "rustauncher",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(App::new(cc.egui_ctx.clone())))
        }),
    )
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) enum Page {
    Apps,
    Prefixes,
    Runners,
    Settings,
}

impl Page {
    pub(crate) const ALL: [Page; 4] = [
        Page::Apps,
        Page::Prefixes,
        Page::Runners,
        Page::Settings,
    ];

    pub(crate) fn title(self) -> &'static str {
        match self {
            Page::Apps => "Приложения",
            Page::Prefixes => "Префиксы",
            Page::Runners => "Раннеры",
            Page::Settings => "Настройки",
        }
    }

    pub(crate) fn icon(self) -> Icon {
        match self {
            Page::Apps => Icon::Apps,
            Page::Prefixes => Icon::Layers,
            Page::Runners => Icon::Bolt,
            Page::Settings => Icon::Sliders,
        }
    }
}

pub(crate) struct Dl {
    rx: mpsc::Receiver<DlMsg>,
    cancel: Arc<AtomicBool>,
    pub(crate) label: String,
    pub(crate) got: u64,
    pub(crate) total: Option<u64>,
    pub(crate) speed: f64,
    last_tick: Instant,
    last_got: u64,
    pub(crate) finished: bool,
}

#[derive(Clone, Default)]
pub(crate) struct PrefixForm {
    pub(crate) name: String,
    pub(crate) dir: String,
    pub(crate) winver: String,
}

impl PrefixForm {
    pub(crate) fn new() -> Self {
        Self {
            winver: "win10".to_string(),
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub(crate) struct AppForm {
    pub(crate) prefix: usize,
    pub(crate) index: Option<usize>,
    pub(crate) name: String,
    pub(crate) exe: String,
    pub(crate) workdir: String,
    pub(crate) args: String,
    pub(crate) env: String,
    pub(crate) dll: String,
}

impl AppForm {
    pub(crate) fn new(prefix: usize) -> Self {
        Self {
            prefix,
            index: None,
            name: String::new(),
            exe: String::new(),
            workdir: String::new(),
            args: String::new(),
            env: String::new(),
            dll: String::new(),
        }
    }

    pub(crate) fn from_app(prefix: usize, index: usize, app: &AppEntry) -> Self {
        Self {
            prefix,
            index: Some(index),
            name: app.name.clone(),
            exe: app.exe.clone(),
            workdir: app.workdir.clone(),
            args: app.args.clone(),
            env: app.env.join("\n"),
            dll: app.dll_overrides.join("\n"),
        }
    }
}

#[derive(Clone)]
pub(crate) enum ConfirmAction {
    DeletePrefix(usize),
    DeleteApp(usize, usize),
    DeleteRunner(String),
}

#[derive(Clone)]
pub(crate) struct Confirm {
    pub(crate) title: String,
    pub(crate) text: String,
    pub(crate) confirm_label: String,
    pub(crate) danger: bool,
    pub(crate) delete_folder: bool,
    pub(crate) action: ConfirmAction,
}

pub(crate) struct App {
    pub(crate) cfg: Config,
    pub(crate) ctx: egui::Context,

    pub(crate) page: Page,
    pub(crate) selected_prefix: Option<usize>,
    pub(crate) app_search: String,

    pub(crate) runners: Vec<Runner>,
    pub(crate) sysver: String,

    pub(crate) dl: Option<Dl>,
    pub(crate) log: Arc<Mutex<Vec<String>>>,
    pub(crate) log_expanded: bool,
    pub(crate) pending_prefix: Option<(String, mpsc::Receiver<bool>)>,

    pub(crate) prefix_dialog: Option<PrefixForm>,
    pub(crate) app_dialog: Option<AppForm>,
    pub(crate) confirm: Option<Confirm>,

    pub(crate) dirty: bool,
}

impl App {
    fn new(ctx: egui::Context) -> Self {
        let cfg = Config::load();
        let runners = runners::scan();
        let sysver = wine::system_version();
        let selected_prefix = if cfg.prefixes.is_empty() {
            None
        } else {
            Some(0)
        };

        let mut app = App {
            cfg,
            ctx,
            page: Page::Apps,
            selected_prefix,
            app_search: String::new(),
            runners,
            sysver,
            dl: None,
            log: Arc::new(Mutex::new(Vec::new())),
            log_expanded: false,
            pending_prefix: None,
            prefix_dialog: None,
            app_dialog: None,
            confirm: None,
            dirty: false,
        };

        if let Some(name) = app.cfg.active_runner.clone() {
            if !app.runners.iter().any(|r| r.name == name) {
                app.cfg.active_runner = None;
            }
        }

        app.logln(format!("Системный Wine: {}", app.sysver));
        app.logln(format!("Раннеров установлено: {}", app.runners.len()));
        app
    }

    pub(crate) fn logln(&mut self, s: impl Into<String>) {
        if let Ok(mut l) = self.log.lock() {
            l.push(s.into());
            if l.len() > 600 {
                let extra = l.len() - 600;
                l.drain(0..extra);
            }
        }
        self.ctx.request_repaint();
    }

    pub(crate) fn last_log(&self) -> Option<String> {
        self.log.lock().ok().and_then(|l| l.last().cloned())
    }

    pub(crate) fn log_len(&self) -> usize {
        self.log.lock().map(|l| l.len()).unwrap_or(0)
    }

    pub(crate) fn log_text(&self) -> String {
        self.log.lock().map(|l| l.join("\n")).unwrap_or_default()
    }

    pub(crate) fn busy(&self) -> bool {
        self.pending_prefix.is_some() || self.dl.as_ref().map(|d| !d.finished).unwrap_or(false)
    }

    pub(crate) fn save(&mut self) {
        if let Err(e) = self.cfg.save() {
            self.logln(format!("Ошибка сохранения конфига: {e}"));
        } else {
            self.dirty = false;
        }
    }

    fn poll_download(&mut self, ctx: &egui::Context) {
        let mut logs: Vec<String> = Vec::new();
        let mut done = false;

        if let Some(dl) = self.dl.as_mut() {
            while let Ok(msg) = dl.rx.try_recv() {
                match msg {
                    DlMsg::Progress { got, total, label } => {
                        let now = Instant::now();
                        let dt = now.duration_since(dl.last_tick).as_secs_f64();
                        if dt > 0.3 {
                            dl.speed = got.saturating_sub(dl.last_got) as f64 / dt;
                            dl.last_tick = now;
                            dl.last_got = got;
                        }
                        dl.got = got;
                        dl.total = total;
                        dl.label = label;
                    }
                    DlMsg::Log(s) => logs.push(s),
                    DlMsg::Done(s) => {
                        logs.push(s);
                        done = true;
                    }
                    DlMsg::Error(e) => {
                        logs.push(format!("Ошибка: {e}"));
                        done = true;
                    }
                }
            }
            if done {
                dl.finished = true;
            }
        }

        for line in logs {
            self.logln(line);
        }
        if done {
            self.runners = runners::scan();
        }

        if self.dl.as_ref().map(|d| !d.finished).unwrap_or(false) {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }

    fn poll_prefix_creation(&mut self) {
        let (path, result) = match self.pending_prefix.as_ref() {
            Some((path, rx)) => match rx.try_recv() {
                Ok(ok) => (path.clone(), ok),
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => (path.clone(), false),
            },
            None => return,
        };
        self.pending_prefix = None;

        if result {
            return;
        }
        if let Some(idx) = self.cfg.prefixes.iter().position(|p| p.path == path) {
            let removed = self.cfg.prefixes.remove(idx);
            if self.selected_prefix == Some(idx) {
                self.selected_prefix = if self.cfg.prefixes.is_empty() {
                    None
                } else {
                    Some(idx.min(self.cfg.prefixes.len() - 1))
                };
            } else if let Some(sel) = self.selected_prefix {
                if sel > idx {
                    self.selected_prefix = Some(sel - 1);
                }
            }
            self.save();
            self.logln(format!(
                "Префикс «{}» не создан и убран из списка",
                removed.name
            ));
        }
    }

    fn start_download(&mut self, kind: RunnerKind) {
        if self.dl.as_ref().map(|d| !d.finished).unwrap_or(false) {
            self.logln("Дождись окончания текущей загрузки");
            return;
        }
        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        self.dl = Some(Dl {
            rx,
            cancel: cancel.clone(),
            label: String::new(),
            got: 0,
            total: None,
            speed: 0.0,
            last_tick: Instant::now(),
            last_got: 0,
            finished: false,
        });
        self.logln(format!("Устанавливаю {}", kind.title()));
        thread::spawn(move || runners::install(kind, tx, cancel));
    }

    pub(crate) fn create_prefix_from_dialog(&mut self) {
        let Some(form) = self.prefix_dialog.clone() else {
            return;
        };
        if self.pending_prefix.is_some() {
            self.logln("Дождись окончания создания текущего префикса");
            return;
        }

        let name = form.name.trim().to_string();
        if name.is_empty() {
            self.logln("Укажи имя префикса");
            return;
        }
        let path = if form.dir.trim().is_empty() {
            runners::prefixes_dir().join(&name)
        } else {
            PathBuf::from(form.dir.trim())
        };
        if let Some(existing) = self
            .cfg
            .prefixes
            .iter()
            .find(|p| Path::new(&p.path) == path)
        {
            self.logln(format!("Такая папка уже занята префиксом «{}»", existing.name));
            return;
        }

        let winver = if form.winver.trim().is_empty() {
            "win10".to_string()
        } else {
            form.winver.trim().to_string()
        };
        let path_str = path.to_string_lossy().to_string();
        self.cfg.prefixes.push(Prefix {
            name: name.clone(),
            path: path_str.clone(),
            runner: None,
            windows_version: winver.clone(),
            dll_overrides: Vec::new(),
            env: Vec::new(),
            apps: Vec::new(),
        });
        self.selected_prefix = Some(self.cfg.prefixes.len() - 1);
        self.save();
        self.prefix_dialog = None;

        let runner = self.cfg.active_runner.clone();
        let log = self.log.clone();
        let ctx = self.ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.pending_prefix = Some((path_str, rx));
        self.logln(format!("Создаю префикс «{name}» — это может занять минуту…"));
        thread::spawn(move || {
            let ok = wine::create_prefix(&path, runner, winver, log, ctx);
            let _ = tx.send(ok);
        });
    }

    pub(crate) fn launch_app(&mut self, i: usize, j: usize) {
        let Some(prefix) = self.cfg.prefixes.get(i).cloned() else {
            return;
        };
        let Some(app) = prefix.apps.get(j).cloned() else {
            return;
        };
        let global = self.cfg.active_runner.clone();
        match wine::launch(
            &prefix,
            &app,
            global.as_deref(),
            self.log.clone(),
            self.ctx.clone(),
        ) {
            Ok(_) => self.logln(format!("Запущен «{}»", app.name)),
            Err(e) => self.logln(format!("Не смог запустить «{}»: {e}", app.name)),
        }
    }

    pub(crate) fn apply_winver(&mut self, prefix: Prefix, ver: String) {
        let log = self.log.clone();
        let ctx = self.ctx.clone();
        let global = self.cfg.active_runner.clone();
        self.logln(format!("Применяю версию Windows {ver}…"));
        thread::spawn(move || wine::apply_winver(&prefix, global, ver, log, ctx));
    }

    pub(crate) fn open_app_dialog(&mut self, prefix: usize, index: Option<usize>) {
        let Some(p) = self.cfg.prefixes.get(prefix) else {
            return;
        };
        let form = match index {
            Some(j) if j < p.apps.len() => AppForm::from_app(prefix, j, &p.apps[j]),
            _ => AppForm::new(prefix),
        };
        self.app_dialog = Some(form);
    }

    pub(crate) fn save_app_dialog(&mut self) {
        let Some(form) = self.app_dialog.clone() else {
            return;
        };
        let name = form.name.trim().to_string();
        let exe = form.exe.trim().to_string();
        if name.is_empty() || exe.is_empty() {
            self.logln("Заполни имя и путь к exe");
            return;
        }
        if form.prefix >= self.cfg.prefixes.len() {
            self.logln("Префикс для приложения не найден");
            self.app_dialog = None;
            return;
        }
        let entry = AppEntry {
            name,
            exe,
            args: form.args.trim().to_string(),
            workdir: form.workdir.trim().to_string(),
            env: split_lines(&form.env),
            dll_overrides: split_lines(&form.dll),
        };
        match form.index {
            Some(j) => {
                if let Some(slot) = self.cfg.prefixes[form.prefix].apps.get_mut(j) {
                    *slot = entry;
                } else {
                    self.cfg.prefixes[form.prefix].apps.push(entry);
                }
            }
            None => self.cfg.prefixes[form.prefix].apps.push(entry),
        }
        self.app_dialog = None;
        self.dirty = true;
        self.save();
    }

    pub(crate) fn apply_confirm(&mut self) {
        let Some(c) = self.confirm.take() else {
            return;
        };
        match c.action {
            ConfirmAction::DeletePrefix(i) => {
                if i >= self.cfg.prefixes.len() {
                    return;
                }
                let removed = self.cfg.prefixes.remove(i);
                if c.delete_folder {
                    self.logln(format!("Удаляю папку {}", removed.path));
                    if let Err(e) = std::fs::remove_dir_all(&removed.path) {
                        self.logln(format!("Не смог удалить папку: {e}"));
                    }
                }
                self.selected_prefix = if self.cfg.prefixes.is_empty() {
                    None
                } else {
                    Some(i.min(self.cfg.prefixes.len() - 1))
                };
                self.logln(format!("Префикс «{}» убран", removed.name));
                self.dirty = true;
                self.save();
            }
            ConfirmAction::DeleteApp(pi, ai) => {
                if let Some(prefix) = self.cfg.prefixes.get_mut(pi) {
                    if ai < prefix.apps.len() {
                        let removed = prefix.apps.remove(ai);
                        self.logln(format!("Приложение «{}» убрано", removed.name));
                        self.dirty = true;
                        self.save();
                    }
                }
            }
            ConfirmAction::DeleteRunner(name) => {
                let path = self
                    .runners
                    .iter()
                    .find(|r| r.name == name)
                    .map(|r| r.path.clone());
                if let Some(path) = path {
                    match runners::delete_runner(&path) {
                        Ok(_) => self.logln(format!("Раннер «{name}» удалён")),
                        Err(e) => self.logln(format!("Не смог удалить «{name}»: {e}")),
                    }
                }
                if self.cfg.active_runner.as_deref() == Some(name.as_str()) {
                    self.cfg.active_runner = None;
                    self.dirty = true;
                }
                self.runners = runners::scan();
                self.save();
            }
        }
    }

    pub(crate) fn open_path(&mut self, path: &Path) {
        if let Err(e) = std::process::Command::new("xdg-open").arg(path).spawn() {
            self.logln(format!("Не смог открыть {}: {e}", path.display()));
        }
    }

    pub(crate) fn open_dir(&mut self, path: &Path) {
        if let Err(e) = std::fs::create_dir_all(path) {
            self.logln(format!("Не смог создать {}: {e}", path.display()));
            return;
        }
        self.open_path(path);
    }

    fn handle_dropped(&mut self, ctx: &egui::Context) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        let Some(path) = dropped.into_iter().next() else {
            return;
        };
        let is_exe = path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);
        if !is_exe {
            self.logln("Перетащи .exe файл");
            return;
        }
        if self.cfg.prefixes.is_empty() {
            self.logln("Сначала создай префикс — открою диалог");
            self.page = Page::Prefixes;
            self.prefix_dialog = Some(PrefixForm::new());
            return;
        }
        let prefix = self
            .selected_prefix
            .filter(|i| *i < self.cfg.prefixes.len())
            .unwrap_or(0);
        let mut form = AppForm::new(prefix);
        form.exe = path.to_string_lossy().to_string();
        form.name = file_stem(&form.exe);
        self.app_dialog = Some(form);
        self.page = Page::Apps;
        self.logln("Файл добавлен перетаскиванием");
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_download(ctx);
        self.poll_prefix_creation();
        if self.pending_prefix.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(300));
        }
        self.handle_dropped(ctx);

        egui::TopBottomPanel::bottom("events")
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin {
                        left: 16.0,
                        right: 16.0,
                        top: 8.0,
                        bottom: 8.0,
                    }),
            )
            .show(ctx, |ui| pages::event_bar(self, ui));

        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(theme::NAV_WIDTH)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL)
                    .inner_margin(egui::Margin {
                        left: 12.0,
                        right: 12.0,
                        top: 16.0,
                        bottom: 12.0,
                    }),
            )
            .show(ctx, |ui| pages::nav(self, ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(theme::BG)
                    .inner_margin(egui::Margin::same(theme::SP_XL)),
            )
            .show(ctx, |ui| match self.page {
                Page::Apps => {
                    egui::ScrollArea::vertical()
                        .id_salt("apps-page")
                        .auto_shrink([false, false])
                        .show(ui, |ui| pages::apps(self, ui));
                }
                Page::Prefixes => pages::prefixes(self, ui),
                Page::Runners => {
                    egui::ScrollArea::vertical()
                        .id_salt("runners-page")
                        .auto_shrink([false, false])
                        .show(ui, |ui| pages::runners(self, ui));
                }
                Page::Settings => {
                    egui::ScrollArea::vertical()
                        .id_salt("settings-page")
                        .auto_shrink([false, false])
                        .show(ui, |ui| pages::settings(self, ui));
                }
            });

        dialogs::show(self, ctx);

        if self.dirty {
            self.save();
        }
    }
}

pub(crate) fn split_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn file_stem(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("app")
        .to_string()
}

/// Файловый диалог через kdialog (KDE), с откатом на zenity.
pub(crate) fn pick_file(start: &str) -> Option<String> {
    let dir = if start.is_empty() {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    } else {
        start.to_string()
    };

    if let Ok(out) = std::process::Command::new("kdialog")
        .args(["--getopenfilename", &dir, "*.exe *.EXE *.msi *.bat|Программы"])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    if let Ok(out) = std::process::Command::new("zenity")
        .args(["--file-selection", &format!("--filename={dir}/")])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

pub(crate) fn pick_dir(start: &str) -> Option<String> {
    let dir = if start.is_empty() {
        dirs::home_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string())
    } else {
        start.to_string()
    };

    if let Ok(out) = std::process::Command::new("kdialog")
        .args(["--getexistingdirectory", &dir])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    if let Ok(out) = std::process::Command::new("zenity")
        .args(["--file-selection", "--directory", &format!("--filename={dir}/")])
        .output()
    {
        if out.status.success() {
            let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }
    None
}

fn setup_fonts(ctx: &egui::Context) {
    let mut candidates: Vec<String> = vec![
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_string(),
        "/usr/share/fonts/google-noto/NotoSans-Regular.ttf".to_string(),
        "/usr/share/fonts/noto/NotoSans-Regular.ttf".to_string(),
    ];

    if let Ok(out) = std::process::Command::new("fc-match")
        .args(["-f", "%{file}", "sans"])
        .output()
    {
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !path.is_empty() {
            candidates.insert(0, path);
        }
    }

    for path in candidates {
        if let Ok(bytes) = std::fs::read(&path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("system".to_owned(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, "system".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
}
