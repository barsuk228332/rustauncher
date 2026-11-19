use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use crate::config::{self, AppEntry, Prefix};
use crate::runners;

pub type LogList = Arc<Mutex<Vec<String>>>;

/// Версия системного wine (или сообщение об ошибке)
pub fn system_version() -> String {
    match Command::new("wine").arg("--version").output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        Ok(_) => "wine есть, но не запускается".to_string(),
        Err(_) => "не найден".to_string(),
    }
}

/// Путь к бинарю wine для выбранного раннера
pub fn runner_bin(runner: Option<&str>) -> PathBuf {
    match runner {
        None => PathBuf::from("wine"),
        Some(name) => {
            let dir = runners::runners_dir().join(name);
            let plain = dir.join("bin").join("wine");
            if plain.exists() {
                plain
            } else {
                // Proton-GE: files/bin/wine
                dir.join("files").join("bin").join("wine")
            }
        }
    }
}

fn base_command(runner: Option<&str>, prefix: &Path) -> Command {
    let mut cmd = Command::new(runner_bin(runner));
    cmd.env("WINEPREFIX", prefix);
    cmd.env("WINEDEBUG", "-all");
    if let Some(name) = runner {
        let dir = runners::runners_dir().join(name);
        let old_path = std::env::var("PATH").unwrap_or_default();
        cmd.env(
            "PATH",
            format!("{}:{}", dir.join("bin").display(), old_path),
        );
        cmd.env("WINELOADER", runner_bin(Some(name)));
    }
    cmd
}

pub fn push_log(log: &LogList, ctx: &egui::Context, line: impl Into<String>) {
    if let Ok(mut l) = log.lock() {
        l.push(line.into());
        if l.len() > 600 {
            let extra = l.len() - 600;
            l.drain(0..extra);
        }
    }
    ctx.request_repaint();
}

fn spawn_reader<R>(reader: R, log: LogList, ctx: egui::Context, tag: String)
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let buf = BufReader::new(reader);
        for line in buf.lines().map_while(Result::ok) {
            if line.trim().is_empty() {
                continue;
            }
            push_log(&log, &ctx, format!("[{tag}] {line}"));
        }
    });
}

/// Дожидается завершения процесса в фоне, чтобы не копить зомби.
fn wait_child(mut child: std::process::Child, log: LogList, ctx: egui::Context, tag: String) {
    std::thread::spawn(move || match child.wait() {
        Ok(status) if status.success() => push_log(&log, &ctx, format!("[{tag}] завершён")),
        Ok(status) => push_log(&log, &ctx, format!("[{tag}] завершён с кодом {status}")),
        Err(e) => push_log(&log, &ctx, format!("[{tag}] не дождался завершения: {e}")),
    });
}

/// Применяет общие настройки запуска: раннер, overrides, env, рабочая папка.
fn prepare(
    prefix: &Prefix,
    global_runner: Option<&str>,
    app: Option<&AppEntry>,
) -> anyhow::Result<Command> {
    let path = Path::new(&prefix.path);
    if !path.exists() {
        anyhow::bail!("папка префикса не существует: {}", path.display());
    }

    let runner = prefix.runner.as_deref().or(global_runner);
    let mut cmd = base_command(runner, path);

    let mut overrides: Vec<String> = Vec::new();
    if let Some(v) = prefix.overrides_env() {
        overrides.push(v);
    }
    if let Some(v) = app.and_then(|a| a.overrides_env()) {
        overrides.push(v);
    }
    if !overrides.is_empty() {
        cmd.env("WINEDLLOVERRIDES", overrides.join(";"));
    }

    let mut envs = config::parse_env(&prefix.env);
    if let Some(a) = app {
        envs.extend(config::parse_env(&a.env));
    }
    if let Some(a) = app {
        if !a.workdir.trim().is_empty() {
            let wd = PathBuf::from(a.workdir.trim());
            if wd.exists() {
                cmd.current_dir(wd);
            }
        } else if let Some(parent) = Path::new(&a.exe).parent() {
            if parent.exists() {
                cmd.current_dir(parent);
            }
        }
    }
    for (k, v) in envs {
        cmd.env(k, v);
    }

    Ok(cmd)
}

/// Запуск приложения внутри префикса с захватом вывода.
pub fn launch(
    prefix: &Prefix,
    app: &AppEntry,
    global_runner: Option<&str>,
    log: LogList,
    ctx: egui::Context,
) -> anyhow::Result<()> {
    let mut cmd = prepare(prefix, global_runner, Some(app))?;
    cmd.arg(&app.exe);
    if !app.args.trim().is_empty() {
        for arg in shell_split(&app.args) {
            cmd.arg(arg);
        }
    }
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    push_log(
        &log,
        &ctx,
        format!("[{}] запущен, pid {}", app.name, child.id()),
    );
    if let Some(out) = child.stdout.take() {
        spawn_reader(out, log.clone(), ctx.clone(), app.name.clone());
    }
    if let Some(err) = child.stderr.take() {
        spawn_reader(err, log.clone(), ctx.clone(), app.name.clone());
    }
    wait_child(child, log, ctx, app.name.clone());
    Ok(())
}

