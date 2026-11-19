use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub fn data_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("rustauncher")
}

pub fn runners_dir() -> PathBuf {
    data_dir().join("runners")
}

pub fn prefixes_dir() -> PathBuf {
    data_dir().join("prefixes")
}

#[derive(Clone, Debug)]
pub struct Runner {
    pub name: String,
    pub path: PathBuf,
    /// "wine-ge" | "proton-ge" | "custom"
    pub kind: &'static str,
    /// Вывод `wine --version`
    pub version: String,
}

fn wine_binary(path: &Path) -> Option<PathBuf> {
    let plain = path.join("bin/wine");
    if plain.exists() {
        return Some(plain);
    }
    let proton = path.join("files/bin/wine");
    if proton.exists() {
        return Some(proton);
    }
    None
}

fn detect_version(path: &Path) -> String {
    let Some(bin) = wine_binary(path) else {
        return "-".to_string();
    };
    match Command::new(bin).arg("--version").output() {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        }
        _ => "?".to_string(),
    }
}

/// Сканирует установленные раннеры в ~/.local/share/rustauncher/runners
pub fn scan() -> Vec<Runner> {
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(runners_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) if !n.starts_with('.') => n.to_string(),
            _ => continue,
        };
        if wine_binary(&path).is_none() {
            continue;
        }
        let lower = name.to_lowercase();
        let kind = if lower.contains("proton") {
            "proton-ge"
        } else if lower.contains("wine") {
            "wine-ge"
        } else {
            "custom"
        };
        let version = detect_version(&path);
        out.push(Runner {
            name,
            path,
            kind,
            version,
        });
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn delete_runner(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunnerKind {
    WineGe,
    ProtonGe,
}

impl RunnerKind {
    pub fn repo(self) -> &'static str {
        match self {
            RunnerKind::WineGe => "GloriousEggroll/wine-ge-custom",
            RunnerKind::ProtonGe => "GloriousEggroll/proton-ge-custom",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            RunnerKind::WineGe => "wine-ge",
            RunnerKind::ProtonGe => "Proton-GE",
        }
    }
}

pub enum DlMsg {
    Progress {
        got: u64,
        total: Option<u64>,
        label: String,
    },
    Log(String),
    Done(String),
    Error(String),
}

/// Ставит раннер в фоновом потоке, присылая прогресс в канал.
pub fn install(kind: RunnerKind, tx: Sender<DlMsg>, cancel: Arc<AtomicBool>) {
    let send = |m: DlMsg| {
        let _ = tx.send(m);
    };
    send(DlMsg::Log(format!(
        "{}: запрашиваю последнюю версию на GitHub…",
        kind.title()
    )));

    match install_inner(kind, &tx, &cancel) {
        Ok(name) => send(DlMsg::Done(format!("Установлен раннер: {name}"))),
        Err(e) => send(DlMsg::Error(format!("{e:#}"))),
    }
}

fn install_inner(
    kind: RunnerKind,
    tx: &Sender<DlMsg>,
    cancel: &AtomicBool,
) -> anyhow::Result<String> {
    let (tag, url, file_name) = latest_asset(kind.repo())?;
    let _ = tx.send(DlMsg::Log(format!("Релиз: {tag}, файл: {file_name}")));

    let dir = runners_dir();
    fs::create_dir_all(&dir)?;

    let tmp = dir.join(format!(".tmp-{}", std::process::id()));
    if tmp.exists() {
        fs::remove_dir_all(&tmp).ok();
    }
    fs::create_dir_all(&tmp)?;

    let archive = tmp.join(&file_name);
    download(&url, &archive, tx, &file_name, cancel)?;

    if cancel.load(Ordering::Relaxed) {
        anyhow::bail!("отменено");
    }

    let extract_dir = tmp.join("extract");
    fs::create_dir_all(&extract_dir)?;

    let _ = tx.send(DlMsg::Log("Распаковываю…".to_string()));
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(&extract_dir)
        .status()?;
    if !status.success() {
        anyhow::bail!("tar завершился с ошибкой: {status}");
    }

    // Внутри архива обычно одна папка верхнего уровня
    let mut top: Vec<PathBuf> = fs::read_dir(&extract_dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    if top.len() != 1 {
        anyhow::bail!("неожиданная структура архива (ожидал одну папку)");
    }
    let src = top.remove(0);
    let name = src
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&tag)
        .to_string();
    let dest = dir.join(&name);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::rename(&src, &dest)?;

    fs::remove_dir_all(&tmp).ok();
    Ok(name)
}

