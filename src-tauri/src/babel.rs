//! Ядро «Вавилонской Библиотеки» — детерминированная обратимая генерация страниц.
//!
//! Алфавит: 64 символа (26 латинских + 33 кириллических + 5 знаков).
//! Страница: 3200 символов = 2400 байт = 19200 бит.
//! Адрес: имя гекса (base-36) + стена (1..=4) + полка (1..=5) + том (1..=32) + страница (1..=410).
//!
//! Ключевая идея: страница — это 19200-битное число V.
//! Адрес задаёт книгу B = H * 640 + offset, где H — имя гекса как число в base-36.
//! Содержимое страницы p: V = (B * S_p) mod 2^19200, где S_p — нечётная константа
//! (S_p = S0 + p*O, O чётна). Умножение на нечётное число mod 2^k — биекция,
//! поэтому содержимое однозначно восстанавливает адрес, и наоборот.
//! Хранить ничего не нужно: вся библиотека вычисляется на лету.

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::OnceLock;

/// Алфавит библиотеки: 64 символа (base-64).
pub const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyzабвгдежзийклмнопрстуфхцчшщъыьэюяё ,.!-";

/// Символов на странице.
pub const PAGE_SYMBOLS: usize = 3200;
/// Строк на странице.
pub const LINES_PER_PAGE: usize = 40;
/// Символов в строке.
pub const CHARS_PER_LINE: usize = 80;
/// Страниц в томе.
pub const PAGES_PER_BOOK: usize = 410;
/// Стен в гексе.
pub const WALLS: u32 = 4;
/// Полок на стене.
pub const SHELVES: u32 = 5;
/// Томов на полке.
pub const VOLUMES: u32 = 32;
/// Байт на страницу (3200 * 6 бит / 8).
pub const PAGE_BYTES: usize = 2400;
/// Бит на страницу.
pub const PAGE_BITS: usize = PAGE_BYTES * 8;
/// Книг в гексе (4 стены * 5 полок * 32 тома).
pub const BOOKS_PER_HEX: u32 = WALLS * SHELVES * VOLUMES;

/// Символы алфавита как массив — для индексации по номеру символа.
/// `ALPHABET.as_bytes()` не годится: кириллица в UTF-8 занимает по 2 байта.
pub fn alphabet_chars() -> &'static [char; 64] {
    static TABLE: OnceLock<[char; 64]> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut arr = [' '; 64];
        for (i, ch) in ALPHABET.chars().enumerate() {
            arr[i] = ch;
        }
        arr
    })
}

fn symbol_table() -> &'static HashMap<char, u8> {
    static TABLE: OnceLock<HashMap<char, u8>> = OnceLock::new();
    TABLE.get_or_init(|| {
        let mut m = HashMap::with_capacity(64);
        for (i, ch) in ALPHABET.chars().enumerate() {
            m.insert(ch, i as u8);
        }
        m
    })
}

/// Значение символа в алфавите, если он есть.
pub fn symbol_value(ch: char) -> Option<u8> {
    symbol_table().get(&ch).copied()
}

/// Приводит произвольный текст к алфавиту: нижний регистр, «?» -> «.»,
/// неподдерживаемые символы -> пробел.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        for c in ch.to_lowercase() {
            match c {
                '?' => out.push('.'),
                _ => {
                    if symbol_value(c).is_some() {
                        out.push(c);
                    } else {
                        out.push(' ');
                    }
                }
            }
        }
    }
    out
}

/// Превращает поток байт в поток символов алфавита (6 бит на символ).
fn symbols_from_bytes(bytes: &[u8]) -> Vec<char> {
    let alpha = alphabet_chars();
    let mut out = Vec::with_capacity(PAGE_SYMBOLS);
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &b in bytes {
        acc = (acc << 8) | b as u32;
        nbits += 8;
        while nbits >= 6 {
            nbits -= 6;
            out.push(alpha[((acc >> nbits) & 0x3f) as usize]);
        }
        acc &= (1u32 << nbits) - 1;
    }
    out
}

