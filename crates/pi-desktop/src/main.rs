mod app;
mod backend;
mod components;
mod design;
mod ui;

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use anyhow::Result;
use gpui::{
    App, AppContext as _, Application, AssetSource, Bounds, SharedString, WindowBounds,
    WindowOptions, px, size,
};

struct FileAssets;

impl AssetSource for FileAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        std::fs::read(path)
            .map(|bytes| Some(Cow::Owned(bytes)))
            .map_err(Into::into)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(std::fs::read_dir(Path::new(path))?
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|entry| SharedString::from(entry.path().display().to_string()))
            })
            .collect())
    }
}

fn main() {
    Application::new()
        .with_assets(FileAssets)
        .run(|cx: &mut App| {
            if let Err(error) = load_custom_fonts(cx) {
                eprintln!("failed to load custom fonts: {error:#}");
            }
            gpui_component::init(cx);
            gpui_component::Theme::change(gpui_component::ThemeMode::Dark, None, cx);
            let bounds = Bounds::centered(None, size(px(1180.0), px(780.0)), cx);
            if let Err(error) = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| app::PiDesktop::new(window, cx));
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                },
            ) {
                eprintln!("failed to open pi desktop window: {error}");
            }
        });
}

fn load_custom_fonts(cx: &mut App) -> Result<()> {
    let root = workspace_root();
    let fonts = [
        "PlantinNowVariable-Upright.woff2",
        "PlantinNowVariable-Italic.woff2",
    ]
    .into_iter()
    .map(|file| std::fs::read(root.join("assets/fonts").join(file)).map(Cow::Owned))
    .collect::<std::io::Result<Vec<_>>>()?;
    cx.text_system().add_fonts(fonts)
}

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or(manifest)
}
