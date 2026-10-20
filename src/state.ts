/** Состояние приложения и общие типы. */

import type {
  AddressInfo,
  AppInfo,
  ArchiveInfo,
  ArchiveView,
  Bookmark,
  ColorFrame,
  ColorMode,
  ImageMatch,
  PageInfo,
} from "./api";
import { currentTheme, type Theme } from "./theme";

export type ViewName =
  | "reader"
  | "browse"
  | "search"
  | "images"
  | "archive"
  | "coloroscope"
  | "contents"
  | "bookmarks"
  | "about";

export interface AppState {
  view: ViewName;
  info: AppInfo | null;
  page: PageInfo | null;
  contents: { page: number; title: string }[];
  bookmarks: Bookmark[];
  query: string;
  browseHex: string;
  browseWall: number;
  browseShelf: number;
  browseVolume: number;
  highlight: string;
  busy: boolean;
  /** Последняя ошибка раздела: показывается панелью над содержимым. */
  error: string;
  /** Результат поиска по изображению. */
  imageMatch: ImageMatch | null;
  /** Путь к выбранному файлу изображения. */
  imagePath: string;
  /** PNG текущей страницы, отрисованной как сетка цветов. */
  pageImage: string;
  /** Текущее изображение архива изображений. */
  archive: ArchiveView | null;
  /** Сведения об архиве. */
  archiveInfo: ArchiveInfo | null;
  /** Файл, выбранный для поиска в архиве. */
  archivePath: string;
  /** Ориентация при поиске в архиве. */
  archiveOrientation: "auto" | "landscape" | "portrait";
  /** Заполнение при поиске в архиве. */
  archiveFill: "bars" | "stretch";
  /** Цвет рамки при поиске в архиве. */
  archiveBorder: number;
  /** Автопрокрутка слайдшоу архива. */
  archivePlaying: boolean;
  /** Текущий кадр колоскопа. */
  color: ColorFrame | null;
  /** Режимы колоскопа. */
  colorModes: ColorMode[];
  /** Идёт ли обход цветов. */
  colorPlaying: boolean;
  /** Скорость обхода: шагов за тик. */
  colorSpeed: number;
  /** Выбранная тема оформления. */
  theme: Theme;
}

export const state: AppState = {
  view: "reader",
  info: null,
  page: null,
  contents: [],
  bookmarks: [],
  query: "",
  browseHex: "jeb0110jlb",
  browseWall: 1,
  browseShelf: 1,
  browseVolume: 1,
  highlight: "",
  busy: false,
  error: "",
  imageMatch: null,
  imagePath: "",
  pageImage: "",
  archive: null,
  archiveInfo: null,
  archivePath: "",
  archiveOrientation: "auto",
  archiveFill: "bars",
  archiveBorder: 0x222,
  archivePlaying: false,
  color: null,
  colorModes: [],
  colorPlaying: false,
  colorSpeed: 1,
  theme: currentTheme(),
};

// Отрисовка регистрируется извне, чтобы state.ts не зависел от render.ts
// (иначе получится циклический импорт).
let renderFn: (() => void) | null = null;

export function setRenderer(fn: () => void): void {
  renderFn = fn;
}

function redraw(): void {
  if (renderFn) renderFn();
}

export function setBusy(value: boolean): void {
  state.busy = value;
  redraw();
}

/** Показать ошибку в текущем разделе. */
export function setError(message: string): void {
  state.error = message;
}

/** Снять ошибку раздела. */
export function clearError(): void {
  state.error = "";
}

/** Адрес текущей открытой страницы (или null). */
export function currentAddress(): AddressInfo | null {
  return state.page ? state.page.address : null;
}
