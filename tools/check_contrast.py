"""Проверка контраста всех пар, которые реально использует интерфейс.

Список пар выведен из styles.css: каждое правило читает конкретные
переменные, значит и контраст считать надо для конкретных пар.

Запуск:  python tools/check_contrast.py

Считает контраст по формуле WCAG 2.x, без внешних зависимостей.
Порог 4.5:1 для обычного текста, 3:1 для нетекстовых элементов
(границы, индикаторы) по WCAG 1.4.11.

Нулевой код возврата означает, что все пары обеих тем проходят.
"""

import sys

# --- Формула WCAG 2.x -------------------------------------------------------


def luminance(hex_color: str) -> float:
    """Относительная яркость цвета по WCAG."""
    h = hex_color.lstrip("#")
    if len(h) == 3:
        h = "".join(c * 2 for c in h)
    r, g, b = (int(h[i : i + 2], 16) / 255 for i in (0, 2, 4))

    def lin(c: float) -> float:
        return c / 12.92 if c <= 0.03928 else ((c + 0.055) / 1.055) ** 2.4

    return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b)


def contrast(a: str, b: str) -> float:
    """Контраст двух цветов: 1.0 (неразличимы) .. 21.0 (чёрное на белом)."""
    l1, l2 = sorted((luminance(a), luminance(b)), reverse=True)
    return (l1 + 0.05) / (l2 + 0.05)


def hx(h: str):
    h = h.lstrip("#")
    return tuple(int(h[i : i + 2], 16) for i in (0, 2, 4))


def mix(fg: str, bg: str, pct: float) -> str:
    """Смесь fg поверх bg, pct процентов непрозрачности fg."""
    f, b = hx(fg), hx(bg)
    return "#%02x%02x%02x" % tuple(
        round(f[i] * pct / 100 + b[i] * (1 - pct / 100)) for i in range(3)
    )


# --- Палитры: те же значения, что в styles.css ------------------------------


def theme(
    bg, panel, panel2, line, line_soft, ink, ink2, ink3,
    accent, accent_ink, danger, ok, mark_bg, accent_tint, danger_tint,
):
    t = {
        "bg": bg,
        "panel": panel,
        "panel-2": panel2,
        "line": line,
        "line-soft": line_soft,
        "ink": ink,
        "ink-2": ink2,
        "ink-3": ink3,
        "accent": accent,
        "accent-ink": accent_ink,
        "danger": danger,
        "ok": ok,
        "mark-bg": mark_bg,
        "accent-tint": accent_tint,
        "danger-tint": danger_tint,
    }
    return t


DARK = theme(
    bg="#1e1e2e",
    panel="#181825",
    panel2="#313244",
    line="#7c7f93",
    line_soft="#313244",
    ink="#cdd6f4",
    ink2="#a6adc8",
    ink3="#9399b2",
    accent="#fab387",
    accent_ink="#181825",
    danger="#f38ba8",
    ok="#a6e3a1",
    mark_bg="#5c4642",
    accent_tint="#51474f",
    danger_tint="#41394c",
)

LIGHT = theme(
    bg="#eff1f5",
    panel="#e6e9ef",
    panel2="#ccd0da",
    line="#666a80",
    line_soft="#dce0e8",
    ink="#4c4f69",
    ink2="#54586e",
    ink3="#63667d",
    accent="#8a4010",
    accent_ink="#ffffff",
    danger="#aa2333",
    ok="#236b24",
    mark_bg="#d0c0b9",
    accent_tint="#d9d1d0",
    danger_tint="#decdd5",
)


# --- Правила: (название, текст, фон, порог) --------------------------------


