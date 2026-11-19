use std::sync::Arc;

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// Встроенные растровые ассеты (декодируются один раз при старте).
pub struct Assets {
    pub logo: TextureHandle,
    pub empty_apps: TextureHandle,
    pub empty_prefixes: TextureHandle,
    pub empty_search: TextureHandle,
}

impl Assets {
    pub fn load(ctx: &Context) -> Self {
        Self {
            logo: texture(ctx, include_bytes!("../assets/icon-128.png"), "logo"),
            empty_apps: texture(
                ctx,
                include_bytes!("../assets/empty-apps.png"),
                "empty-apps",
            ),
            empty_prefixes: texture(
                ctx,
                include_bytes!("../assets/empty-prefixes.png"),
                "empty-prefixes",
            ),
            empty_search: texture(
                ctx,
                include_bytes!("../assets/empty-search.png"),
                "empty-search",
            ),
        }
    }
}

/// Иконка окна/панели задач.
pub fn window_icon() -> Option<Arc<egui::IconData>> {
    let img = image::load_from_memory(include_bytes!("../assets/icon-256.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = (img.width(), img.height());
    Some(Arc::new(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }))
}

fn texture(ctx: &Context, bytes: &[u8], name: &str) -> TextureHandle {
    let img = image::load_from_memory(bytes)
        .expect("встроенный PNG-ассет должен читаться")
        .into_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let pixels = ColorImage::from_rgba_unmultiplied(size, &img.into_raw());
    ctx.load_texture(name, pixels, TextureOptions::LINEAR)
}
