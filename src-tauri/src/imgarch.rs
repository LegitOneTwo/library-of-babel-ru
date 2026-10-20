//! Архив изображений «Вавилонской Библиотеки».
//!
//! Реализация по образцу babelia.libraryofbabel.info: вместо букв
//! перебираются цвета. Вместо страницы из 3200 символов — пиксельная сетка
//! 640×416, то есть 266 240 пикселей; вместо 29 символов — 4096 цветов
//! (12 бит на пиксель: по 4 бита на R, G и B).
//!
//! Локация изображения — целое число из 3 194 880 бит (это примерно
//! 961 000 десятичных цифр, как у оригинала). Изображение получается
//! умножением локации на нечётную константу по модулю 2^K — так же, как
//! страница в текстовой библиотеке. Поэтому:
//!
//! * одна и та же локация всегда даёт одно и то же изображение;
//! * по изображению локация восстанавливается однозначно — картинку можно
//!   «вписать» в архив навсегда;
//! * хранить ничего не нужно.
//!
//! Упаковка: два пикселя — это ровно 3 байта (24 бита), поэтому поток
//! читается тройками байт без битовых сдвигов.

use num_bigint::BigUint;
use num_traits::{One, Zero};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// Ширина сетки (столбцов).
pub const COLS: usize = 640;
/// Высота сетки (строк).
pub const ROWS: usize = 416;
/// Число пикселей.
pub const PIXELS: usize = COLS * ROWS;
/// Бит на пиксель (4096 цветов).
pub const COLOR_BITS: usize = 12;
/// Число цветов.
pub const COLORS: usize = 4096;
/// Полное число бит одного изображения.
pub const TOTAL_BITS: usize = PIXELS * COLOR_BITS;
/// Максимальное число десятичных цифр в локации.
pub const LOCATION_DIGITS: usize = 961_755;

/// Ориентация изображения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// По соотношению сторон исходника.
    Auto,
    /// Альбомная: сетка 640×416.
    Landscape,
    /// Книжная: сетка 416×640.
    Portrait,
}

/// Способ вписывания в сетку.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fill {
    /// С сохранением пропорций, поля заполняются цветом рамки.
    Bars,
    /// Растянуть на всю сетку.
    Stretch,
}

/// Цвета рамки (12 бит: r4 g4 b4).
pub const BORDER_GREY: u16 = 0x222;
pub const BORDER_BLACK: u16 = 0x000;
pub const BORDER_WHITE: u16 = 0xfff;
pub const BORDER_RED: u16 = 0xf00;
pub const BORDER_GREEN: u16 = 0x0f0;
pub const BORDER_BLUE: u16 = 0x00f;
pub const BORDER_YELLOW: u16 = 0xff0;
pub const BORDER_ORANGE: u16 = 0xf80;
pub const BORDER_PURPLE: u16 = 0x80f;

/// Цвет по индексу палитры (0..4096) -> RGB.
#[inline]
pub fn color_of(index: u16) -> (u8, u8, u8) {
    let r = ((index >> 8) & 0xf) as u8;
    let g = ((index >> 4) & 0xf) as u8;
    let b = (index & 0xf) as u8;
    (r * 17, g * 17, b * 17)
}

/// RGB -> ближайший индекс палитры (по 4 бита на канал).
#[inline]
pub fn index_of(r: u8, g: u8, b: u8) -> u16 {
    let r4 = (r as u16 * 15 + 127) / 255;
    let g4 = (g as u16 * 15 + 127) / 255;
    let b4 = (b as u16 * 15 + 127) / 255;
    (r4 << 8) | (g4 << 4) | b4
}

/// Нечётный множитель размера TOTAL_BITS бит.
fn multiplier() -> BigUint {
    static M: OnceLock<BigUint> = OnceLock::new();
    M.get_or_init(|| {
        let mut bytes = vec![0u8; TOTAL_BITS / 8];
        // Детерминированный поток: SHA-256 с меткой и счётчиком.
        let mut counter: u64 = 0;
        let mut block: Vec<u8> = Vec::with_capacity(bytes.len() + 32);
        while block.len() < bytes.len() {
            let mut h = Sha256::new();
            h.update(b"babelia/S");
            h.update(counter.to_le_bytes());
            block.extend_from_slice(&h.finalize());
            counter += 1;
        }
        block.truncate(bytes.len());
        bytes.copy_from_slice(&block);
        let mut m = BigUint::from_bytes_be(&bytes);
        m |= BigUint::one(); // нечётность обязательна
        m
    })
    .clone()
}