/// Кодирует 2400 байт в 3200 символов (6 бит на символ, MSB-first).
pub fn bytes_to_text(bytes: &[u8; PAGE_BYTES]) -> String {
    let alpha = alphabet_chars();
    let mut out = String::with_capacity(PAGE_SYMBOLS);
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    for &b in bytes.iter() {
        acc = (acc << 8) | b as u32;
        nbits += 8;
        while nbits >= 6 {
            nbits -= 6;
            let idx = ((acc >> nbits) & 0x3f) as usize;
            out.push(alpha[idx]);
        }
        acc &= (1u32 << nbits) - 1;
    }
    out
}

/// Декодирует текст в 2400 байт. Короткий текст дополняется нулями.
pub fn text_to_bytes(text: &str) -> Result<[u8; PAGE_BYTES], String> {
    let mut bytes = [0u8; PAGE_BYTES];
    let mut acc: u32 = 0;
    let mut nbits: u32 = 0;
    let mut written: usize = 0;
    for ch in text.chars() {
        if written >= PAGE_BYTES {
            break;
        }
        let v = match symbol_value(ch) {
            Some(v) => v,
            None => return Err(format!("символ {:?} отсутствует в алфавите", ch)),
        };
        acc = (acc << 6) | v as u32;
        nbits += 6;
        while nbits >= 8 && written < PAGE_BYTES {
            nbits -= 8;
            bytes[written] = ((acc >> nbits) & 0xff) as u8;
            written += 1;
        }
        acc &= (1u32 << nbits) - 1;
    }
    Ok(bytes)
}

fn modulus() -> BigUint {
    BigUint::one() << PAGE_BITS
}

fn mask() -> BigUint {
    modulus() - BigUint::one()
}

/// Детерминированный поток байт из SHA-256 (метка + счётчик).
fn derive_bytes(label: &str, n: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(n + 32);
    let mut counter: u64 = 0;
    while out.len() < n {
        let mut h = Sha256::new();
        h.update(label.as_bytes());
        h.update(counter.to_le_bytes());
        out.extend_from_slice(&h.finalize());
        counter += 1;
    }
    out.truncate(n);
    out
}

/// Нечётная константа S0 и чётная O: S_p = S0 + p*O всегда нечётно.
fn constants() -> &'static (BigUint, BigUint) {
    static C: OnceLock<(BigUint, BigUint)> = OnceLock::new();
    C.get_or_init(|| {
        let mut s0 = BigUint::from_bytes_be(&derive_bytes("babel/S0", PAGE_BYTES));
        s0 |= BigUint::one();
        let mut o = BigUint::from_bytes_be(&derive_bytes("babel/O", PAGE_BYTES));
        o -= &o & BigUint::one(); // обнуляем младший бит -> O чётно
        (s0, o)
    })
}

/// Множитель страницы p (нечётный, обратимый mod 2^PAGE_BITS).
fn page_multiplier(page: u32) -> BigUint {
    let (s0, o) = constants();
    let m = mask();
    let v = s0 + o * BigUint::from(page);
    v & m
}

/// Обратный элемент нечётного числа по модулю 2^PAGE_BITS (метод Ньютона).
/// x <- x * (2 - a*x) mod 2^k; каждая итерация удваивает число верных бит.
fn inverse_pow2(a: &BigUint) -> BigUint {
    let m = mask();
    // 2^k + 2 - ax  ==  (2 - ax) mod 2^k, так как m = 2^k - 1.
    let two_plus = BigUint::from(3u32);
    let mut x = a.clone(); // для нечётного a: a^2 == 1 (mod 8)
    for _ in 0..16 {
        let ax = (a * &x) & &m;
        let factor = (&m + &two_plus) - &ax;
        x = (&x * &factor) & &m;
    }
    x
}

/// Обратный множитель страницы.
fn page_multiplier_inverse(page: u32) -> BigUint {
    inverse_pow2(&page_multiplier(page))
}

/// Адрес книги/страницы.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Address {
    /// Имя гекса в base-36 (может быть очень длинным).
    pub hex: String,
    /// Сена 1..=4.
    pub wall: u32,
    /// Полка 1..=5.
    pub shelf: u32,
    /// Том 1..=32.
    pub volume: u32,
    /// Страница 1..=410.
    pub page: u32,
}

