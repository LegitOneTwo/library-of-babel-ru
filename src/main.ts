/** Точка входа: обработчики событий, горячие клавиши, загрузка. */

import "./styles.css";
import { api, runningInTauri, type ColorState } from "./api";
import { shortenAddress } from "./dom";
import { render, rememberFocus, updateArchiveView, updateColorFrame, updateSpeedLabel } from "./render";
import {
  clearError,
  setBusy,
  setError,
  setRenderer,
  state,
  type ViewName,
} from "./state";
import { showToast } from "./toast";
import { currentTheme, initTheme, toggleTheme } from "./theme";

/** Копирование в буфер обмена (Tauri-плагин, с откатом на navigator.clipboard). */
async function copyText(text: string): Promise<void> {
  if (runningInTauri) {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
    return;
  }
  await navigator.clipboard.writeText(text);
}

/** Диалог сохранения файла. */
async function pickSavePath(
  defaultName: string,
  kind: "text" | "image" = "text"
): Promise<string | null> {
  if (!runningInTauri) return null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  const result = await save({
    defaultPath: defaultName,
    filters:
      kind === "image"
        ? [{ name: "PNG", extensions: ["png"] }]
        : [{ name: "Текст", extensions: ["txt"] }],
  });
  return typeof result === "string" ? result : null;
}

/** Диалог открытия файла. */
async function pickOpenPath(
  kind: "text" | "image" = "text"
): Promise<string | null> {
  if (!runningInTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    multiple: false,
    directory: false,
    filters:
      kind === "image"
        ? [
            {
              name: "Изображения",
              extensions: ["png", "jpg", "jpeg", "bmp", "gif", "webp"],
            },
          ]
        : [{ name: "Текст", extensions: ["txt"] }],
  });
  return typeof result === "string" ? result : null;
}

function intValue(id: string, fallback: number): number {
  const el = document.getElementById(id) as HTMLInputElement | null;
  if (!el) return fallback;
  const n = parseInt(el.value, 10);
  return Number.isFinite(n) ? n : fallback;
}

function strValue(id: string, fallback: string): string {
  const el = document.getElementById(id) as HTMLInputElement | null;
  if (!el) return fallback;
  return el.value;
}

/** Сообщение об ошибке: панель в разделе плюс всплывашка. */
function fail(err: unknown): string {
  const message = err instanceof Error ? err.message : String(err);
  setError(message);
  showToast(message, "error");
  return message;
}

/** Переключение бокового меню на узких экранах. */
function setNavOpen(open: boolean): void {
  document.body.classList.toggle("nav-open", open);
  const toggle = document.getElementById("nav-toggle");
  if (toggle) toggle.setAttribute("aria-expanded", open ? "true" : "false");
  const scrim = document.getElementById("scrim");
  if (scrim) scrim.hidden = !open;
}