/// Обратный множитель по модулю 2^TOTAL_BITS.
///
/// Метод Ньютона для обратного по модулю степени двойки:
/// `x ← x·(2 − a·x)`. Если x — обратный по модулю 2^m, то новый x — обратный
/// по модулю 2^(2m), то есть число верных бит удваивается каждый шаг.
/// Старт: для нечётного a обратный по модулю 2 равен 1.
fn inverse_multiplier() -> BigUint {
    static INV: OnceLock<BigUint> = OnceLock::new();
    INV.get_or_init(|| {
        let a = multiplier();
        let mut prec = 1usize;
        let mut x = BigUint::one(); // a^-1 mod 2 = 1 (a нечётно)
        while prec < TOTAL_BITS {
            let next = (prec * 2).min(TOTAL_BITS);
            let pow2 = BigUint::one() << next; // 2^next
            let mask_next = &pow2 - BigUint::one(); // 2^next - 1
            let ax = (&a * &x) & &mask_next; // a*x mod 2^next
                                             // factor = (2 - a*x) mod 2^next = (2^next + 2 - a*x) mod 2^next
            let factor = (&pow2 + BigUint::from(2u32) - &ax) & &mask_next;
            x = (&x * &factor) & &mask_next;
            prec = next;
        }
        x
    })
    .clone()
}

/// Побитовая маска размера TOTAL_BITS.
fn mask() -> BigUint {
    (BigUint::one() << TOTAL_BITS) - BigUint::one()
}

/// Локация -> пиксели (индексы палитры, 12 бит на пиксель).
pub fn pixels_from_location(location: &BigUint) -> Vec<u16> {
    let m = mask();
    let v = (location * multiplier()) & m;
    let bytes = v.to_bytes_be();
    let mut padded = vec![0u8; TOTAL_BITS / 8];
    let off = padded.len() - bytes.len();
    padded[off..].copy_from_slice(&bytes);

    // Два пикселя = 24 бита = ровно 3 байта.
    let mut out = Vec::with_capacity(PIXELS);
    for j in 0..PIXELS / 2 {
        let g = ((padded[j * 3] as u32) << 16)
            | ((padded[j * 3 + 1] as u32) << 8)
            | padded[j * 3 + 2] as u32;
        out.push(((g >> 12) & 0xfff) as u16);
        out.push((g & 0xfff) as u16);
    }
    out
}

/// Пиксели -> локация (обратное преобразование).
pub fn location_from_pixels(pixels: &[u16]) -> BigUint {
    let mut bytes = vec![0u8; TOTAL_BITS / 8];
    for j in 0..PIXELS / 2 {
        let p0 = (pixels[j * 2] & 0xfff) as u32;
        let p1 = (pixels[j * 2 + 1] & 0xfff) as u32;
        let g = (p0 << 12) | p1;
        bytes[j * 3] = (g >> 16) as u8;
        bytes[j * 3 + 1] = (g >> 8) as u8;
        bytes[j * 3 + 2] = g as u8;
    }
    let v = BigUint::from_bytes_be(&bytes);
    (&v * inverse_multiplier()) & mask()
}

/// Локация -> десятичная строка (как в оригинале: только цифры).
pub fn dec_from_location(location: &BigUint) -> String {
    location.to_str_radix(10)
}

/// Десятичная строка -> локация.
pub fn location_from_dec(dec: &str) -> Result<BigUint, String> {
    let cleaned: String = dec.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() {
        return Err("локация должна содержать цифры".into());
    }
    if cleaned.len() > LOCATION_DIGITS {
        return Err("локация слишком длинная".into());
    }
    let v = BigUint::parse_bytes(cleaned.as_bytes(), 10).ok_or("не удалось разобрать локацию")?;
    if v.bits() > TOTAL_BITS as u64 {
        return Err("локация слишком длинная".into());
    }
    Ok(v)
}

/// Краткое отображение локации (как у оригинала: начало и конец).
pub fn short_location(dec: &str) -> String {
    if dec.len() <= 24 {
        return dec.to_string();
    }
    format!("{}…{}", &dec[..12], &dec[dec.len() - 10..])
}