def rules(t):
    bg, panel, p2 = t["bg"], t["panel"], t["panel-2"]
    accent, danger, ok = t["accent"], t["danger"], t["ok"]
    ink, ink2, ink3 = t["ink"], t["ink-2"], t["ink-3"]
    line = t["line"]
    return [
        ("текст на фоне", ink, bg, 4.5),
        ("подпись бренда", ink3, panel, 4.5),
        ("заголовок бренда", ink, panel, 4.5),
        ("подпись группы меню", ink3, panel, 4.5),
        ("пункт меню", ink2, panel, 4.5),
        ("пункт меню при наведении", ink, p2, 4.5),
        ("активный пункт меню", accent, p2, 4.5),
        ("команда в меню", ink3, panel, 4.5),
        ("команда при наведении", ink, panel, 4.5),
        ("подсказка клавиши", ink3, panel, 4.5),
        ("подпись спецификации", ink3, panel, 4.5),
        ("значение спецификации", ink2, panel, 4.5),
        ("адрес в шапке", ink3, panel, 4.5),
        ("кнопка меню на узком экране", ink, p2, 4.5),
        ("заголовок панели", ink, panel, 4.5),
        ("обычный текст", ink2, panel, 4.5),
        ("строка состояния", ink3, panel, 4.5),
        ("ошибка: заголовок", danger, panel, 4.5),
        ("ошибка: пояснение", ink2, panel, 4.5),
        ("подпись поля", ink3, panel, 4.5),
        ("поле ввода", ink, bg, 4.5),
        ("подсказка в поле", ink3, bg, 4.5),
        ("кнопка", ink, p2, 4.5),
        ("главная кнопка", t["accent-ink"], accent, 4.5),
        ("тихая кнопка", ink2, panel, 4.5),
        ("заголовок страницы", ink, panel, 4.5),
        ("координаты", ink3, panel, 4.5),
        ("текст страницы", ink2, panel, 4.5),
        ("строка страницы при наведении", ink2, p2, 4.5),
        ("подсветка в тексте", ink, t["mark-bg"], 4.5),
        ("номер страницы в поле", ink, bg, 4.5),
        ("подпись «из N»", ink3, panel, 4.5),
        ("пункт оглавления", ink2, panel, 4.5),
        ("номер в оглавлении", accent, panel, 4.5),
        ("адрес закладки", ink, bg, 4.5),
        ("метаданные закладки", ink3, bg, 4.5),
        ("режим колоскопа", ink2, bg, 4.5),
        ("активный режим", ink, p2, 4.5),
        ("каналы режима", ink3, bg, 4.5),
        ("буква алфавита", ink2, p2, 4.5),
        ("кириллица в алфавите", accent, t["accent-tint"], 4.5),
        ("знак в алфавите", danger, t["danger-tint"], 4.5),
        ("ссылка", accent, panel, 4.5),
        ("тост", ink, p2, 4.5),
        # нетекстовые: границы и индикаторы, порог 3:1 (WCAG 1.4.11)
        ("граница поля", line, bg, 3.0),
        ("граница тоста", line, p2, 3.0),
        ("линия команд", line, panel, 3.0),
        ("фокус на фоне", accent, bg, 3.0),
        ("фокус на панели", accent, panel, 3.0),
        ("спиннер", line, bg, 3.0),
        ("плашка тоста: акцент", accent, p2, 3.0),
        ("плашка тоста: ошибка", danger, p2, 3.0),
        ("плашка тоста: успех", ok, p2, 3.0),
    ]


def check(name, t):
    print(f"=== {name} ===")
    bad = []
    for label, fg, bg, need in rules(t):
        r = contrast(fg, bg)
        if r < need:
            bad.append((label, fg, bg, r, need))
    for label, fg, bg, r, need in bad:
        print(f"  FAIL {label}: {fg} на {bg} = {r:.2f}, нужно {need}")
    if not bad:
        print(f"  все пары проходят ({len(rules(t))} проверок)")
    else:
        print(f"  {len(bad)} пар ниже порога")
    return bad


if __name__ == "__main__":
    failures = check("ТЁМНАЯ (Catppuccin Mocha)", DARK)
    failures += check("СВЕТЛАЯ (Catppuccin Latte)", LIGHT)
    if failures:
        print(f"\nИТОГ: {len(failures)} пар не проходят")
        sys.exit(1)
    print("\nИТОГ: все пары обеих тем проходят")