impl Address {
    /// Короткая запись адреса: `hex-w2-s4-v16`.
    pub fn canonical(&self) -> String {
        format!(
            "{}-w{}-s{}-v{}",
            self.hex, self.wall, self.shelf, self.volume
        )
    }

    /// Полная запись адреса: `hex-w2-s4-v16:12`.
    pub fn full(&self) -> String {
        format!("{}:{}", self.canonical(), self.page)
    }

    /// Короткий хэш адреса (для закладок).
    pub fn hash(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.full().as_bytes());
        let digest = h.finalize();
        digest[..8].iter().map(|b| format!("{:02x}", b)).collect()
    }

    /// Короткое, безопасное имя файла: без гигантского имени гекса.
    /// Имя гекса может достигать ~12 000 символов (base-36 от 19200-битного
    /// числа), поэтому в имя файла идёт только короткий хэш адреса.
    pub fn file_stem(&self) -> String {
        let mut h = Sha256::new();
        h.update(self.full().as_bytes());
        let digest = h.finalize();
        let short: String = digest[..4].iter().map(|b| format!("{:02x}", b)).collect();
        // Плюс короткий префикс гекса, чтобы имя было хоть немного читаемым.
        let head: String = self
            .hex
            .chars()
            .take(8)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string();
        let head = if head.is_empty() {
            "0".to_string()
        } else {
            head
        };
        format!(
            "{}-w{}-s{}-v{}-{}",
            head, self.wall, self.shelf, self.volume, short
        )
    }
}

fn hex_value(hex: &str) -> Result<BigUint, String> {
    let cleaned: String = hex
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if cleaned.is_empty() {
        return Ok(BigUint::from(0u32));
    }
    if !cleaned
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err("имя гекса: допустимы только цифры 0-9 и буквы a-z".into());
    }
    BigUint::parse_bytes(cleaned.as_bytes(), 36)
        .ok_or_else(|| "имя гекса: не удалось разобрать как base-36".to_string())
}

fn offset_of(wall: u32, shelf: u32, volume: u32) -> u32 {
    ((wall - 1) * SHELVES + (shelf - 1)) * VOLUMES + (volume - 1)
}

fn decode_offset(offset: u32) -> (u32, u32, u32) {
    let volume = offset % VOLUMES + 1;
    let shelf = (offset / VOLUMES) % SHELVES + 1;
    let wall = offset / (VOLUMES * SHELVES) + 1;
    (wall, shelf, volume)
}

/// Номер книги: H * 640 + смещение координат.
fn book_number(hex_value: &BigUint, wall: u32, shelf: u32, volume: u32) -> BigUint {
    hex_value * BigUint::from(BOOKS_PER_HEX) + BigUint::from(offset_of(wall, shelf, volume))
}

/// Восстанавливает адрес из номера книги и номера страницы.
fn address_from_book(book: &BigUint, page: u32) -> Address {
    let hex_big = book / BigUint::from(BOOKS_PER_HEX);
    let offset = (book % BigUint::from(BOOKS_PER_HEX)).to_u32().unwrap_or(0);
    let (wall, shelf, volume) = decode_offset(offset);
    Address {
        hex: hex_big.to_str_radix(36),
        wall,
        shelf,
        volume,
        page,
    }
}

/// Содержимое страницы по адресу.
pub fn page_content(addr: &Address) -> Result<String, String> {
    if !(1..=WALLS).contains(&addr.wall) {
        return Err(format!("стена должна быть 1..={}", WALLS));
    }
    if !(1..=SHELVES).contains(&addr.shelf) {
        return Err(format!("полка должна быть 1..={}", SHELVES));
    }
    if !(1..=VOLUMES).contains(&addr.volume) {
        return Err(format!("том должен быть 1..={}", VOLUMES));
    }
    if !(1..=PAGES_PER_BOOK as u32).contains(&addr.page) {
        return Err(format!("страница должна быть 1..={}", PAGES_PER_BOOK));
    }
    let hv = hex_value(&addr.hex)?;
    let b = book_number(&hv, addr.wall, addr.shelf, addr.volume);
    let m = mask();
    if b > m {
        return Err("имя гекса слишком длинное".into());
    }
    let v = (b * page_multiplier(addr.page)) & m;
    let raw = v.to_bytes_be();
    let mut bytes = [0u8; PAGE_BYTES];
    if raw.len() > PAGE_BYTES {
        return Err("внутренняя ошибка: страница не помещается".into());
    }
    bytes[PAGE_BYTES - raw.len()..].copy_from_slice(&raw);
    Ok(bytes_to_text(&bytes))
}

