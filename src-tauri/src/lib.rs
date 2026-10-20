mod babel;
mod coloroscope;
mod coloroscope_cmd;
mod image;
mod image_cmd;
mod imgarch;
mod imgarch_cmd;
pub use babel::babel_probe;
pub use babel::image_probe;
pub use coloroscope::seq as coloroscope_seq;
pub use imgarch::probe as imgarch_probe;

use babel::Address;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Serialize, Clone)]
pub struct AddressInfo {
    pub hex: String,
    pub wall: u32,
    pub shelf: u32,
    pub volume: u32,
    pub page: u32,
    pub canonical: String,
    pub full: String,
    pub hash: String,
}

#[derive(Serialize)]
pub struct PageInfo {
    pub text: String,
    pub lines: Vec<String>,
    pub title: String,
    pub address: AddressInfo,
    pub pages_in_book: u32,
    pub chars_per_line: usize,
    pub lines_per_page: usize,
    /// Предупреждения: какие символы запроса не входят в алфавит.
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct ContentsEntry {
    pub page: u32,
    pub title: String,
}

#[derive(Serialize, Clone, Deserialize)]
pub struct Bookmark {
    pub hash: String,
    pub hex: String,
    pub wall: u32,
    pub shelf: u32,
    pub volume: u32,
    pub page: u32,
    pub canonical: String,
    pub full: String,
    pub saved_at: u64,
}

#[derive(Serialize)]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub alphabet: String,
    pub alphabet_size: usize,
    pub page_symbols: usize,
    pub lines_per_page: usize,
    pub chars_per_line: usize,
    pub pages_per_book: u32,
    pub walls: u32,
    pub shelves: u32,
    pub volumes: u32,
}

pub fn to_info(addr: &Address) -> AddressInfo {
    AddressInfo {
        hex: addr.hex.clone(),
        wall: addr.wall,
        shelf: addr.shelf,
        volume: addr.volume,
        page: addr.page,
        canonical: addr.canonical(),
        full: addr.full(),
        hash: addr.hash(),
    }
}

fn build_page(addr: &Address) -> Result<PageInfo, String> {
    let text = babel::page_content(addr)?;
    Ok(PageInfo {
        title: babel::title_of(&text),
        lines: babel::page_lines(&text),
        text,
        address: to_info(addr),
        pages_in_book: babel::PAGES_PER_BOOK as u32,
        chars_per_line: babel::CHARS_PER_LINE,
        lines_per_page: babel::LINES_PER_PAGE,
        warnings: Vec::new(),
    })
}

/// Символы, отсутствующие в алфавите, с пояснением чем они заменены.
fn alphabet_warnings(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: Vec<char> = Vec::new();
    for ch in text.chars() {
        for c in ch.to_lowercase() {
            if c == '?' || babel::symbol_value(c).is_some() {
                continue;
            }
            if !seen.contains(&c) {
                seen.push(c);
                out.push(format!(
                    "Символ {:?} не входит в алфавит, заменён пробелом",
                    c
                ));
            }
        }
    }
    out
}

#[tauri::command]
fn open_page(
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
    page: u32,
) -> Result<PageInfo, String> {
    let addr = Address {
        hex,
        wall,
        shelf,
        volume,
        page,
    };
    build_page(&addr)
}

#[tauri::command]
fn search_page(text: String) -> Result<PageInfo, String> {
    let (addr, page) = babel::find_text(&text)?;
    let mut info = build_page(&addr)?;
    info.lines = babel::page_lines(&page);
    info.title = babel::title_of(&page);
    info.text = page;
    info.warnings = alphabet_warnings(&text);
    Ok(info)
}

#[tauri::command]
fn random_page() -> Result<PageInfo, String> {
    let addr = babel::random_address();
    build_page(&addr)
}

#[tauri::command]
async fn book_contents(
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
) -> Result<Vec<ContentsEntry>, String> {
    let addr = Address {
        hex,
        wall,
        shelf,
        volume,
        page: 1,
    };
    tauri::async_runtime::spawn_blocking(move || babel::book_contents(&addr))
        .await
        .map_err(|e| e.to_string())?
        .map(|v| {
            v.into_iter()
                .map(|(page, title)| ContentsEntry { page, title })
                .collect()
        })
}

