//! Tauri-команды для работы с изображениями.

use crate::babel;
use crate::image as bim;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
pub struct ImageMatch {
    /// Адрес найденной страницы.
    pub address: crate::AddressInfo,
    /// Квантованное изображение в виде PNG (data URL).
    pub quantized_png: String,
    /// Миниатюра исходника (data URL).
    pub source_png: String,
    /// Соответствие исходной картинки и найденной страницы, 0..=1.
    pub similarity: f32,
    /// Страница как текст (3200 символов).
    pub text: String,
    /// Короткое имя файла для сохранения.
    pub file_stem: String,
}

/// Ищет в библиотеке страницу, соответствующую изображению.
///
/// Алгоритм: картинка сжимается до сетки 80×40 (= 3200 символов = длина
/// страницы) и квантуется в 64-цветную палитру, где индекс цвета — это
/// номер символа алфавита. Полученный текст — это страница. Обратным
/// преобразованием находим её адрес. Картинка оказывается «зашита» в
/// библиотеку навсегда, ровно как текст.
#[tauri::command]
pub async fn search_image(path: String) -> Result<ImageMatch, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let img = bim::load(&path)?;
        let pixels = bim::quantize(&img);
        let text = bim::pixels_to_text(&pixels)?;
        // Страница 1 — адрес восстанавливается из содержимого.
        let addr = babel::address_for_content(&text, 1)?;
        let source_png = bim::thumbnail(&img, 320)?;
        let quantized_png = bim::pixels_to_png(&pixels, 6)?;
        // Сравниваем исходник с тем, что получилось после квантования.
        let similarity = similarity(&img, &pixels);
        Ok(ImageMatch {
            address: crate::to_info(&addr),
            quantized_png: data_url(&quantized_png),
            source_png: data_url(&source_png),
            similarity,
            text,
            file_stem: addr.file_stem(),
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Доля совпадения исходного изображения с квантованным (0..=1).
fn similarity(img: &image::DynamicImage, pixels: &[u8]) -> f32 {
    let small = image::imageops::resize(
        img,
        bim::COLS as u32,
        bim::ROWS as u32,
        image::imageops::FilterType::Triangle,
    );
    let pal = bim::palette();
    let mut total = 0u64;
    let mut matched = 0u64;
    for y in 0..bim::ROWS {
        for x in 0..bim::COLS {
            let px = small.get_pixel(x as u32, y as u32).0;
            let idx = pixels[y * bim::COLS + x] as usize % pal.len();
            let (r, g, b) = pal[idx];
            let dr = px[0] as i32 - r as i32;
            let dg = px[1] as i32 - g as i32;
            let db = px[2] as i32 - b as i32;
            let d = (dr * dr + dg * dg + db * db) as u64;
            total += d;
            if d < 4000 {
                matched += 1;
            }
        }
    }
    if total == 0 {
        return 1.0;
    }
    // Близкие цвета дают малую ошибку; нормируем через долю «хороших» ячеек.
    let raw = matched as f32 / (bim::CELLS as f32);
    let err = (total as f32 / bim::CELLS as f32).sqrt() / 255.0;
    (raw * (1.0 - err.min(0.9)) * 1.15).min(1.0)
}

/// Рендерит страницу как изображение (сетка 80×40 цветных ячеек).
#[tauri::command]
pub async fn render_page_as_image(
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
    page: u32,
    scale: u32,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let addr = babel::Address {
            hex,
            wall,
            shelf,
            volume,
            page,
        };
        let text = babel::page_content(&addr)?;
        let pixels = bim::text_to_pixels(&text)?;
        let png = bim::pixels_to_png(&pixels, scale.clamp(1, 20))?;
        Ok(data_url(&png))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Сохраняет страницу как PNG-изображение.
#[tauri::command]
pub async fn save_page_image(
    path: String,
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
    page: u32,
    scale: u32,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let addr = babel::Address {
            hex,
            wall,
            shelf,
            volume,
            page,
        };
        let text = babel::page_content(&addr)?;
        let pixels = bim::text_to_pixels(&text)?;
        let png = bim::pixels_to_png(&pixels, scale.clamp(1, 20))?;
        let path = PathBuf::from(path);
        fs::write(&path, png).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Сохраняет найденное изображение как PNG.
#[tauri::command]
pub async fn save_image_match(path: String, pixels: Vec<u8>, scale: u32) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let png = bim::pixels_to_png(&pixels, scale.clamp(1, 20))?;
        let path = PathBuf::from(path);
        fs::write(&path, png).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Сохраняет data URL (base64) в файл.
#[tauri::command]
pub async fn save_data_url(path: String, data_url: String) -> Result<String, String> {
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    tauri::async_runtime::spawn_blocking(move || {
        let b64 = data_url
            .split_once(',')
            .map(|(_, b)| b)
            .unwrap_or(&data_url)
            .trim();
        let bytes = B64
            .decode(b64)
            .map_err(|e| format!("не удалось декодировать изображение: {}", e))?;
        let path = PathBuf::from(path);
        fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// PNG-байты -> data URL для <img src>.
pub fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", B64.encode(png))
}
