/**
 * Всплывающие сообщения.
 *
 * Живут в отдельном контейнере вне #app, поэтому обновление тоста не
 * перерисовывает интерфейс. Это важно для кнопок «Пуск» и «Стоп»: полная
 * перерисовка внутри цикла пересоздаёт кнопку между нажатием и отпусканием,
 * и клик не срабатывает.
 */

import { esc } from "./dom";

export type ToastKind = "ok" | "error" | "info";

let timer: number | undefined;

function host(): HTMLElement | null {
  return document.getElementById("toast-host");
}

export function showToast(text: string, kind: ToastKind = "info"): void {
  const root = host();
  if (!root) return;
  const cls = kind === "error" ? "error" : kind === "ok" ? "ok" : "";
  const live = kind === "error" ? "assertive" : "polite";
  root.innerHTML = `<div class="toast ${cls}" role="status" aria-live="${live}">${esc(
    text
  )}</div>`;
  window.clearTimeout(timer);
  timer = window.setTimeout(() => {
    root.innerHTML = "";
  }, kind === "error" ? 6500 : 3200);
}