fn latest_asset(repo: &str) -> anyhow::Result<(String, String, String)> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = ureq::get(&url)
        .set("User-Agent", "rustauncher")
        .set("Accept", "application/vnd.github+json")
        .call()?;
    let json: serde_json::Value = resp.into_json()?;

    let tag = json["tag_name"].as_str().unwrap_or("latest").to_string();
    let assets = json["assets"].as_array().cloned().unwrap_or_default();
    for asset in assets {
        let name = asset["name"].as_str().unwrap_or("");
        let is_archive = name.ends_with(".tar.xz") || name.ends_with(".tar.gz");
        if is_archive && !name.contains("sha") {
            let url = asset["browser_download_url"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            if !url.is_empty() {
                return Ok((tag, url, name.to_string()));
            }
        }
    }
    anyhow::bail!("в последнем релизе {repo} не нашёл tar-архив")
}

/// Скачивание с ретраями (до 3 попыток) и возможностью отмены.
fn download(
    url: &str,
    dest: &Path,
    tx: &Sender<DlMsg>,
    label: &str,
    cancel: &AtomicBool,
) -> anyhow::Result<()> {
    let mut last_err = None;
    for attempt in 1..=3 {
        if cancel.load(Ordering::Relaxed) {
            anyhow::bail!("отменено");
        }
        match download_once(url, dest, tx, label, cancel) {
            Ok(()) => return Ok(()),
            Err(e) => {
                let _ = tx.send(DlMsg::Log(format!(
                    "Попытка {attempt} не удалась: {e}"
                )));
                last_err = Some(e);
                if cancel.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("не удалось скачать {label}")))
}

fn download_once(
    url: &str,
    dest: &Path,
    tx: &Sender<DlMsg>,
    label: &str,
    cancel: &AtomicBool,
) -> anyhow::Result<()> {
    let resp = ureq::get(url).set("User-Agent", "rustauncher").call()?;
    let total = resp
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok());
    let mut reader = resp.into_reader();
    let mut file = File::create(dest)?;
    let mut buf = vec![0u8; 128 * 1024];
    let mut got = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            anyhow::bail!("отменено");
        }
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        got += n as u64;
        let _ = tx.send(DlMsg::Progress {
            got,
            total,
            label: label.to_string(),
        });
    }
    Ok(())
}

pub fn human_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    let n = n as f64;
    if n < KB * KB {
        format!("{:.0} КБ", n / KB)
    } else if n < KB * KB * KB {
        format!("{:.1} МБ", n / (KB * KB))
    } else {
        format!("{:.2} ГБ", n / (KB * KB * KB))
    }
}

pub fn human_speed(bps: f64) -> String {
    if bps <= 0.0 {
        return "-".to_string();
    }
    const KB: f64 = 1024.0;
    if bps < KB * KB {
        format!("{:.0} КБ/с", bps / KB)
    } else {
        format!("{:.1} МБ/с", bps / (KB * KB))
    }
}

pub fn human_eta(secs: u64) -> String {
    if secs < 60 {
        format!("{secs} с")
    } else if secs < 3600 {
        format!("{} мин", secs / 60)
    } else {
        format!("{} ч {} мин", secs / 3600, (secs % 3600) / 60)
    }
}