/// Рисует пиксели в PNG. `transpose` меняет местами строки и столбцы.
pub fn render_png(pixels: &[u16], transpose: bool) -> Result<Vec<u8>, String> {
    use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
    if pixels.len() != PIXELS {
        return Err(format!(
            "ожидалось {} пикселей, получено {}",
            PIXELS,
            pixels.len()
        ));
    }
    let (w, h) = if transpose {
        (ROWS, COLS)
    } else {
        (COLS, ROWS)
    };
    let mut img = RgbImage::new(w as u32, h as u32);
    for y in 0..ROWS {
        for x in 0..COLS {
            let idx = pixels[y * COLS + x];
            let (r, g, b) = color_of(idx);
            let (px, py) = if transpose { (y, x) } else { (x, y) };
            img.put_pixel(px as u32, py as u32, Rgb([r, g, b]));
        }
    }
    let mut buf = std::io::Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, ImageFormat::Png)
        .map_err(|e| format!("не удалось записать PNG: {}", e))?;
    Ok(buf.into_inner())
}

/// Квантует изображение в палитру и сетку 640×416.
pub fn quantize(
    img: &image::DynamicImage,
    orientation: Orientation,
    fill: Fill,
    border: u16,
) -> Vec<u16> {
    use image::imageops::FilterType;

    let transpose = match orientation {
        Orientation::Auto => img.width() < img.height(),
        Orientation::Landscape => false,
        Orientation::Portrait => true,
    };

    // При книжной ориентации исходник поворачивается, чтобы портретная
    // картинка заняла альбомную сетку (как в оригинале: flip=portrait).
    let source = if transpose {
        image::DynamicImage::ImageRgba8(image::imageops::rotate90(&img.to_rgba8()))
    } else {
        img.clone()
    };

    let (sw, sh) = (source.width().max(1), source.height().max(1));
    let (br, bg, bb) = color_of(border);

    match fill {
        Fill::Stretch => {
            let scaled =
                image::imageops::resize(&source, COLS as u32, ROWS as u32, FilterType::Triangle);
            image::DynamicImage::ImageRgba8(scaled)
                .to_rgb8()
                .pixels()
                .map(|p| index_of(p[0], p[1], p[2]))
                .collect()
        }
        Fill::Bars => {
            // Вписываем с сохранением пропорций, поля — цветом рамки.
            let scale = (COLS as f32 / sw as f32).min(ROWS as f32 / sh as f32);
            let nw = ((sw as f32 * scale).round() as u32).clamp(1, COLS as u32);
            let nh = ((sh as f32 * scale).round() as u32).clamp(1, ROWS as u32);
            let scaled = image::imageops::resize(&source, nw, nh, FilterType::Triangle);
            let ox = (COLS as u32 - nw) / 2;
            let oy = (ROWS as u32 - nh) / 2;
            let mut out = Vec::with_capacity(PIXELS);
            for y in 0..ROWS as u32 {
                for x in 0..COLS as u32 {
                    let px = if x >= ox && x < ox + nw && y >= oy && y < oy + nh {
                        let p = scaled.get_pixel(x - ox, y - oy).0;
                        [p[0], p[1], p[2]]
                    } else {
                        [br, bg, bb]
                    };
                    out.push(index_of(px[0], px[1], px[2]));
                }
            }
            out
        }
    }
}

/// Случайная локация.
pub fn random_location() -> BigUint {
    let mut buf = vec![0u8; TOTAL_BITS / 8];
    {
        use rand::RngCore;
        rand::thread_rng().fill_bytes(&mut buf);
    }
    BigUint::from_bytes_be(&buf)
}

/// Предыдущая локация.
pub fn prev_location(location: &BigUint) -> BigUint {
    let m = mask();
    if location.is_zero() {
        m
    } else {
        location - BigUint::one()
    }
}

/// Смещение локации вперёд на n.
pub fn add(location: &BigUint, n: u64) -> BigUint {
    let m = mask();
    (location + BigUint::from(n)) & m
}

