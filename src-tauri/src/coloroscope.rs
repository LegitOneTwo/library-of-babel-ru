//! «Колоскоп» — перестановщик цветов.
//!
//! Порядок обхода взят из оригинала: страница coloroscopy.html в архиве
//! изображений libraryofbabel.info (файл js/Coloroscopy.js). Три канала RGB;
//! самый быстрый меняется «пилообразно» (0→255→0), средний и самый медленный
//! считаются как разряды смешанной системы счисления.
//!
//! Оригинал начинает со случайного цвета, но там же оставлены закомментированные
//! `//var r = 0;` — с них начинается чистый обход.

use serde::{Deserialize, Serialize};

/// Канал RGB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    R,
    G,
    B,
}

impl Channel {
    fn get(self, s: &State) -> u16 {
        match self {
            Channel::R => s.r,
            Channel::G => s.g,
            Channel::B => s.b,
        }
    }

    fn set(self, s: &mut State, v: u16) {
        match self {
            Channel::R => s.r = v,
            Channel::G => s.g = v,
            Channel::B => s.b = v,
        }
    }
}

/// Режим обхода: какой канал меняется быстрее всех.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mode {
    /// Порядковый номер режима (0..6).
    pub index: usize,
    /// Название для интерфейса: «быстрый → средний → медленный».
    pub name: &'static str,
    /// Канал, меняющийся на каждом шаге.
    pub fast: Channel,
    /// Канал, растущий на каждом повороте быстрого.
    pub mid: Channel,
    /// Канал, растущий на каждом обороте среднего.
    pub slow: Channel,
}

/// Все шесть режимов — перестановки трёх каналов.
pub const MODES: [Mode; 6] = [
    Mode {
        index: 0,
        name: "синий → зелёный → красный",
        fast: Channel::B,
        mid: Channel::G,
        slow: Channel::R,
    },
    Mode {
        index: 1,
        name: "зелёный → синий → красный",
        fast: Channel::G,
        mid: Channel::B,
        slow: Channel::R,
    },
    Mode {
        index: 2,
        name: "красный → синий → зелёный",
        fast: Channel::R,
        mid: Channel::B,
        slow: Channel::G,
    },
    Mode {
        index: 3,
        name: "красный → зелёный → синий",
        fast: Channel::R,
        mid: Channel::G,
        slow: Channel::B,
    },
    Mode {
        index: 4,
        name: "зелёный → красный → синий",
        fast: Channel::G,
        mid: Channel::R,
        slow: Channel::B,
    },
    Mode {
        index: 5,
        name: "синий → красный → зелёный",
        fast: Channel::B,
        mid: Channel::R,
        slow: Channel::G,
    },
];

/// Шагов в полном цикле: 256 средних × 256 медленных × 512 шагов быстрого.
pub const CYCLE_STEPS: u64 = 256 * 256 * 512;

/// Число различных цветов в обходе.
pub const DISTINCT_COLORS: u64 = 256 * 256 * 256;

/// Состояние обхода.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct State {
    /// Номер режима (0..6).
    pub mode: usize,
    pub r: u16,
    pub g: u16,
    pub b: u16,
    /// 0 — быстрый канал растёт, 1 — падает.
    pub counter: u8,
    /// Сколько шагов сделано с начала.
    pub step: u64,
}

impl State {
    /// Режим по номеру (с витком по кругу).
    pub fn mode_info(&self) -> &'static Mode {
        &MODES[self.mode % MODES.len()]
    }

    /// Один шаг обхода — ровно как в оригинальном JS.
    pub fn advance(&mut self) {
        let mode = self.mode_info();
        let (fast, mid, slow) = (mode.fast, mode.mid, mode.slow);

        if self.counter == 0 {
            // Растущая фаза: быстрый канал идёт вверх.
            let f = fast.get(self);
            let m = mid.get(self);
            if f < 255 {
                fast.set(self, f + 1);
            } else if m < 255 {
                mid.set(self, m + 1);
                self.counter = 1;
            } else {
                // Оборот среднего: растёт медленный, быстрый и средний в ноль.
                let s = slow.get(self);
                if s < 255 {
                    slow.set(self, s + 1);
                } else {
                    slow.set(self, 0);
                }
                mid.set(self, 0);
                fast.set(self, 0);
            }
        } else {
            // Падающая фаза: быстрый канал идёт вниз.
            let f = fast.get(self);
            let m = mid.get(self);
            if f > 0 {
                fast.set(self, f - 1);
            } else if m < 255 {
                mid.set(self, m + 1);
                self.counter = 0;
            } else {
                let s = slow.get(self);
                if s < 255 {
                    slow.set(self, s + 1);
                } else {
                    slow.set(self, 0);
                }
                mid.set(self, 0);
                fast.set(self, 0);
                self.counter = 0;
            }
        }
        self.step = self.step.wrapping_add(1);
    }

    /// Несколько шагов подряд.
    pub fn advance_by(&mut self, n: u64) {
        for _ in 0..n {
            self.advance();
        }
    }

    /// Цвет как `#rrggbb`.
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Цвет как `rgb(r, g, b)`.
    pub fn rgb(&self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }

    /// Начальное состояние со случайным цветом — как в оригинале.
    pub fn random(mode: usize) -> State {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        State {
            mode: mode % MODES.len(),
            r: rng.gen_range(0..256),
            g: rng.gen_range(0..256),
            b: rng.gen_range(0..256),
            counter: 0,
            step: 0,
        }
    }
}

