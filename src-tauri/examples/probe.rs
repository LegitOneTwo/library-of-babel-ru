use library_of_babel_ru_lib::babel_probe::*;
use library_of_babel_ru_lib::image_probe::*;

/// Проверка обратимости на нескольких адресах и страницах.
fn main() {
    let cases = [
        ("jeb0110jlb", 2, 4, 16),
        ("0", 1, 1, 1),
        ("zzz", 4, 5, 32),
        ("a", 1, 1, 1),
        ("7f3k9x", 3, 2, 7),
    ];
    for (hex, wall, shelf, volume) in cases {
        for page in [1u32, 2, 100, 410] {
            let addr = format!("{}-w{}-s{}-v{}:{}", hex, wall, shelf, volume, page);
            let text = probe_page(&addr);
            let n = text.chars().count();
            assert_eq!(n, 3200, "длина страницы {} = {}", addr, n);
        }
        println!("ok: {}-w{}-s{}-v{}", hex, wall, shelf, volume);
    }

    // Поиск кириллицы: страница обязана содержать запрос и совпадать при переоткрытии.
    for needle in [
        "привет мир",
        "библиотека бабля",
        "борхес",
        "hello world",
        "съешь ещё этих мягких французских булок",
    ] {
        let (addr, page) = probe_find(needle);
        assert!(page.contains(needle), "запрос {:?} не найден", needle);
        assert_eq!(page.chars().count(), 3200, "длина найденной страницы");
        let reopened = probe_page(&addr);
        assert_eq!(reopened, page, "страница изменилась при переоткрытии");
        println!("ok поиск {:?} -> {} симв.", needle, page.chars().count());
    }
    // Короткое имя файла: без гигантского имени гекса.
    let (addr, _) = probe_find("имя файла");
    let stem = probe_file_stem(&addr);
    assert!(stem.len() < 60, "имя файла слишком длинное: {}", stem);
    assert!(!stem.contains(' '), "пробел в имени файла");
    println!("ok имя файла: {} ({} симв.)", stem, stem.len());

    // Изображение -> страница -> адрес: картинка живёт в библиотеке навсегда.
    let png = probe_make_test_png();
    let (img_addr, quantized, reopened_png) = probe_image_roundtrip(&png);
    assert_eq!(quantized.chars().count(), 3200, "квантованная страница");
    assert!(reopened_png.len() > 100, "перерисованный PNG пуст");
    // Адрес восстанавливается из самого содержимого страницы.
    let addr2 = probe_address_from_text(&quantized);
    assert_eq!(addr2, img_addr, "адрес картинки не восстановился");
    // И по этому адресу открывается ровно та же страница.
    let reopened = probe_page(&img_addr);
    assert_eq!(reopened, quantized, "страница картинки не совпала");
    println!("ok изображение -> страница -> адрес: {}", img_addr);

    println!("ВСЁ СОШЛОСЬ");
}