/// Доля совпадения исходника с квантованной версией (0..=1).
pub fn similarity(original: &image::DynamicImage, pixels: &[u16]) -> f32 {
    use image::imageops::FilterType;
    let small = image::imageops::resize(original, COLS as u32, ROWS as u32, FilterType::Triangle);
    let mut matched = 0u64;
    for (i, p) in small.pixels().enumerate() {
        let q = pixels.get(i).copied().unwrap_or(0);
        let (r, g, b) = color_of(q);
        let dr = p[0] as i32 - r as i32;
        let dg = p[1] as i32 - g as i32;
        let db = p[2] as i32 - b as i32;
        if dr * dr + dg * dg + db * db < 4000 {
            matched += 1;
        }
    }
    matched as f32 / PIXELS as f32
}

/// Число возможных изображений (2^TOTAL_BITS).
pub fn total_images() -> String {
    // log10(2) = 0.3010299956639812..., округление вниз даёт число цифр.
    let digits = (TOTAL_BITS as f64 * std::f64::consts::LOG10_2).floor() as u64;
    format!("~10^{}", digits)
}

/// Сквозная проверка: вписать картинку, найти локацию, восстановить и
/// сравнить с исходником. Возвращает текстовый отчёт.
pub fn probe(path: &str) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("не удалось открыть {}: {}", path, e))?;
    let original_size = (img.width(), img.height());

    // 1. Вписываем с сохранением пропорций.
    let pixels = quantize(&img, Orientation::Auto, Fill::Bars, BORDER_GREY);
    // 2. Локация.
    let loc = location_from_pixels(&pixels);
    let dec = dec_from_location(&loc);
    // 3. Восстанавливаем изображение из локации.
    let restored = pixels_from_location(&loc);
    // 4. Проверяем, что всё совпало.
    let identical = restored == pixels;
    let sim = similarity(&img, &pixels);

    // 5. Проверяем, что локация переживает «перезагрузку» через строку.
    let reparsed = location_from_dec(&dec).map_err(|e| e.to_string())?;
    let stable = reparsed == loc;

    Ok(format!(
        "исходник:       {}x{}\n\
         локация:        {} цифр, начало {}\n\
         восстановлено:  {}\n\
         локация через строку: {}\n\
         совпадение:     {:.1}%\n\
         PNG:            {} байт",
        original_size.0,
        original_size.1,
        dec.len(),
        &dec[..12.min(dec.len())],
        if identical {
            "да, попиксельно"
        } else {
            "НЕТ"
        },
        if stable {
            "стабильна"
        } else {
            "НЕСТАБИЛЬНА"
        },
        sim * 100.0,
        render_png(&restored, false)?.len(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_constants() {
        assert_eq!(PIXELS, 266_240);
        assert_eq!(TOTAL_BITS, 3_194_880);
        assert_eq!(PIXELS % 2, 0);
    }

    #[test]
    fn palette_roundtrip() {
        for i in 0..COLORS as u16 {
            let (r, g, b) = color_of(i);
            assert_eq!(index_of(r, g, b), i);
        }
    }

    #[test]
    fn multiplier_is_odd() {
        assert!(multiplier().bit(0));
    }

    #[test]
    fn inverse_is_correct() {
        let a = multiplier();
        let inv = inverse_multiplier();
        let m = mask();
        assert_eq!((&a * &inv) & &m, BigUint::one());
    }

    #[test]
    fn location_roundtrip_is_stable() {
        let loc = random_location();
        let pixels = pixels_from_location(&loc);
        assert_eq!(pixels.len(), PIXELS);
        let back = location_from_pixels(&pixels);
        assert_eq!(back, loc);
    }

    #[test]
    fn decimal_roundtrip() {
        let loc = random_location();
        let dec = dec_from_location(&loc);
        assert!(dec.chars().all(|c| c.is_ascii_digit()));
        assert_eq!(location_from_dec(&dec).unwrap(), loc);
    }

    #[test]
    fn decimal_small_values() {
        assert_eq!(location_from_dec("1").unwrap(), BigUint::one());
        assert_eq!(location_from_dec("0").unwrap(), BigUint::zero());
        assert_eq!(dec_from_location(&BigUint::from(42u32)), "42");
    }

    #[test]
    fn decimal_rejects_garbage() {
        assert!(location_from_dec("").is_err());
        assert!(location_from_dec("abc").is_err());
    }

    #[test]
    fn png_renders() {
        let loc = random_location();
        let pixels = pixels_from_location(&loc);
        let png = render_png(&pixels, false).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        assert!(png.len() > 1000);
    }
}
