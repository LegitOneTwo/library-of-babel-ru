//! Tauri-команды «колоскопа» — перестановщика цветов.

use crate::coloroscope as cs;
use serde::Serialize;

/// Кадр обхода для интерфейса.
#[derive(Serialize, Clone, Debug)]
pub struct ColorFrame {
    /// `#rrggbb`.
    pub hex: String,
    /// `rgb(r, g, b)`.
    pub rgb: String,
    /// Номер режима (0..6).
    pub mode: usize,
    /// Название режима: «быстрый → средний → медленный».
    pub mode_name: String,
    /// Каналы RGB.
    pub r: u16,
    pub g: u16,
    pub b: u16,
    /// Фаза обхода: 0 — быстрый канал растёт, 1 — падает.
    pub counter: u8,
    /// Полное число шагов с начала.
    pub step: u64,
    /// Шаг внутри текущего цикла.
    pub step_in_cycle: u64,
    /// Полный цикл.
    pub cycle_steps: u64,
    /// Число различных цветов.
    pub distinct_colors: u64,
    /// Идёт ли обход.
    pub playing: bool,
}

/// Список режимов для интерфейса.
#[derive(Serialize, Clone, Debug)]
pub struct ModeInfo {
    pub index: usize,
    pub name: String,
    /// Названия каналов: быстрый, средний, медленный.
    pub channels: [String; 3],
}

/// Новый обход: режим и начальное состояние.
#[tauri::command]
pub async fn coloroscope_new(
    mode: Option<usize>,
    random: Option<bool>,
) -> Result<ColorFrame, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mode = mode.unwrap_or(0);
        let state = if random.unwrap_or(false) {
            cs::State::random(mode)
        } else {
            cs::State {
                mode: mode % cs::MODES.len(),
                ..cs::State::default()
            }
        };
        Ok(build(state, false))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Продвинуть обход на n шагов.
#[tauri::command]
pub async fn coloroscope_step(
    state: cs::State,
    steps: u64,
    playing: Option<bool>,
) -> Result<ColorFrame, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut s = state;
        s.advance_by(steps);
        Ok(build(s, playing.unwrap_or(false)))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Переключить режим.
#[tauri::command]
pub async fn coloroscope_set_mode(state: cs::State, mode: usize) -> Result<ColorFrame, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut s = state;
        s.mode = mode % cs::MODES.len();
        Ok(build(s, false))
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Список режимов.
#[tauri::command]
pub fn coloroscope_modes() -> Vec<ModeInfo> {
    cs::MODES
        .iter()
        .map(|m| ModeInfo {
            index: m.index,
            name: m.name.to_string(),
            channels: [
                channel_name(m.fast).to_string(),
                channel_name(m.mid).to_string(),
                channel_name(m.slow).to_string(),
            ],
        })
        .collect()
}

fn channel_name(c: cs::Channel) -> &'static str {
    match c {
        cs::Channel::R => "красный",
        cs::Channel::G => "зелёный",
        cs::Channel::B => "синий",
    }
}

fn build(state: cs::State, playing: bool) -> ColorFrame {
    let step_in_cycle = state.step % cs::CYCLE_STEPS;
    ColorFrame {
        hex: state.hex(),
        rgb: state.rgb(),
        mode: state.mode % cs::MODES.len(),
        mode_name: state.mode_info().name.to_string(),
        r: state.r,
        g: state.g,
        b: state.b,
        counter: state.counter,
        step: state.step,
        step_in_cycle,
        cycle_steps: cs::CYCLE_STEPS,
        distinct_colors: cs::DISTINCT_COLORS,
        playing,
    }
}