#[tauri::command]
async fn save_book(
    path: String,
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
) -> Result<String, String> {
    let addr = Address {
        hex,
        wall,
        shelf,
        volume,
        page: 1,
    };
    let path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        let text = babel::book_text(&addr)?;
        let mut out = String::with_capacity(text.len() + 64);
        out.push_str(&format!("# адрес: {}\n", addr.canonical()));
        out.push_str(&text);
        fs::write(&path, out).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Короткое имя файла для адреса (без гигантского имени гекса).
#[tauri::command]
fn suggest_file_name(hex: String, wall: u32, shelf: u32, volume: u32, page: u32) -> String {
    Address {
        hex,
        wall,
        shelf,
        volume,
        page,
    }
    .file_stem()
}

/// Читает файл и возвращает адрес страницы, из которого он был получен.
/// Формат файла: первая строка — заголовок с адресом, далее текст.
#[tauri::command]
async fn open_saved_file(path: String) -> Result<PageInfo, String> {
    let path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        let data = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let (head, body) = split_saved_file(&data);
        // Адрес может быть записан в заголовке файла.
        if let Some(addr) = head {
            let text = if body.trim().is_empty() {
                babel::page_content(&addr)?
            } else {
                body
            };
            let mut info = build_page(&addr)?;
            info.lines = babel::page_lines(&text);
            info.title = babel::title_of(&text);
            info.text = text;
            return Ok(info);
        }
        // Иначе адрес восстанавливаем из самого текста: страница однозначно
        // определяет своё место (обратимое отображение).
        let text = body.trim_end_matches('\n').to_string();
        if text.chars().count() < babel::PAGE_SYMBOLS {
            return Err("файл слишком короткий: это не страница библиотеки".into());
        }
        let text: String = text.chars().take(babel::PAGE_SYMBOLS).collect();
        let addr = babel::address_for_content(&text, 1)?;
        let mut info = build_page(&addr)?;
        info.text = text;
        info.lines = babel::page_lines(&info.text);
        info.title = babel::title_of(&info.text);
        Ok(info)
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Разбирает сохранённый файл: заголовок с адресом (если есть) и тело.
fn split_saved_file(data: &str) -> (Option<Address>, String) {
    let mut lines = data.lines();
    let first = lines.next().unwrap_or("");
    let rest: String = lines.collect::<Vec<_>>().join("\n");
    let trimmed = first.trim();
    // Заголовок вида: "# адрес: jeb0110jlb-w2-s4-v16:12" или просто адрес.
    let candidate = trimmed
        .strip_prefix('#')
        .map(|s| s.trim())
        .and_then(|s| s.strip_prefix("адрес:").map(|s| s.trim()))
        .unwrap_or(trimmed);
    if !candidate.is_empty() && candidate.contains("-w") {
        if let Ok(addr) = babel::parse_address(candidate) {
            return (Some(addr), rest);
        }
    }
    (None, data.to_string())
}

#[tauri::command]
async fn save_page(
    path: String,
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
    page: u32,
) -> Result<String, String> {
    let addr = Address {
        hex,
        wall,
        shelf,
        volume,
        page,
    };
    let path = PathBuf::from(path);
    tauri::async_runtime::spawn_blocking(move || {
        let text = babel::page_content(&addr)?;
        // Первая строка — адрес, чтобы файл можно было открыть обратно.
        let mut out = String::with_capacity(text.len() + 64);
        out.push_str(&format!("# адрес: {}\n", addr.full()));
        out.push_str(&text);
        fs::write(&path, out).map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn bookmarks_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("не удалось найти каталог данных: {}", e))?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("bookmarks.json"))
}

fn load_bookmarks(app: &AppHandle) -> BTreeMap<String, Bookmark> {
    let mut map = BTreeMap::new();
    if let Ok(path) = bookmarks_path(app) {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(parsed) = serde_json::from_str::<BTreeMap<String, Bookmark>>(&data) {
                map = parsed;
            }
        }
    }
    map
}

fn save_bookmarks(app: &AppHandle, map: &BTreeMap<String, Bookmark>) -> Result<(), String> {
    let path = bookmarks_path(app)?;
    let data = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_bookmarks(app: AppHandle) -> Vec<Bookmark> {
    load_bookmarks(&app).into_values().collect()
}

#[tauri::command]
fn add_bookmark(
    app: AppHandle,
    hex: String,
    wall: u32,
    shelf: u32,
    volume: u32,
    page: u32,
) -> Result<Vec<Bookmark>, String> {
    let addr = Address {
        hex,
        wall,
        shelf,
        volume,
        page,
    };
    let mut map = load_bookmarks(&app);
    let hash = addr.hash();
    map.insert(
        hash.clone(),
        Bookmark {
            hash,
            hex: addr.hex.clone(),
            wall: addr.wall,
            shelf: addr.shelf,
            volume: addr.volume,
            page: addr.page,
            canonical: addr.canonical(),
            full: addr.full(),
            saved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        },
    );
    save_bookmarks(&app, &map)?;
    Ok(map.into_values().collect())
}

#[tauri::command]
fn remove_bookmark(app: AppHandle, hash: String) -> Result<Vec<Bookmark>, String> {
    let mut map = load_bookmarks(&app);
    map.remove(&hash);
    save_bookmarks(&app, &map)?;
    Ok(map.into_values().collect())
}

#[tauri::command]
fn parse_address(input: String) -> Result<AddressInfo, String> {
    let addr = babel::parse_address(&input)?;
    Ok(to_info(&addr))
}

#[tauri::command]
fn address_from_text(text: String, page: u32) -> Result<AddressInfo, String> {
    let addr = babel::address_for_content(&text, page)?;
    Ok(to_info(&addr))
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        name: "Вавилонская Библиотека".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        alphabet: babel::ALPHABET.to_string(),
        alphabet_size: babel::ALPHABET.chars().count(),
        page_symbols: babel::PAGE_SYMBOLS,
        lines_per_page: babel::LINES_PER_PAGE,
        chars_per_line: babel::CHARS_PER_LINE,
        pages_per_book: babel::PAGES_PER_BOOK as u32,
        walls: babel::WALLS,
        shelves: babel::SHELVES,
        volumes: babel::VOLUMES,
    }
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("разрешены только http(s)-ссылки".into());
    }
    tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_page,
            search_page,
            random_page,
            book_contents,
            save_book,
            save_page,
            list_bookmarks,
            add_bookmark,
            remove_bookmark,
            parse_address,
            address_from_text,
            suggest_file_name,
            open_saved_file,
            image_cmd::search_image,
            image_cmd::render_page_as_image,
            image_cmd::save_page_image,
            image_cmd::save_image_match,
            image_cmd::save_data_url,
            imgarch_cmd::archive_random,
            imgarch_cmd::archive_by_location,
            imgarch_cmd::archive_step,
            imgarch_cmd::archive_search_image,
            imgarch_cmd::archive_save_png,
            imgarch_cmd::archive_info,
            coloroscope_cmd::coloroscope_new,
            coloroscope_cmd::coloroscope_step,
            coloroscope_cmd::coloroscope_set_mode,
            coloroscope_cmd::coloroscope_modes,
            app_info,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("ошибка запуска приложения");
}