/// Адрес, восстанавливаемый из содержимого страницы (обратная операция).
pub fn address_for_content(text: &str, page: u32) -> Result<Address, String> {
    if !(1..=PAGES_PER_BOOK as u32).contains(&page) {
        return Err(format!("страница должна быть 1..={}", PAGES_PER_BOOK));
    }
    let bytes = text_to_bytes(text)?;
    let v = BigUint::from_bytes_be(&bytes);
    let m = mask();
    let b = (v * page_multiplier_inverse(page)) & m;
    Ok(address_from_book(&b, page))
}

/// Ищет страницу, содержащую заданный текст. Возвращает адрес и саму страницу.
pub fn find_text(text: &str) -> Result<(Address, String), String> {
    let needle = normalize(text);
    let needle = needle.trim().to_string();
    if needle.is_empty() {
        return Err("пустой поисковый запрос".into());
    }
    if needle.chars().count() > PAGE_SYMBOLS {
        return Err("запрос длиннее страницы".into());
    }
    let seed = format!("babel/search/{}", needle);
    let stream = derive_bytes(&seed, PAGE_BYTES);
    let mut symbols = symbols_from_bytes(&stream);
    symbols.truncate(PAGE_SYMBOLS);
    // Ставим запрос в начало строки, выбранной по хэшу.
    let mut digest = Sha256::new();
    digest.update(seed.as_bytes());
    let h = digest.finalize();
    let line = (h[0] as usize) % LINES_PER_PAGE;
    let start = line * CHARS_PER_LINE;
    let needle_chars: Vec<char> = needle.chars().collect();
    for (i, ch) in needle_chars.iter().enumerate() {
        if start + i >= symbols.len() {
            break;
        }
        symbols[start + i] = *ch;
    }
    let page_text: String = symbols.into_iter().collect();
    let bytes = text_to_bytes(&page_text)?;
    let v = BigUint::from_bytes_be(&bytes);
    let page = (h[1] as u32 % PAGES_PER_BOOK as u32) + 1;
    let m = mask();
    let b = (v * page_multiplier_inverse(page)) & m;
    Ok((address_from_book(&b, page), page_text))
}

/// Случайный адрес (равномерно по всем страницам).
pub fn random_address() -> Address {
    let mut buf = [0u8; PAGE_BYTES];
    fill_random(&mut buf);
    let v = BigUint::from_bytes_be(&buf);
    let page = (random_u32() % PAGES_PER_BOOK as u32) + 1;
    let m = mask();
    let b = (v * page_multiplier_inverse(page)) & m;
    address_from_book(&b, page)
}

fn fill_random(buf: &mut [u8]) {
    use rand::RngCore;
    rand::thread_rng().fill_bytes(buf);
}

fn random_u32() -> u32 {
    use rand::RngCore;
    rand::thread_rng().next_u32()
}

/// Содержимое всех страниц тома.
pub fn book_text(addr: &Address) -> Result<String, String> {
    let mut out = String::with_capacity(PAGE_SYMBOLS * PAGES_PER_BOOK);
    for page in 1..=PAGES_PER_BOOK as u32 {
        let mut a = addr.clone();
        a.page = page;
        out.push_str(&page_content(&a)?);
        out.push('\n');
    }
    Ok(out)
}

/// Заголовки страниц тома: (номер, первые слова).
pub fn book_contents(addr: &Address) -> Result<Vec<(u32, String)>, String> {
    let mut out = Vec::with_capacity(PAGES_PER_BOOK);
    for page in 1..=PAGES_PER_BOOK as u32 {
        let mut a = addr.clone();
        a.page = page;
        out.push((page, title_of(&page_content(&a)?)));
    }
    Ok(out)
}

