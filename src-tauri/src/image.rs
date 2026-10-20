//! Изображения: страница как растровая сетка.
//!
//! Страница — это ровно 3200 символов = сетка 80 колонок × 40 строк.
//! Совпадение не случайно: любое изображение, сжатое до 80×40, помещается
//! в страницу без потерь. Каждая ячейка сетки — один символ алфавита, а
//! символ отображается в фиксированный цвет 64-цветной палитры.
//!
//! Отсюда две возможности:
//!  - «страница как изображение» — читаем страницу и рисуем её;
//!  - «поиск по изображению» — квантуем картинку в палитру, получаем текст
//!    страницы, обратным преобразованием находим её адрес. Картинка живёт
//!    в библиотеке по тому же закону, что и текст: один раз и навсегда.

use crate::babel;
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;

/// Ширина сетки (совпадает с длиной строки).
pub const COLS: usize = babel::CHARS_PER_LINE;
/// Высота сетки (совпадает с числом строк на странице).
pub const ROWS: usize = babel::LINES_PER_PAGE;
/// Число ячеек — ровно длина страницы.
pub const CELLS: usize = COLS * ROWS;

/// 64-цветная палитра: 8 оттенков × 8 уровней светлоты.
pub fn palette() -> Vec<(u8, u8, u8)> {
    let mut out = Vec::with_capacity(64);
    for h in 0..8u32 {
        for l in 0..8u32 {
            let hue = h as f32 * 45.0;
            let light = 0.18 + l as f32 * 0.098;
            let sat = 0.62 - l as f32 * 0.02;
            out.push(hsl_to_rgb(hue, sat, light));
        }
    }
    out
}

/// HSL -> RGB.
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h % 360.0) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        (((r1 + m) * 255.0).round() as i32).clamp(0, 255) as u8,
        (((g1 + m) * 255.0).round() as i32).clamp(0, 255) as u8,
        (((b1 + m) * 255.0).round() as i32).clamp(0, 255) as u8,
    )
}

/// Ближайший цвет палитры к данному RGB.
fn nearest_palette(pal: &[(u8, u8, u8)], r: u8, g: u8, b: u8) -> u8 {
    let mut best = 0usize;
    let mut best_d = u32::MAX;
    for (i, (pr, pg, pb)) in pal.iter().enumerate() {
        let dr = r as i32 - *pr as i32;
        let dg = g as i32 - *pg as i32;
        let db = b as i32 - *pb as i32;
        // Взвешенная евклидова метрика ( perceptual-ish ).
        let d = (2 * dr * dr + 4 * dg * dg + 3 * db * db) as u32;
        if d < best_d {
            best_d = d;
            best = i;
        }
    }
    best as u8
}

/// Сжимает изображение до сетки 80×40 и квантует в палитру.
/// Возвращает 3200 индексов палитры (0..63).
pub fn quantize(img: &DynamicImage) -> Vec<u8> {
    let small = image::imageops::resize(
        img,
        COLS as u32,
        ROWS as u32,
        image::imageops::FilterType::Triangle,
    );
    let pal = palette();
    let mut out = Vec::with_capacity(CELLS);
    for y in 0..ROWS {
        for x in 0..COLS {
            let px = small.get_pixel(x as u32, y as u32).0;
            out.push(nearest_palette(&pal, px[0], px[1], px[2]));
        }
    }
    out
}

/// Индексы палитры -> текст страницы (3200 символов).
pub fn pixels_to_text(pixels: &[u8]) -> Result<String, String> {
    if pixels.len() != CELLS {
        return Err(format!(
            "ожидалось {} ячеек, получено {}",
            CELLS,
            pixels.len()
        ));
    }
    let alpha = babel::alphabet_chars();
    let mut out = String::with_capacity(CELLS);
    for p in pixels {
        let idx = (*p as usize) % alpha.len();
        out.push(alpha[idx]);
    }
    Ok(out)
}

/// Текст страницы -> индексы палитры.
pub fn text_to_pixels(text: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::with_capacity(CELLS);
    for ch in text.chars() {
        if out.len() >= CELLS {
            break;
        }
        let v = babel::symbol_value(ch)
            .ok_or_else(|| format!("символ {:?} отсутствует в алфавите", ch))?;
        out.push(v);
    }
    if out.len() != CELLS {
        return Err(format!(
            "текст должен содержать {} символов, получено {}",
            CELLS,
            out.len()
        ));
    }
    Ok(out)
}

/// Рисует сетку индексов палитры в PNG. `scale` — увеличение «пикселя».
pub fn pixels_to_png(pixels: &[u8], scale: u32) -> Result<Vec<u8>, String> {
    if pixels.len() != CELLS {
        return Err(format!(
            "ожидалось {} ячеек, получено {}",
            CELLS,
            pixels.len()
        ));
    }
    let pal = palette();
    let scale = scale.clamp(1, 32);
    let mut img = RgbImage::new((COLS as u32) * scale, (ROWS as u32) * scale);
    for y in 0..ROWS {
        for x in 0..COLS {
            let idx = pixels[y * COLS + x] as usize % pal.len();
            let color = pal[idx];
            let rgb = image::Rgb([color.0, color.1, color.2]);
            for dy in 0..scale {
                for dx in 0..scale {
                    img.put_pixel((x as u32) * scale + dx, (y as u32) * scale + dy, rgb);
                }
            }
        }
    }
    let mut buf = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| format!("не удалось записать PNG: {}", e))?;
    Ok(buf.into_inner())
}

/// Цветная миниатюра исходного изображения (для сравнения с квантованным).
pub fn thumbnail(img: &DynamicImage, width: u32) -> Result<Vec<u8>, String> {
    let height = (img.height() as f32 * width as f32 / img.width() as f32).round() as u32;
    let height = height.clamp(1, 4096);
    let small = image::imageops::resize(img, width, height, image::imageops::FilterType::Triangle);
    let mut buf = Cursor::new(Vec::new());
    small
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(buf.into_inner())
}

/// Читает файл изображения с диска.
pub fn load(path: &str) -> Result<DynamicImage, String> {
    image::open(path).map_err(|e| format!("не удалось открыть изображение: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_has_64_colors() {
        assert_eq!(palette().len(), 64);
    }

    #[test]
    fn pixels_roundtrip_through_text() {
        let mut pixels = Vec::with_capacity(CELLS);
        for i in 0..CELLS {
            pixels.push((i % 64) as u8);
        }
        let text = pixels_to_text(&pixels).unwrap();
        assert_eq!(text.chars().count(), CELLS);
        assert_eq!(text_to_pixels(&text).unwrap(), pixels);
    }

    #[test]
    fn quantize_produces_exact_grid() {
        let img = DynamicImage::ImageRgb8(RgbImage::from_pixel(20, 10, image::Rgb([120, 40, 200])));
        let pixels = quantize(&img);
        assert_eq!(pixels.len(), CELLS);
    }

    #[test]
    fn png_renders() {
        let pixels = vec![7u8; CELLS];
        let png = pixels_to_png(&pixels, 4).unwrap();
        assert!(png.len() > 100);
        assert_eq!(&png[1..4], b"PNG");
    }
}
