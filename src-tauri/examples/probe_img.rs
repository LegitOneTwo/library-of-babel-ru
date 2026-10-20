//! Сквозная проверка архива изображений: вписывание картинки, поиск
//! локации, восстановление и сверка с исходником.

use library_of_babel_ru_lib::imgarch_probe;

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "test_pattern.png".to_string());
    match imgarch_probe(&path) {
        Ok(report) => println!("{report}"),
        Err(e) => {
            eprintln!("ошибка: {e}");
            std::process::exit(1);
        }
    }
}
