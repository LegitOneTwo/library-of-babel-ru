//! Tauri-команды архива изображений (по образцу babelia).

use crate::imgarch as ia;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Ориентация изображения (приходит строкой из интерфейса).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OrientationArg {
    Auto,
    Landscape,
    Portrait,
}

/// Способ вписывания в сетку (приходит строкой из интерфейса).
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum FillArg {
    Bars,
    Stretch,
}

impl From<OrientationArg> for ia::Orientation {
    fn from(v: OrientationArg) -> Self {
        match v {
            OrientationArg::Auto => ia::Orientation::Auto,
            OrientationArg::Landscape => ia::Orientation::Landscape,
            OrientationArg::Portrait => ia::Orientation::Portrait,
        }
    }
}

impl From<FillArg> for ia::Fill {
    fn from(v: FillArg) -> Self {
        match v {
            FillArg::Bars => ia::Fill::Bars,
            FillArg::Stretch => ia::Fill::Stretch,
        }
    }
}

/// Результат показа изображения в архиве.
#[derive(Serialize)]
pub struct ArchiveView {
    /// Полная локация (десятичная строка, ~961 755 цифр).
    pub location: String,
    /// Сокращённая локация для показа.
    pub short: String,
    /// Изображение в виде PNG (data URL).
    pub png: String,
    /// Размер сетки: 640×416.
    pub width: usize,
    pub height: usize,
    /// Повёрнуто ли изображение (книжная ориентация).
    pub transposed: bool,
}

/// Информация об архиве.
#[derive(Serialize)]
pub struct ArchiveInfo {
    pub width: usize,
    pub height: usize,
    pub pixels: usize,
    pub colors: usize,
    pub location_digits: usize,
    pub total_images: String,
    pub distinct_pixels: String,
    /// Цвета полей для вписывания картинки: (подпись, индекс палитры).
    pub border_colors: Vec<(String, u16)>,
}

/// Случайное изображение архива.
#[tauri::command]
pub async fn archive_random() -> Result<ArchiveView, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let loc = ia::random_location();
        build_view(&loc, false)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Изображение по локации (десятичная строка).
#[tauri::command]
pub async fn archive_by_location(location: String) -> Result<ArchiveView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loc = ia::location_from_dec(&location)?;
        build_view(&loc, false)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Следующее / предыдущее изображение (для слайдшоу).
#[tauri::command]
pub async fn archive_step(
    location: String,
    forward: bool,
    step: u64,
) -> Result<ArchiveView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loc = ia::location_from_dec(&location)?;
        let next = if forward {
            ia::add(&loc, step)
        } else {
            ia::prev_location(&loc)
        };
        build_view(&next, false)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Поиск изображения в архиве: картинка квантуется в сетку 640×416,
/// затем обратным преобразованием находится её локация.
#[tauri::command]
pub async fn archive_search_image(
    path: String,
    orientation: OrientationArg,
    fill: FillArg,
    border: u16,
) -> Result<ArchiveView, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let img =
            image::open(&path).map_err(|e| format!("не удалось открыть изображение: {}", e))?;
        let pixels = ia::quantize(&img, orientation.into(), fill.into(), border);
        let loc = ia::location_from_pixels(&pixels);
        let transposed = matches!(orientation, OrientationArg::Portrait)
            || (matches!(orientation, OrientationArg::Auto) && img.width() < img.height());
        let mut view = build_view(&loc, transposed)?;
        // Показываем квантованную версию, а не случайную картинку.
        let png = ia::render_png(&pixels, transposed)?;
        view.png = data_url(&png);
        Ok(view)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Сохраняет изображение архива в PNG.
#[tauri::command]
pub async fn archive_save_png(path: String, location: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loc = ia::location_from_dec(&location)?;
        let pixels = ia::pixels_from_location(&loc);
        let png = ia::render_png(&pixels, false)?;
        let path = PathBuf::from(path);
        fs::write(&path, png).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Сведения об архиве.
#[tauri::command]
pub fn archive_info() -> ArchiveInfo {
    ArchiveInfo {
        width: ia::COLS,
        height: ia::ROWS,
        pixels: ia::PIXELS,
        colors: ia::COLORS,
        location_digits: ia::LOCATION_DIGITS,
        total_images: ia::total_images(),
        distinct_pixels: "266 240 (каждый пиксель это 12 бит)".to_string(),
        // Список живёт здесь, а не в интерфейсе: иначе индексы палитры
        // пришлось бы дублировать на двух сторонах.
        border_colors: vec![
            ("Серый".to_string(), ia::BORDER_GREY),
            ("Чёрный".to_string(), ia::BORDER_BLACK),
            ("Белый".to_string(), ia::BORDER_WHITE),
            ("Красный".to_string(), ia::BORDER_RED),
            ("Зелёный".to_string(), ia::BORDER_GREEN),
            ("Синий".to_string(), ia::BORDER_BLUE),
            ("Жёлтый".to_string(), ia::BORDER_YELLOW),
            ("Оранжевый".to_string(), ia::BORDER_ORANGE),
            ("Фиолетовый".to_string(), ia::BORDER_PURPLE),
        ],
    }
}

fn build_view(loc: &num_bigint::BigUint, transposed: bool) -> Result<ArchiveView, String> {
    let dec = ia::dec_from_location(loc);
    let pixels = ia::pixels_from_location(loc);
    let png = ia::render_png(&pixels, transposed)?;
    Ok(ArchiveView {
        short: ia::short_location(&dec),
        location: dec,
        png: data_url(&png),
        width: ia::COLS,
        height: ia::ROWS,
        transposed,
    })
}

/// PNG-байты -> data URL.
pub fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", B64.encode(png))
}