/** Открывает страницу по координатам. */
async function openAddress(
  hex: string,
  wall: number,
  shelf: number,
  volume: number,
  page: number
): Promise<void> {
  clearError();
  setBusy(true);
  try {
    const info = await api.openPage(hex, wall, shelf, volume, page);
    state.page = info;
    state.view = "reader";
    render();
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Загружает оглавление текущего тома. */
async function loadContents(): Promise<void> {
  if (!state.page) return;
  const a = state.page.address;
  clearError();
  setBusy(true);
  try {
    state.contents = await api.bookContents(a.hex, a.wall, a.shelf, a.volume);
  } catch (err) {
    fail(err);
  } finally {
    setBusy(false);
  }
}

async function refreshBookmarks(): Promise<void> {
  try {
    state.bookmarks = await api.listBookmarks();
  } catch {
    state.bookmarks = [];
  }
}

async function doRandom(): Promise<void> {
  clearError();
  setBusy(true);
  try {
    const info = await api.randomPage();
    state.page = info;
    state.view = "reader";
    render();
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

async function doSearch(): Promise<void> {
  const query = strValue("query-input", state.query);
  state.query = query;
  if (!query.trim()) {
    showToast("Введите текст для поиска", "error");
    return;
  }
  clearError();
  setBusy(true);
  try {
    const info = await api.searchPage(query);
    state.page = info;
    state.highlight = query.trim();
    state.view = "search";
    render();
    if (info.warnings.length) {
      showToast(info.warnings.join("; "), "info");
    } else {
      showToast(
        `Страница ${info.address.page} найдена: ${shortenAddress(
          info.address.full,
          12,
          8
        )}`,
        "ok"
      );
    }
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

async function doBookmark(): Promise<void> {
  if (!state.page) return;
  try {
    state.bookmarks = await api.addBookmark(
      state.page.address.hex,
      state.page.address.wall,
      state.page.address.shelf,
      state.page.address.volume,
      state.page.address.page
    );
    render();
    showToast("Страница добавлена в закладки", "ok");
  } catch (err) {
    fail(err);
  }
}

async function doSaveBook(): Promise<void> {
  if (!state.page) return;
  const a = state.page.address;
  // Короткое имя: полное имя гекса может занимать тысячи символов.
  const stem = await api.suggestFileName(a.hex, a.wall, a.shelf, a.volume, 1);
  const path = await pickSavePath(`${stem}.txt`);
  if (!path) return;
  setBusy(true);
  try {
    const saved = await api.saveBook(path, a.hex, a.wall, a.shelf, a.volume);
    showToast(`Том сохранён: ${saved}`, "ok");
  } catch (err) {
    fail(err);
  } finally {
    setBusy(false);
  }
}

async function doSavePage(): Promise<void> {
  if (!state.page) return;
  const a = state.page.address;
  const stem = await api.suggestFileName(a.hex, a.wall, a.shelf, a.volume, a.page);
  const path = await pickSavePath(`${stem}.txt`);
  if (!path) return;
  setBusy(true);
  try {
    const saved = await api.savePage(path, a.hex, a.wall, a.shelf, a.volume, a.page);
    showToast(`Страница сохранена: ${saved}`, "ok");
  } catch (err) {
    fail(err);
  } finally {
    setBusy(false);
  }
}

/** Открывает сохранённый файл и показывает соответствующую страницу. */
async function doOpenFile(): Promise<void> {
  const path = await pickOpenPath("text");
  if (!path) return;
  clearError();
  setBusy(true);
  try {
    const info = await api.openSavedFile(path);
    state.page = info;
    state.highlight = "";
    state.view = "reader";
    render();
    showToast(
      `Открыта страница ${info.address.page}: ${shortenAddress(
        info.address.full,
        12,
        8
      )}`,
      "ok"
    );
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Выбор файла изображения для поиска. */
async function doPickImage(): Promise<void> {
  const path = await pickOpenPath("image");
  if (!path) return;
  state.imagePath = path;
  state.imageMatch = null;
  state.view = "images";
  render();
  await doSearchImage();
}

/** Ищет выбранное изображение в библиотеке. */
async function doSearchImage(): Promise<void> {
  if (!state.imagePath) return;
  clearError();
  setBusy(true);
  try {
    const match = await api.searchImage(state.imagePath);
    state.imageMatch = match;
    state.view = "images";
    render();
    showToast(
      `Изображение найдено: страница ${match.address.page}, совпадение ${(
        match.similarity * 100
      ).toFixed(0)}%`,
      "ok"
    );
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Отрисовывает текущую страницу как цветную сетку. */
async function doRenderPageImage(): Promise<void> {
  if (!state.page) return;
  const a = state.page.address;
  clearError();
  setBusy(true);
  try {
    state.pageImage = await api.renderPageAsImage(
      a.hex,
      a.wall,
      a.shelf,
      a.volume,
      a.page,
      6
    );
    state.view = "images";
    render();
    showToast("Страница отрисована как изображение", "ok");
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Сохраняет текущую страницу как PNG. */
async function doSavePageImage(): Promise<void> {
  if (!state.page) return;
  const a = state.page.address;
  const stem = await api.suggestFileName(a.hex, a.wall, a.shelf, a.volume, a.page);
  const path = await pickSavePath(`${stem}.png`, "image");
  if (!path) return;
  setBusy(true);
  try {
    const saved = await api.savePageImage(
      path,
      a.hex,
      a.wall,
      a.shelf,
      a.volume,
      a.page,
      6
    );
    showToast(`PNG сохранён: ${saved}`, "ok");
  } catch (err) {
    fail(err);
  } finally {
    setBusy(false);
  }
}

/** Генерация случайного имени гекса (base-36, длина 1..16). */
function randomHex(): string {
  const alphabet = "0123456789abcdefghijklmnopqrstuvwxyz";
  const len = 1 + Math.floor(Math.random() * 16);
  let out = "";
  for (let i = 0; i < len; i += 1) {
    out += alphabet[Math.floor(Math.random() * alphabet.length)];
  }
  return out;
}

/** Загружает сведения об архиве (один раз). */
async function ensureArchiveInfo(): Promise<void> {
  if (state.archiveInfo) return;
  try {
    state.archiveInfo = await api.archiveInfo();
  } catch {
    state.archiveInfo = null;
  }
}

/** Случайное изображение архива. */
async function doArchiveRandom(): Promise<void> {
  clearError();
  setBusy(true);
  try {
    state.archive = await api.archiveRandom();
    state.view = "archive";
    await ensureArchiveInfo();
    render();
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Переход к соседней локации архива. */
async function doArchiveStep(forward: boolean): Promise<void> {
  const a = state.archive;
  if (!a) return;
  try {
    // Без setBusy: он вызывает полный render() и пересоздаёт кнопку «Стоп»
    // между mousedown и mouseup, клик не срабатывает.
    state.archive = await api.archiveStep(a.location, forward, 1);
    updateArchiveView();
  } catch (err) {
    fail(err);
  }
}

/** Вписывает выбранный файл в архив: картинка получает свою локацию. */
async function doArchiveSearch(): Promise<void> {
  if (!state.archivePath) return;
  const orientation = strValue(
    "archive-orientation",
    state.archiveOrientation
  ) as "auto" | "landscape" | "portrait";
  const fill = strValue("archive-fill", state.archiveFill) as "bars" | "stretch";
  const borderRaw = intValue("archive-border", state.archiveBorder);
  state.archiveOrientation = orientation;
  state.archiveFill = fill;
  state.archiveBorder = borderRaw;

  clearError();
  setBusy(true);
  try {
    state.archive = await api.archiveSearchImage(
      state.archivePath,
      orientation,
      fill,
      borderRaw
    );
    state.view = "archive";
    await ensureArchiveInfo();
    render();
    showToast("Картинка вписана в архив, у неё есть локация", "ok");
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Слайдшоу архива: перелистывает локации вперёд. */
let archiveTimer: number | undefined;

function toggleArchiveSlideshow(): void {
  const btn = document.getElementById("archive-play");
  if (state.archivePlaying) {
    window.clearInterval(archiveTimer);
    archiveTimer = undefined;
    state.archivePlaying = false;
    if (btn) btn.textContent = "Слайдшоу";
    return;
  }
  state.archivePlaying = true;
  if (btn) btn.textContent = "Стоп";
  archiveTimer = window.setInterval(() => {
    void doArchiveStep(true);
  }, 1200);
}

/** Загружает список режимов колоскопа (один раз). */
async function ensureColorModes(): Promise<void> {
  if (state.colorModes.length) return;
  try {
    state.colorModes = await api.coloroscopeModes();
  } catch {
    state.colorModes = [];
  }
}

/** Текущее состояние обхода в виде того, что ждёт бэкенд. */
function colorState(): ColorState | null {
  const c = state.color;
  if (!c) return null;
  return {
    mode: c.mode,
    r: c.r,
    g: c.g,
    b: c.b,
    counter: c.counter,
    step: c.step,
  };
}

/** Новый обход: с нуля или со случайного цвета. */
async function doColorNew(random: boolean): Promise<void> {
  clearError();
  setBusy(true);
  try {
    state.color = await api.coloroscopeNew(state.color?.mode ?? 0, random);
    state.colorPlaying = false;
    stopColorTimer();
    await ensureColorModes();
    render();
  } catch (err) {
    fail(err);
    render();
  } finally {
    setBusy(false);
  }
}

/** Продвинуть обход на N шагов. */
async function doColorStep(steps: number): Promise<void> {
  const s = colorState();
  if (!s) {
    await doColorNew(false);
    return;
  }
  try {
    state.color = await api.coloroscopeStep(s, steps, state.colorPlaying);
    // Точечное обновление, а не render(): полная перерисовка пересоздаёт
    // кнопку «Стоп» между нажатием и отпусканием, и клик не срабатывает.
    updateColorFrame();
  } catch (err) {
    fail(err);
  }
}

/** Переключить режим обхода. */
async function doColorSetMode(mode: number): Promise<void> {
  const s = colorState();
  if (!s) {
    state.color = await api.coloroscopeNew(mode, false);
    await ensureColorModes();
    render();
    return;
  }
  try {
    state.color = await api.coloroscopeSetMode(s, mode);
    render();
  } catch (err) {
    fail(err);
  }
}

/** Таймер обхода цветов. */
let colorTimer: number | undefined;

function stopColorTimer(): void {
  if (colorTimer !== undefined) {
    window.clearInterval(colorTimer);
    colorTimer = undefined;
  }
}

function toggleColorPlay(): void {
  const btn = document.getElementById("color-play");
  if (state.colorPlaying) {
    stopColorTimer();
    state.colorPlaying = false;
    if (btn) btn.textContent = "Пуск";
    return;
  }
  state.colorPlaying = true;
  if (btn) btn.textContent = "Стоп";
  colorTimer = window.setInterval(() => {
    void doColorStep(state.colorSpeed);
  }, 40);
}

async function handleAction(action: string): Promise<void> {
  switch (action) {
    case "random":
      await doRandom();
      break;
    case "copy-page":
      if (state.page) {
        await copyText(state.page.text);
        showToast("Страница скопирована в буфер обмена", "ok");
      }
      break;
    case "copy-address":
      if (state.page) {
        await copyText(state.page.address.full);
        showToast(
          `Адрес скопирован: ${shortenAddress(state.page.address.full, 12, 8)}`,
          "ok"
        );
      }
      break;
    case "save-book":
      await doSaveBook();
      break;
    case "save-page":
      await doSavePage();
      break;
    case "bookmark":
      await doBookmark();
      break;
    case "prev-page":
      if (state.page) {
        const p = Math.max(1, state.page.address.page - 1);
        await openAddress(
          state.page.address.hex,
          state.page.address.wall,
          state.page.address.shelf,
          state.page.address.volume,
          p
        );
      }
      break;
    case "next-page":
      if (state.page) {
        const p = Math.min(state.page.pages_in_book, state.page.address.page + 1);
        await openAddress(
          state.page.address.hex,
          state.page.address.wall,
          state.page.address.shelf,
          state.page.address.volume,
          p
        );
      }
      break;
    case "goto-page":
      if (state.page) {
        const p = intValue("page-input", state.page.address.page);
        await openAddress(
          state.page.address.hex,
          state.page.address.wall,
          state.page.address.shelf,
          state.page.address.volume,
          p
        );
      }
      break;
    case "browse-open": {
      const hex = strValue("hex-input", state.browseHex).trim();
      const wall = intValue("wall-input", 1);
      const shelf = intValue("shelf-input", 1);
      const volume = intValue("volume-input", 1);
      const page = intValue("browse-page-input", 1);
      state.browseHex = hex;
      state.browseWall = wall;
      state.browseShelf = shelf;
      state.browseVolume = volume;
      await openAddress(hex, wall, shelf, volume, page);
      break;
    }
    case "browse-random-hex": {
      const hex = randomHex();
      state.browseHex = hex;
      render();
      break;
    }
    case "browse-random-book": {
      const hex = randomHex();
      const wall = 1 + Math.floor(Math.random() * 4);
      const shelf = 1 + Math.floor(Math.random() * 5);
      const volume = 1 + Math.floor(Math.random() * 32);
      state.browseHex = hex;
      state.browseWall = wall;
      state.browseShelf = shelf;
      state.browseVolume = volume;
      await openAddress(hex, wall, shelf, volume, 1);
      break;
    }
    case "search-run":
      await doSearch();
      break;
    case "search-reset": {
      state.query = "";
      state.highlight = "";
      clearError();
      render();
      // Возвращаем фокус в поле запроса, чтобы можно было сразу ввести новый.
      const queryInput = document.getElementById("query-input") as HTMLInputElement | null;
      if (queryInput) {
        queryInput.focus();
        queryInput.setSelectionRange(0, 0);
      }
      break;
    }
    case "goto-read":
      state.view = "reader";
      render();
      break;
    case "open-file":
      await doOpenFile();
      break;
    case "pick-image":
      await doPickImage();
      break;
    case "search-image-run":
      await doSearchImage();
      break;
    case "render-page-image":
      await doRenderPageImage();
      break;
    case "save-page-image":
      await doSavePageImage();
      break;
    case "image-open-page": {
      const m = state.imageMatch;
      if (m) {
        await openAddress(
          m.address.hex,
          m.address.wall,
          m.address.shelf,
          m.address.volume,
          m.address.page
        );
      }
      break;
    }
    case "image-save-png": {
      const m = state.imageMatch;
      if (!m) break;
      const path = await pickSavePath(`${m.file_stem}.png`, "image");
      if (!path) break;
      setBusy(true);
      try {
        // Перерисовываем найденную страницу и сохраняем.
        const dataUrl = await api.renderPageAsImage(
          m.address.hex,
          m.address.wall,
          m.address.shelf,
          m.address.volume,
          m.address.page,
          6
        );
        const saved = await api.saveDataUrl(path, dataUrl);
        showToast(`PNG сохранён: ${saved}`, "ok");
      } catch (err) {
        fail(err);
      } finally {
        setBusy(false);
      }
      break;
    }
    case "open-site":
      await api.openExternal("https://libraryofbabel.info");
      break;
    case "theme-toggle": {
      // Тема меняется без полной перерисовки: интерфейс читает только
      // CSS-переменные, поэтому достаточно сменить атрибут на <html>.
      // Перерисовка нужна лишь для обновления надписи на кнопке.
      state.theme = toggleTheme();
      render();
      break;
    }
    case "archive-random":
      await doArchiveRandom();
      break;
    case "archive-next":
      await doArchiveStep(true);
      break;
    case "archive-prev":
      await doArchiveStep(false);
      break;
    case "archive-goto": {
      const loc = strValue("archive-location", "").trim();
      if (!loc) {
        showToast("Введите локацию", "error");
        break;
      }
      clearError();
      setBusy(true);
      try {
        state.archive = await api.archiveByLocation(loc);
        render();
        showToast("Изображение загружено", "ok");
      } catch (err) {
        fail(err);
        render();
      } finally {
        setBusy(false);
      }
      break;
    }
    case "archive-copy":
      if (state.archive) {
        await copyText(state.archive.location);
        showToast("Локация скопирована в буфер обмена", "ok");
      }
      break;
    case "archive-save": {
      const a = state.archive;
      if (!a) break;
      const path = await pickSavePath("babelia.png", "image");
      if (!path) break;
      setBusy(true);
      try {
        const saved = await api.archiveSavePng(path, a.location);
        showToast(`PNG сохранён: ${saved}`, "ok");
      } catch (err) {
        fail(err);
      } finally {
        setBusy(false);
      }
      break;
    }
    case "archive-play":
      toggleArchiveSlideshow();
      break;
    case "archive-pick-image": {
      const path = await pickOpenPath("image");
      if (!path) break;
      state.archivePath = path;
      state.view = "archive";
      render();
      await doArchiveSearch();
      break;
    }
    case "archive-search-run":
      await doArchiveSearch();
      break;
    case "color-new":
      await doColorNew(false);
      break;
    case "color-random":
      await doColorNew(true);
      break;
    case "color-step":
      await doColorStep(1);
      break;
    case "color-play":
      toggleColorPlay();
      break;
    case "color-copy":
      if (state.color) {
        await copyText(state.color.hex);
        showToast(`Цвет ${state.color.hex} скопирован`, "ok");
      }
      break;
    default:
      break;
  }
}

/** Переход в раздел: снимаем ошибку и закрываем мобильное меню. */
async function goToView(view: ViewName): Promise<void> {
  state.view = view;
  setNavOpen(false);
  if (view === "contents") {
    await loadContents();
    // loadContents ничего не рисует, если тома ещё нет, поэтому экран
    // перерисовываем здесь в любом случае.
  }
  render();
}

/** Глобальные обработчики. */
function bindEvents(): void {
  document.addEventListener(
    "click",
    async (event) => {
      const target = event.target as HTMLElement;

      const toggle = target.closest<HTMLElement>("#nav-toggle");
      if (toggle) {
        setNavOpen(!document.body.classList.contains("nav-open"));
        return;
      }

      const scrim = target.closest<HTMLElement>("#scrim");
      if (scrim) {
        setNavOpen(false);
        return;
      }

      const navBtn = target.closest<HTMLElement>("[data-view]");
      if (navBtn) {
        await goToView(navBtn.dataset.view as ViewName);
        return;
      }

      const actionEl = target.closest<HTMLElement>("[data-action]");
      if (actionEl) {
        const action = actionEl.dataset.action ?? "";
        if (action === "open-site") {
          event.preventDefault();
        }
        await handleAction(action);
        return;
      }

      const preset = target.closest<HTMLElement>("[data-preset]");
      if (preset) {
        const value = preset.dataset.preset ?? "";
        try {
          const info = await api.parseAddress(value);
          state.browseHex = info.hex;
          state.browseWall = info.wall;
          state.browseShelf = info.shelf;
          state.browseVolume = info.volume;
          render();
        } catch (err) {
          fail(err);
          render();
        }
        return;
      }

      const modeBtn = target.closest<HTMLElement>("[data-mode]");
      if (modeBtn) {
        const mode = parseInt(modeBtn.dataset.mode ?? "0", 10);
        await doColorSetMode(mode);
        return;
      }

      const contentsItem = target.closest<HTMLElement>("[data-page]");
      if (contentsItem && state.page) {
        const page = parseInt(contentsItem.dataset.page ?? "1", 10);
        await openAddress(
          state.page.address.hex,
          state.page.address.wall,
          state.page.address.shelf,
          state.page.address.volume,
          page
        );
        return;
      }

      const bmOpen = target.closest<HTMLElement>("[data-bm-open]");
      if (bmOpen) {
        const hash = bmOpen.dataset.bmOpen ?? "";
        const bm = state.bookmarks.find((b) => b.hash === hash);
        if (bm) {
          await openAddress(bm.hex, bm.wall, bm.shelf, bm.volume, bm.page);
        }
        return;
      }

      const bmDel = target.closest<HTMLElement>("[data-bm-del]");
      if (bmDel) {
        const hash = bmDel.dataset.bmDel ?? "";
        try {
          state.bookmarks = await api.removeBookmark(hash);
          render();
          showToast("Закладка удалена", "ok");
        } catch (err) {
          fail(err);
          render();
        }
      }
    },
    true
  );

  // Ввод в полях: сохраняем состояние без полной перерисовки.
  document.addEventListener("input", (event) => {
    const el = event.target as HTMLInputElement | HTMLTextAreaElement;
    if (el.id === "hex-input") state.browseHex = el.value;
    if (el.id === "query-input") state.query = el.value;
    if (el.id === "wall-input") state.browseWall = parseInt(el.value, 10) || 1;
    if (el.id === "shelf-input") state.browseShelf = parseInt(el.value, 10) || 1;
    if (el.id === "volume-input") state.browseVolume = parseInt(el.value, 10) || 1;
    if (el.id === "color-speed") {
      state.colorSpeed = parseInt(el.value, 10) || 1;
      // Подпись обновляем на месте: полная перерисовка во время обхода
      // пересоздаёт кнопку «Стоп» и ломает её нажатие.
      updateSpeedLabel();
    }
  });

  document.addEventListener("keydown", async (event) => {
    const el = event.target as HTMLElement;
    const typing =
      el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.isContentEditable;

    if (event.key === "Escape") {
      if (typing) {
        (el as HTMLInputElement).blur();
        return;
      }
      if (document.body.classList.contains("nav-open")) {
        setNavOpen(false);
        return;
      }
      if (state.colorPlaying) {
        toggleColorPlay();
        return;
      }
      if (state.archivePlaying) {
        toggleArchiveSlideshow();
      }
      return;
    }

    if (event.ctrlKey && event.key === "Enter") {
      event.preventDefault();
      if (state.view === "search") await doSearch();
      else if (state.view === "browse") await handleAction("browse-open");
      return;
    }

    if (typing) return;

    if (event.ctrlKey) {
      const map: Record<string, ViewName> = {
        "1": "reader",
        "2": "browse",
        "3": "search",
        "4": "images",
        "5": "archive",
        "6": "coloroscope",
        "7": "contents",
        "8": "bookmarks",
        "9": "about",
      };
      const view = map[event.key];
      if (view) {
        event.preventDefault();
        await goToView(view);
        return;
      }
      if (event.key.toLowerCase() === "r") {
        event.preventDefault();
        await doRandom();
      }
    }
  });

  // Сохраняем позицию курсора перед перерисовкой.
  document.addEventListener("focusin", rememberFocus);
}

async function boot(): Promise<void> {
  // Тема применяется до первой отрисовки, иначе интерфейс мелькнёт тёмным.
  initTheme();
  // state.theme читается при создании состояния, то есть до initTheme(),
  // поэтому после применения сохранённой темы синхронизируем его.
  state.theme = currentTheme();
  setRenderer(render);
  bindEvents();
  if (!runningInTauri) {
    showToast(
      "Интерфейс открыт в браузере, команды Tauri недоступны. Запустите `npm run tauri dev`.",
      "info"
    );
    render();
    return;
  }
  try {
    state.info = await api.appInfo();
  } catch (err) {
    fail(err);
  }
  await refreshBookmarks();
  await ensureArchiveInfo();
  await ensureColorModes();
  render();
  await doRandom();
}

boot().catch((err) => {
  fail(err);
  render();
});
