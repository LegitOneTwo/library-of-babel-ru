//! Сверка порта «колоскопа» с оригинальным JS.
//!
//! Печатает последовательность цветов в том же формате, что и
//! js/Coloroscopy.js, чтобы можно было сравнить файлы.

use library_of_babel_ru_lib::coloroscope_seq;

fn main() {
    let mode: usize = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let steps: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1200);

    let seq = coloroscope_seq(mode, steps);
    for (i, s) in seq.iter().enumerate() {
        println!("{}: {},{},{},{}", i, s.r, s.g, s.b, s.counter);
    }
}