/// Заголовок страницы: первые слова первой строки.
///
/// Берутся до 3 слов первой строки (80 символов) — этого хватает, чтобы
/// отличить страницы друг от друга, и заголовок не занимает полшапки.
/// Если слова всё же получаются длиннее лимита, он обрезается с многоточием.
pub fn title_of(text: &str) -> String {
    const MAX_TITLE: usize = 60;
    let first_line: String = text.chars().take(CHARS_PER_LINE).collect();
    let mut title: String = first_line
        .split_whitespace()
        .take(3)
        .collect::<Vec<&str>>()
        .join(" ");
    if title.is_empty() {
        return "(пустая строка)".to_string();
    }
    if title.chars().count() > MAX_TITLE {
        let cut: String = title.chars().take(MAX_TITLE - 1).collect();
        title = format!("{cut}…");
    }
    title
}

/// Разбивает страницу на строки.
pub fn page_lines(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(CHARS_PER_LINE)
        .map(|c| c.iter().collect::<String>())
        .collect()
}

/// Разбор пользовательской строки адреса: `hex-w2-s4-v16` или `hex-w2-s4-v16:12`.
pub fn parse_address(input: &str) -> Result<Address, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("пустой адрес".into());
    }
    let (head, page) = match trimmed.rsplit_once(':') {
        Some((h, p)) => {
            let n: u32 = p
                .parse()
                .map_err(|_| "номер страницы: не число".to_string())?;
            (h.to_string(), Some(n))
        }
        None => (trimmed.to_string(), None),
    };
    let parts: Vec<&str> = head.split('-').collect();
    let hex = parts[0].to_string();
    if hex.is_empty() {
        return Err("в адресе нет имени гекса".into());
    }
    let mut wall = 1u32;
    let mut shelf = 1u32;
    let mut volume = 1u32;
    if parts.len() >= 2 {
        wall = parse_coord(parts[1], "w")?;
    }
    if parts.len() >= 3 {
        shelf = parse_coord(parts[2], "s")?;
    }
    if parts.len() >= 4 {
        volume = parse_coord(parts[3], "v")?;
    }
    Ok(Address {
        hex,
        wall,
        shelf,
        volume,
        page: page.unwrap_or(1),
    })
}

fn parse_coord(part: &str, kind: &str) -> Result<u32, String> {
    let cleaned: String = part.chars().filter(|c| c.is_ascii_digit()).collect();
    if cleaned.is_empty() {
        return Err(format!("координата {}: не число", kind));
    }
    cleaned
        .parse::<u32>()
        .map_err(|_| format!("координата {}: не число", kind))
}

/// Отладочная обёртка: поиск + содержимое страницы по каноническому адресу.
pub mod babel_probe {
    use super::*;

    /// Возвращает (канонический адрес с номером страницы, страница) по искомому тексту.
    pub fn probe_find(text: &str) -> (String, String) {
        let (addr, page) = find_text(text).unwrap();
        (addr.full(), page)
    }

    /// Возвращает страницу по адресу `hex-w2-s4-v16:12`.
    pub fn probe_page(full: &str) -> String {
        let addr = parse_address(full).unwrap();
        page_content(&addr).unwrap()
    }

    /// Короткое имя файла для адреса.
    pub fn probe_file_stem(full: &str) -> String {
        let addr = parse_address(full).unwrap();
        addr.file_stem()
    }

    /// Восстанавливает адрес из содержимого страницы (обратное отображение).
    pub fn probe_address_from_text(text: &str) -> String {
        let addr = address_for_content(text, 1).unwrap();
        addr.full()
    }
}

/// Отладочная обёртка для изображений.
pub mod image_probe {
    use super::*;
    use crate::image as bim;

