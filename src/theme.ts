/**
 * Переключение темы.
 *
 * Тема задаётся атрибутом data-theme на <html>: "light" включает светлую,
 * отсутствие атрибута означает тёмную. Компоненты о теме ничего не знают,
 * они читают только CSS-переменные, поэтому переключение мгновенное и не
 * требует перерисовки интерфейса.
 *
 * Выбор сохраняется в localStorage: тема это настройка пользователя,
 * а не состояние сессии, и она должна пережить перезапуск приложения.
 */

export type Theme = "dark" | "light";

const STORAGE_KEY = "babel.theme";

function readStored(): Theme | null {
  try {
    const value = window.localStorage.getItem(STORAGE_KEY);
    return value === "light" || value === "dark" ? value : null;
  } catch {
    // Приватный режим или запрещённое хранилище: тема просто не запомнится.
    return null;
  }
}

/**
 * Тема по умолчанию: тёмная.
 *
 * Причина: продукт это инструмент чтения плотного текста и ночная
 * библиотека, тёмная поверхность не слепит на длинном сеансе.
 * Владелец выбрал Catppuccin Mocha, поэтому тёмная тема основная,
 * а светлая (Latte) даёт выбор для яркого окружения.
 */
export function currentTheme(): Theme {
  const attr = document.documentElement.getAttribute("data-theme");
  if (attr === "light") return "light";
  return "dark";
}

export function applyTheme(theme: Theme): void {
  if (theme === "light") {
    document.documentElement.setAttribute("data-theme", "light");
  } else {
    document.documentElement.removeAttribute("data-theme");
  }
}

/** Включает тему и запоминает выбор. */
export function setTheme(theme: Theme): void {
  applyTheme(theme);
  try {
    window.localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // Сохранить не удалось: тема применится, но не переживёт перезапуск.
  }
}

/** Переключает тему на противоположную. */
export function toggleTheme(): Theme {
  const next: Theme = currentTheme() === "light" ? "dark" : "light";
  setTheme(next);
  return next;
}

/**
 * Применяет сохранённую тему при старте.
 *
 * Вызывается до первой отрисовки, иначе интерфейс мелькнёт тёмным.
 */
export function initTheme(): void {
  const stored = readStored();
  if (stored) applyTheme(stored);
}
