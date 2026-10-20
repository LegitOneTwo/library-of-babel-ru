/** Типы и обёртки над Tauri-командами. */

import { invoke, isTauri } from "@tauri-apps/api/core";

export interface AddressInfo {
  hex: string;
  wall: number;
  shelf: number;
  volume: number;
  page: number;
  canonical: string;
  full: string;
  hash: string;
}

export interface PageInfo {
  text: string;
  lines: string[];
  title: string;
  address: AddressInfo;
  pages_in_book: number;
  chars_per_line: number;
  lines_per_page: number;
  warnings: string[];
}

export interface ContentsEntry {
  page: number;
  title: string;
}

export interface Bookmark {
  hash: string;
  hex: string;
  wall: number;
  shelf: number;
  volume: number;
  page: number;
  canonical: string;
  full: string;
  saved_at: number;
}

export interface AppInfo {
  name: string;
  version: string;
  alphabet: string;
  alphabet_size: number;
  page_symbols: number;
  lines_per_page: number;
  chars_per_line: number;
  pages_per_book: number;
  walls: number;
  shelves: number;
  volumes: number;
}

export interface ImageMatch {
  address: AddressInfo;
  /** Квантованное изображение в 64 цветах (data URL). */
  quantized_png: string;
  /** Миниатюра исходника (data URL). */
  source_png: string;
  /** Насколько исходник совпал с квантованной версией, 0..1. */
  similarity: number;
  /** Страница как текст (3200 символов). */
  text: string;
  /** Короткое имя файла. */
  file_stem: string;
}

export type OrientationArg = "auto" | "landscape" | "portrait";
export type FillArg = "bars" | "stretch";

/** Состояние обхода цветов. */
export interface ColorState {
  mode: number;
  r: number;
  g: number;
  b: number;
  counter: number;
  step: number;
}

/** Кадр «колоскопа»: текущий цвет и положение в обходе. */
export interface ColorFrame {
  hex: string;
  rgb: string;
  mode: number;
  mode_name: string;
  r: number;
  g: number;
  b: number;
  /** Фаза обхода: 0 значит быстрый канал растёт, 1 падает. */
  counter: number;
  /** Полное число шагов с начала. */
  step: number;
  step_in_cycle: number;
  cycle_steps: number;
  distinct_colors: number;
  playing: boolean;
}

/** Режим обхода: какой канал меняется быстрее всех. */
export interface ColorMode {
  index: number;
  name: string;
  /** Названия каналов: быстрый, средний, медленный. */
  channels: [string, string, string];
}

/** Изображение из архива изображений (сетка 640×416, 4096 цветов). */
export interface ArchiveView {
  /** Полная локация (десятичная строка, ~961 755 цифр). */
  location: string;
  /** Сокращённая локация для показа. */
  short: string;
  /** PNG в виде data URL. */
  png: string;
  width: number;
  height: number;
  transposed: boolean;
}

export interface ArchiveInfo {
  width: number;
  height: number;
  pixels: number;
  colors: number;
  location_digits: number;
  total_images: string;
  distinct_pixels: string;
  /** Цвета полей для вписывания картинки: [подпись, индекс палитры]. */
  border_colors: [string, number][];
}

/** Работает ли приложение внутри Tauri (а не в обычном браузере). */
export const runningInTauri = isTauri();

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (!runningInTauri) {
    throw new Error(
      "Команда доступна только в настольном приложении (Tauri). Запустите `npm run tauri dev`."
    );
  }
  return invoke<T>(cmd, args);
}

export const api = {
  openPage: (hex: string, wall: number, shelf: number, volume: number, page: number) =>
    call<PageInfo>("open_page", { hex, wall, shelf, volume, page }),
  searchPage: (text: string) => call<PageInfo>("search_page", { text }),
  randomPage: () => call<PageInfo>("random_page"),
  bookContents: (hex: string, wall: number, shelf: number, volume: number) =>
    call<ContentsEntry[]>("book_contents", { hex, wall, shelf, volume }),
  saveBook: (
    path: string,
    hex: string,
    wall: number,
    shelf: number,
    volume: number
  ) => call<string>("save_book", { path, hex, wall, shelf, volume }),
  savePage: (
    path: string,
    hex: string,
    wall: number,
    shelf: number,
    volume: number,
    page: number
  ) => call<string>("save_page", { path, hex, wall, shelf, volume, page }),
  /** Короткое имя файла для адреса (без гигантского имени гекса). */
  suggestFileName: (
    hex: string,
    wall: number,
    shelf: number,
    volume: number,
    page: number
  ) => call<string>("suggest_file_name", { hex, wall, shelf, volume, page }),
  /** Открывает сохранённый файл и возвращает страницу с её адресом. */
  openSavedFile: (path: string) => call<PageInfo>("open_saved_file", { path }),
  listBookmarks: () => call<Bookmark[]>("list_bookmarks"),
  addBookmark: (hex: string, wall: number, shelf: number, volume: number, page: number) =>
    call<Bookmark[]>("add_bookmark", { hex, wall, shelf, volume, page }),
  removeBookmark: (hash: string) => call<Bookmark[]>("remove_bookmark", { hash }),
  parseAddress: (input: string) => call<AddressInfo>("parse_address", { input }),
  addressFromText: (text: string, page: number) =>
    call<AddressInfo>("address_from_text", { text, page }),
  appInfo: () => call<AppInfo>("app_info"),
  openExternal: (url: string) => call<void>("open_external", { url }),

  // Изображения
  searchImage: (path: string) => call<ImageMatch>("search_image", { path }),
  renderPageAsImage: (
    hex: string,
    wall: number,
    shelf: number,
    volume: number,
    page: number,
    scale: number
  ) => call<string>("render_page_as_image", { hex, wall, shelf, volume, page, scale }),
  savePageImage: (
    path: string,
    hex: string,
    wall: number,
    shelf: number,
    volume: number,
    page: number,
    scale: number
  ) =>
    call<string>("save_page_image", {
      path,
      hex,
      wall,
      shelf,
      volume,
      page,
      scale,
    }),
  /** Сохраняет data URL (base64) в файл. */
  saveDataUrl: (path: string, dataUrl: string) => call<string>("save_data_url", { path, dataUrl }),

  // Архив изображений (640×416, 4096 цветов)
  archiveRandom: () => call<ArchiveView>("archive_random"),
  archiveByLocation: (location: string) => call<ArchiveView>("archive_by_location", { location }),
  archiveStep: (location: string, forward: boolean, step: number) =>
    call<ArchiveView>("archive_step", { location, forward, step }),
  archiveSearchImage: (
    path: string,
    orientation: OrientationArg,
    fill: FillArg,
    border: number
  ) =>
    call<ArchiveView>("archive_search_image", {
      path,
      orientation,
      fill,
      border,
    }),
  archiveSavePng: (path: string, location: string) =>
    call<string>("archive_save_png", { path, location }),
  archiveInfo: () => call<ArchiveInfo>("archive_info"),

  // Колоскоп, перестановщик цветов
  coloroscopeNew: (mode?: number, random?: boolean) =>
    call<ColorFrame>("coloroscope_new", { mode: mode ?? null, random: random ?? null }),
  coloroscopeStep: (state: ColorState, steps: number, playing?: boolean) =>
    call<ColorFrame>("coloroscope_step", { state, steps, playing: playing ?? null }),
  coloroscopeSetMode: (state: ColorState, mode: number) =>
    call<ColorFrame>("coloroscope_set_mode", { state, mode }),
  coloroscopeModes: () => call<ColorMode[]>("coloroscope_modes"),
};