    /// Делает тестовую PNG-картинку (градиент + круг).
    pub fn probe_make_test_png() -> Vec<u8> {
        use image::{DynamicImage, ImageFormat, Rgb, RgbImage};
        let mut img = RgbImage::new(160, 120);
        for y in 0..120u32 {
            for x in 0..160u32 {
                let r = (x * 255 / 160) as u8;
                let g = (y * 255 / 120) as u8;
                let dx = x as i64 - 80;
                let dy = y as i64 - 60;
                let b = if dx * dx + dy * dy < 30 * 30 { 255 } else { 40 };
                img.put_pixel(x, y, Rgb([r, g, b]));
            }
        }
        let mut buf = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgb8(img)
            .write_to(&mut buf, ImageFormat::Png)
            .unwrap();
        buf.into_inner()
    }

    /// Квантует картинку и возвращает (адрес страницы, текст страницы, PNG).
    pub fn probe_image_roundtrip(png: &[u8]) -> (String, String, Vec<u8>) {
        use image::ImageFormat;
        let img = image::load_from_memory_with_format(png, ImageFormat::Png).unwrap();
        let pixels = bim::quantize(&img);
        let text = bim::pixels_to_text(&pixels).unwrap();
        let addr = address_for_content(&text, 1).unwrap();
        let out = bim::pixels_to_png(&pixels, 2).unwrap();
        (addr.full(), text, out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_address() -> Address {
        Address {
            hex: "jeb0110jlb".to_string(),
            wall: 2,
            shelf: 4,
            volume: 16,
            page: 1,
        }
    }

    #[test]
    fn alphabet_is_64_unique() {
        assert_eq!(ALPHABET.chars().count(), 64);
        let mut seen = std::collections::HashSet::new();
        for c in ALPHABET.chars() {
            assert!(seen.insert(c), "дубликат символа {}", c);
        }
    }

    #[test]
    fn title_never_exceeds_the_header_width() {
        // Любая страница: заголовок обязан влезать в шапку.
        let text = "word ".repeat(200);
        assert!(title_of(&text).chars().count() <= 60);
        assert!(title_of(&"x".repeat(500)).chars().count() <= 60);
    }

    #[test]
    fn title_takes_at_most_three_words() {
        let text = "a b c d e f g h";
        assert_eq!(title_of(text), "a b c");
    }

    #[test]
    fn title_keeps_short_text_intact() {
        assert_eq!(title_of("привет мир"), "привет мир");
        assert_eq!(title_of("   "), "(пустая строка)");
    }

    #[test]
    fn bytes_text_roundtrip() {
        let mut buf = [0u8; PAGE_BYTES];
        fill_random(&mut buf);
        let text = bytes_to_text(&buf);
        assert_eq!(text.chars().count(), PAGE_SYMBOLS);
        let back = text_to_bytes(&text).unwrap();
        assert_eq!(back, buf);
    }

    #[test]
    fn page_roundtrip_is_stable() {
        let addr = dummy_address();
        let text = page_content(&addr).unwrap();
        assert_eq!(text.chars().count(), PAGE_SYMBOLS);
        let recovered = address_for_content(&text, addr.page).unwrap();
        assert_eq!(recovered.canonical(), addr.canonical());
        assert_eq!(page_content(&recovered).unwrap(), text);
    }

    #[test]
    fn different_pages_differ() {
        let mut a = dummy_address();
        a.page = 1;
        let mut b = dummy_address();
        b.page = 2;
        assert_ne!(page_content(&a).unwrap(), page_content(&b).unwrap());
    }

    #[test]
    fn search_finds_text() {
        let needle = "привет мир";
        let (addr, page) = find_text(needle).unwrap();
        assert!(page.contains(needle));
        let reopened = page_content(&addr).unwrap();
        assert_eq!(reopened, page);
    }

    #[test]
    fn random_address_roundtrip() {
        let addr = random_address();
        let text = page_content(&addr).unwrap();
        let recovered = address_for_content(&text, addr.page).unwrap();
        assert_eq!(recovered.canonical(), addr.canonical());
    }

    #[test]
    fn address_parser() {
        let a = parse_address("jeb0110jlb-w2-s4-v16:12").unwrap();
        assert_eq!(a.hex, "jeb0110jlb");
        assert_eq!(a.wall, 2);
        assert_eq!(a.shelf, 4);
        assert_eq!(a.volume, 16);
        assert_eq!(a.page, 12);
    }
}