/// Запуск произвольного exe (например, установщика) в префиксе.
pub fn run_exe(
    prefix: &Prefix,
    global_runner: Option<&str>,
    exe: &str,
    log: LogList,
    ctx: egui::Context,
) -> anyhow::Result<()> {
    let app = AppEntry {
        name: Path::new(exe)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("exe")
            .to_string(),
        exe: exe.to_string(),
        ..Default::default()
    };
    launch(prefix, &app, global_runner, log, ctx)
}

/// Создание префикса: wineboot + выставление версии Windows.
/// Возвращает false, если wineboot не удался.
pub fn create_prefix(
    path: &Path,
    runner: Option<String>,
    winver: String,
    log: LogList,
    ctx: egui::Context,
) -> bool {
    if let Err(e) = std::fs::create_dir_all(path) {
        push_log(
            &log,
            &ctx,
            format!("Не смог создать папку {}: {e}", path.display()),
        );
        return false;
    }

    push_log(&log, &ctx, format!("wineboot для {}", path.display()));
    let mut cmd = base_command(runner.as_deref(), path);
    // Не даём wine лезть за mono/gecko
    cmd.env("WINEDLLOVERRIDES", "mscoree,mshtml=");
    cmd.arg("wineboot").arg("-u");
    match cmd.output() {
        Ok(out) if out.status.success() => push_log(&log, &ctx, "wineboot завершён"),
        Ok(out) => {
            push_log(
                &log,
                &ctx,
                format!(
                    "wineboot вернул {}: {}",
                    out.status,
                    String::from_utf8_lossy(&out.stderr).trim()
                ),
            );
            return false;
        }
        Err(e) => {
            push_log(&log, &ctx, format!("Не смог запустить wineboot: {e}"));
            return false;
        }
    }

    set_winver(path, runner.as_deref(), &winver, &log, &ctx);
    push_log(&log, &ctx, "Готово ✅");
    true
}

fn set_winver(path: &Path, runner: Option<&str>, winver: &str, log: &LogList, ctx: &egui::Context) {
    push_log(log, ctx, format!("Выставляю версию Windows: {winver}"));
    let mut cmd = base_command(runner, path);
    cmd.args([
        "reg",
        "add",
        "HKCU\\Software\\Wine",
        "/v",
        "Version",
        "/d",
        winver,
        "/f",
    ]);
    match cmd.output() {
        Ok(out) if out.status.success() => push_log(log, ctx, "Версия Windows применена"),
        Ok(out) => push_log(
            log,
            ctx,
            format!(
                "reg add вернул {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        ),
        Err(e) => push_log(log, ctx, format!("Не смог применить версию Windows: {e}")),
    }
}

/// Применить версию Windows к уже созданному префиксу (в фоне).
pub fn apply_winver(
    prefix: &Prefix,
    global_runner: Option<String>,
    winver: String,
    log: LogList,
    ctx: egui::Context,
) {
    let runner = prefix.runner.clone().or(global_runner);
    set_winver(Path::new(&prefix.path), runner.as_deref(), &winver, &log, &ctx);
}

/// Запуск стандартной утилиты wine: winecfg, regedit, explorer.
pub fn spawn_tool(
    prefix: &Prefix,
    global_runner: Option<&str>,
    tool: &str,
    log: LogList,
    ctx: egui::Context,
) -> anyhow::Result<()> {
    let mut cmd = prepare(prefix, global_runner, None)?;
    cmd.arg(tool);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let tag = tool.to_string();
    push_log(&log, &ctx, format!("[{tag}] запущен, pid {}", child.id()));
    if let Some(out) = child.stdout.take() {
        spawn_reader(out, log.clone(), ctx.clone(), tag.clone());
    }
    if let Some(err) = child.stderr.take() {
        spawn_reader(err, log.clone(), ctx.clone(), tag.clone());
    }
    wait_child(child, log, ctx, tag);
    Ok(())
}

/// Убить все процессы префикса через wineserver -k.
pub fn kill_prefix(prefix: &Prefix, global_runner: Option<&str>) -> anyhow::Result<()> {
    let runner = prefix.runner.as_deref().or(global_runner);
    let wineserver = runner_bin(runner)
        .parent()
        .map(|p| p.join("wineserver"))
        .unwrap_or_else(|| PathBuf::from("wineserver"));
    let mut cmd = Command::new(wineserver);
    cmd.env("WINEPREFIX", &prefix.path);
    cmd.env("WINEDEBUG", "-all");
    cmd.arg("-k");
    let status = cmd.status()?;
    if !status.success() {
        anyhow::bail!("wineserver -k вернул {status}");
    }
    Ok(())
}

/// Простейший разбор аргументов с поддержкой кавычек.
pub fn shell_split(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for ch in input.chars() {
        match (ch, quote) {
            ('"' | '\'', None) => quote = Some(ch),
            (c, Some(q)) if c == q => quote = None,
            (' ', None) => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (c, _) => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}