/// Последовательность состояний обхода — для сверки с оригиналом.
pub fn seq(mode: usize, steps: usize) -> Vec<State> {
    let mut s = State {
        mode: mode % MODES.len(),
        ..State::default()
    };
    let mut out = Vec::with_capacity(steps);
    for _ in 0..steps {
        s.advance();
        out.push(s);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Первые 20 шагов режима 0 — как в оригинальном JS при r=g=b=0.
    #[test]
    fn mode0_matches_original_js() {
        let mut s = State::default();
        let mut got = Vec::new();
        for _ in 0..20 {
            s.advance();
            got.push((s.r, s.g, s.b));
        }
        // Оригинал: b растёт 0..255, потом g, потом r.
        let expect: Vec<(u16, u16, u16)> = (0..20).map(|i| (0, 0, i as u16 + 1)).collect();
        assert_eq!(got, expect);
    }

    /// После полного оборота быстрого канала средний растёт, быстрый падает.
    #[test]
    fn triangle_wave_shape() {
        let mut s = State::default();
        for _ in 0..256 {
            s.advance();
        }
        assert_eq!((s.r, s.g, s.b), (0, 1, 255));
        assert_eq!(s.counter, 1);
        s.advance();
        assert_eq!((s.r, s.g, s.b), (0, 1, 254));
        assert_eq!(s.counter, 1);
    }

    /// Все режимы ведут себя одинаково по форме, но по своим каналам.
    #[test]
    fn every_mode_runs() {
        for m in 0..MODES.len() {
            let mut s = State {
                mode: m,
                ..State::default()
            };
            s.advance_by(1000);
            let f = s.mode_info().fast;
            let v = match f {
                Channel::R => s.r,
                Channel::G => s.g,
                Channel::B => s.b,
            };
            assert!(v > 0, "режим {} не меняет быстрый канал", m);
            assert_eq!(s.step, 1000);
        }
    }

    /// Полный цикл возвращает в начальное состояние.
    #[test]
    fn full_cycle_returns_to_start() {
        let mut s = State::default();
        s.advance_by(CYCLE_STEPS);
        assert_eq!((s.r, s.g, s.b), (0, 0, 0));
        assert_eq!(s.counter, 0);
        assert_eq!(s.step, CYCLE_STEPS);
    }

    /// Hex и rgb совпадают с каналами.
    #[test]
    fn colour_formats() {
        let s = State {
            r: 18,
            g: 52,
            b: 86,
            ..State::default()
        };
        assert_eq!(s.hex(), "#123456");
        assert_eq!(s.rgb(), "rgb(18, 52, 86)");
    }

    /// Шесть режимов — все перестановки каналов.
    #[test]
    fn modes_are_permutations() {
        for m in &MODES {
            let mut seen = vec![m.fast, m.mid, m.slow];
            seen.sort_by_key(|c| match c {
                Channel::R => 0,
                Channel::G => 1,
                Channel::B => 2,
            });
            assert_eq!(seen, vec![Channel::R, Channel::G, Channel::B]);
        }
    }

    /// Эталон из оригинального js/Coloroscopy.js: первые шаги каждого режима
    /// при старте с нуля. Если порт разойдётся с оригиналом — тест упадёт.
    #[test]
    fn matches_original_js_sequences() {
        // Ожидаемая последовательность: (режим, шаг) -> (r, g, b, counter),
        // снято с оригинального JS.
        type Expected = (usize, usize, (u16, u16, u16, u8));
        let expected: [Expected; 12] = [
            (0, 0, (0, 0, 1, 0)),
            (0, 254, (0, 0, 255, 0)),
            (0, 255, (0, 1, 255, 1)),
            (0, 256, (0, 1, 254, 1)),
            (1, 0, (0, 1, 0, 0)),
            (1, 255, (0, 255, 1, 1)),
            (2, 0, (1, 0, 0, 0)),
            (2, 255, (255, 0, 1, 1)),
            (3, 0, (1, 0, 0, 0)),
            (3, 255, (255, 1, 0, 1)),
            (4, 0, (0, 1, 0, 0)),
            (5, 255, (1, 0, 255, 1)),
        ];
        for (mode, step, want) in expected {
            let seq = seq(mode, step + 1);
            let got = seq[step];
            assert_eq!(
                (got.r, got.g, got.b, got.counter),
                want,
                "режим {} шаг {} разошёлся с оригиналом",
                mode,
                step
            );
        }
    }
}
