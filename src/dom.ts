/** Мелые DOM-хелперы: экранирование, подсветка, сокращение адресов. */

export function esc(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

/** Подсвечивает вхождения подстроки в тексте, возвращая безопасный HTML. */
export function highlight(text: string, needle: string): string {
  if (!needle) return esc(text);
  const lower = text.toLowerCase();
  const target = needle.toLowerCase();
  let out = "";
  let i = 0;
  while (i < text.length) {
    const found = lower.indexOf(target, i);
    if (found === -1) {
      out += esc(text.slice(i));
      break;
    }
    out += esc(text.slice(i, found));
    out += `<mark>${esc(text.slice(found, found + target.length))}</mark>`;
    i = found + target.length;
  }
  return out;
}

/** Короткая дата из unix-времени. */
export function formatDate(unix: number): string {
  if (!unix) return "";
  const d = new Date(unix * 1000);
  return d.toLocaleString("ru-RU", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/**
 * Сокращает длинный адрес для показа: `начало…конец`.
 *
 * Имя гекса это base-36 от 19200-битного числа, то есть до 11 500 символов.
 * Выводить его целиком нельзя: он не влезает ни в одну строку интерфейса
 * и выталкивает остальные элементы за границу окна.
 */
export function shortenAddress(full: string, head = 10, tail = 8): string {
  if (full.length <= head + tail + 1) return full;
  return `${full.slice(0, head)}…${full.slice(-tail)}`;
}
