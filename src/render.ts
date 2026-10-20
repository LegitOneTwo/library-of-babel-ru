/**
 * Отрисовка интерфейса.
 *
 * Правила оформления живут в DESIGN.md, фильтр качества в antislop.
 * Здесь только разметка: каждый раздел собран под свою задачу, а не под общий шаблон карточек.
 */

import { type PageInfo } from "./api";
import { esc, formatDate, highlight, shortenAddress } from "./dom";
import { HEX_MARK } from "./hexmark";
import { state, type ViewName } from "./state";

const app = () => document.getElementById("app") as HTMLElement;

const VIEW_TITLES: Record<ViewName, string> = {
  reader: "Чтение",
  browse: "Обзор галереи",
  search: "Поиск по библиотеке",
  images: "Изображения",
  archive: "Архив изображений",
  coloroscope: "Колоскоп",
  contents: "Оглавление тома",
  bookmarks: "Закладки",
  about: "О библиотеке",
};

interface ActionItem {
  action: string;
  label: string;
  hint?: string;
  needsPage?: boolean;
}

interface NavItem {
  id: ViewName;
  label: string;
  hint: string;
  /** Команды, которые относятся именно к этому разделу. */
  commands?: ActionItem[];
}

interface NavGroup {
  label: string;
  items: NavItem[];
}

/**
 * Навигация по разделам.
 *
 * Отдельной категории «Действия» нет: команда лежит внутри того раздела,
 * с которым она связана, поэтому её место видно сразу, без догадок.
 * Команды нарисованы тише разделов: раздел это место, команда это то,
 * что можно сделать в этом месте.
 */
const NAV_GROUPS: NavGroup[] = [
  {
    label: "Текст",
    items: [
      {
        id: "reader",
        label: "Читать",
        hint: "Ctrl+1",
        commands: [
          { action: "open-file", label: "Открыть файл" },
          { action: "copy-page", label: "Копировать текст", needsPage: true },
          { action: "save-page", label: "Сохранить страницу", needsPage: true },
        ],
      },
      { id: "browse", label: "Обзор", hint: "Ctrl+2" },
      { id: "search", label: "Поиск", hint: "Ctrl+3" },
      {
        id: "contents",
        label: "Оглавление",
        hint: "Ctrl+7",
        commands: [{ action: "save-book", label: "Сохранить том", needsPage: true }],
      },
      { id: "bookmarks", label: "Закладки", hint: "Ctrl+8" },
      { id: "about", label: "О библиотеке", hint: "Ctrl+9" },
    ],
  },
  {
    label: "Изображения",
    items: [
      {
        id: "images",
        label: "Страница как картинка",
        hint: "Ctrl+4",
        commands: [
          { action: "save-page-image", label: "Сохранить как PNG", needsPage: true },
        ],
      },
      { id: "archive", label: "Архив картинок", hint: "Ctrl+5" },
      { id: "coloroscope", label: "Колоскоп", hint: "Ctrl+6" },
    ],
  },
];

function navButtons(items: NavItem[]): string {
  return items
    .map((n) => {
      const commands = n.commands?.length
        ? `<div class="nav-commands">${actionButtons(n.commands)}</div>`
        : "";
      return `
      <button class="nav-btn ${state.view === n.id ? "active" : ""}" data-view="${n.id}"
        ${state.view === n.id ? 'aria-current="page"' : ""}>
        <span class="nav-text">${esc(n.label)}</span>
        <span class="kbd-hint">${esc(n.hint)}</span>
      </button>${commands}`;
    })
    .join("");
}

function actionButtons(items: ActionItem[]): string {
  return items
    .map((a) => {
      const off = Boolean(a.needsPage) && !state.page;
      return `
      <button class="nav-btn nav-action" data-action="${a.action}" ${off ? "disabled" : ""}
        ${off ? 'title="Сначала откройте страницу"' : ""}>
        <span class="nav-text">${esc(a.label)}</span>
        ${a.hint ? `<span class="kbd-hint">${esc(a.hint)}</span>` : ""}
      </button>`;
    })
    .join("");
}

function sidebar(): string {
  const groups = NAV_GROUPS.map(
    (g) => `
      <div class="nav-group">
        <div class="nav-label">${esc(g.label)}</div>
        ${navButtons(g.items)}
      </div>`
  ).join("");

  const info = state.info;

  return `
    <aside class="sidebar" id="sidebar">
      <div class="brand">
        <h1>${HEX_MARK}<span>Вавилонская Библиотека</span></h1>
        <p class="brand-note">Каждая комбинация символов вычисляется на лету. На диске нет ни одной страницы.</p>
      </div>
      <nav class="side-nav" aria-label="Разделы библиотеки">
        ${groups}
      </nav>
      <div class="side-foot">
        <dl class="spec">
          <div><dt>алфавит</dt><dd>${info ? `${info.alphabet_size} символа` : "загрузка"}</dd></div>
          <div><dt>страница</dt><dd>${info ? `${info.page_symbols} символов` : "загрузка"}</dd></div>
          <div><dt>том</dt><dd>${info ? `${info.pages_per_book} страниц` : "загрузка"}</dd></div>
        </dl>
      </div>
    </aside>`;
}

function topbar(): string {
  const addr = state.page ? state.page.address : null;
  return `
    <header class="topbar">
      <button class="nav-toggle" id="nav-toggle" aria-expanded="false" aria-controls="sidebar">
        <span class="nav-toggle-bars" aria-hidden="true"></span>
        <span class="nav-toggle-text">Разделы</span>
      </button>
      <h2 class="topbar-title">${esc(VIEW_TITLES[state.view])}</h2>
      ${
        addr
          ? `<span class="addr mono" title="${esc(addr.full)}">${esc(
              shortenAddress(addr.full, 12, 8)
            )}</span>`
          : ""
      }
      <div class="spacer"></div>
      ${
        state.busy
          ? `<span class="spinner" role="status" aria-label="Идёт вычисление"></span>`
          : ""
      }
      <button class="btn btn-quiet theme-toggle" id="theme-toggle" data-action="theme-toggle"
        aria-pressed="${state.theme === "light"}"
        title="${state.theme === "light" ? "Включить тёмную тему" : "Включить светлую тему"}">
        ${state.theme === "light" ? "Тёмная" : "Светлая"}
      </button>
      <button class="btn btn-primary" data-action="random">Случайная страница</button>
    </header>`;
}

/** Метка координат: повторяемый голос продукта. */
function coords(p: PageInfo): string {
  return `стена ${p.address.wall} · полка ${p.address.shelf} · том ${p.address.volume} · страница ${p.address.page} из ${p.pages_in_book}`;
}

function readerView(): string {
  if (!state.page) {
    return `
    <section class="sheet sheet-empty">
      <p class="sheet-lead">Страница не открыта</p>
      <p class="sheet-note">Возьмите случайную страницу или найдите нужный текст поиском.</p>
      <div class="btn-row">
        <button class="btn btn-primary" data-action="random">Случайная страница</button>
        <button class="btn" data-view="search">Перейти к поиску</button>
      </div>
    </section>`;
  }
  const p: PageInfo = state.page;
  const lines = p.lines
    .map((line) => `<span class="line">${highlight(line, state.highlight)}</span>`)
    .join("");
  const marked = state.bookmarks.some((b) => b.hash === p.address.hash);
  return `
    <article class="sheet">
      <div class="sheet-rail">
        <div class="sheet-rail-main">
          <p class="sheet-title">${esc(p.title)}</p>
          <p class="coords mono" title="${esc(p.address.full)}">${esc(coords(p))}</p>
        </div>
        <div class="sheet-rail-side">
          <button class="btn btn-quiet" data-action="copy-page">Копировать текст</button>
          <button class="btn btn-quiet" data-action="save-page">Сохранить</button>
          ${
            state.highlight
              ? `<button class="btn btn-quiet" data-action="search-reset">Снять подсветку</button>`
              : ""
          }
          <button class="btn btn-quiet" data-action="render-page-image">Показать как картинку</button>
        </div>
      </div>
      <div class="sheet-body" tabindex="0" role="document" aria-label="Текст страницы">${lines}</div>
      <div class="sheet-rail sheet-rail-foot">
        <div class="pager">
          <button class="btn btn-quiet" data-action="prev-page" title="Предыдущая страница">‹</button>
          <input id="page-input" type="number" min="1" max="${p.pages_in_book}" value="${
            p.address.page
          }" aria-label="Номер страницы" />
          <span class="of">из ${p.pages_in_book}</span>
          <button class="btn btn-quiet" data-action="next-page" title="Следующая страница">›</button>
          <button class="btn" data-action="goto-page">Перейти</button>
        </div>
        <div class="spacer"></div>
        <button class="btn" data-action="bookmark">${marked ? "В закладках" : "В закладки"}</button>
        <button class="btn btn-quiet" data-action="copy-address">Копировать адрес</button>
      </div>
    </article>`;
}

function browseView(): string {
  return `
    <div class="split">
      <section class="well">
        <h3 class="well-title">Адрес в библиотеке</h3>
        <div class="field">
          <label for="hex-input">Имя гекса, цифры и a-z</label>
          <input id="hex-input" class="input mono" type="text" value="${esc(
            state.browseHex
          )}" spellcheck="false" />
        </div>
        <div class="coord-row">
          <div class="field">
            <label for="wall-input">Стена, 1-4</label>
            <input id="wall-input" class="input" type="number" min="1" max="4" value="${
              state.browseWall
            }" />
          </div>
          <div class="field">
            <label for="shelf-input">Полка, 1-5</label>
            <input id="shelf-input" class="input" type="number" min="1" max="5" value="${
              state.browseShelf
            }" />
          </div>
          <div class="field">
            <label for="volume-input">Том, 1-32</label>
            <input id="volume-input" class="input" type="number" min="1" max="32" value="${
              state.browseVolume
            }" />
          </div>
          <div class="field">
            <label for="browse-page-input">Страница, 1-410</label>
            <input id="browse-page-input" class="input" type="number" min="1" max="410" value="1" />
          </div>
        </div>
        <div class="btn-row">
          <button class="btn btn-primary" data-action="browse-open">Открыть страницу</button>
          <button class="btn" data-action="browse-random-hex">Случайный гекс</button>
          <button class="btn" data-action="browse-random-book">Случайный том</button>
        </div>
      </section>
      <section class="well">
        <h3 class="well-title">Устройство адреса</h3>
        <p class="prose">
          Имя гекса кодируется в base-36. Внутри гекса 4 стены, по 5 полок, по 32 тома.
          В томе 410 страниц по 3200 символов.
        </p>
        <h3 class="well-title">Быстрые адреса</h3>
        <div class="btn-row">
          <button class="btn btn-quiet mono" data-preset="jeb0110jlb-w2-s4-v16">jeb0110jlb-w2-s4-v16</button>
          <button class="btn btn-quiet mono" data-preset="a-w1-s1-v1">a-w1-s1-v1</button>
          <button class="btn btn-quiet mono" data-preset="0-w1-s1-v1">0-w1-s1-v1</button>
          <button class="btn btn-quiet mono" data-preset="zzz-w4-s5-v32">zzz-w4-s5-v32</button>
        </div>
      </section>
    </div>`;
}

function searchView(): string {
  return `
    <div class="narrow">
      <section class="well">
        <h3 class="well-title">Найти страницу с текстом</h3>
        <div class="field">
          <label for="query-input">Текст, до 3200 символов</label>
          <textarea id="query-input" class="input" rows="5" placeholder="например: привет мир" spellcheck="false">${esc(
            state.query
          )}</textarea>
        </div>
        <div class="btn-row">
          <button class="btn btn-primary" data-action="search-run">Найти страницу</button>
          <button class="btn" data-action="search-reset">Очистить</button>
        </div>
        <p class="prose">
          Текст встраивается в сгенерированную страницу, а адрес восстанавливается
          обратным преобразованием. Страница всегда содержит ваш текст и всегда
          находится на одном месте.
        </p>
      </section>
      ${
        state.page && state.highlight
          ? `<section class="well">
              <h3 class="well-title">Страница найдена</h3>
              <p class="prose">Страница ${state.page.address.page}, адрес
                <span class="mono" title="${esc(state.page.address.full)}">${esc(
                  shortenAddress(state.page.address.full, 16, 12)
                )}</span>
              </p>
              <div class="btn-row">
                <button class="btn btn-primary" data-action="goto-read">Открыть в читалке</button>
              </div>
            </section>`
          : ""
      }
    </div>`;
}

function imagesView(): string {
  const m = state.imageMatch;
  return `
    <div class="split">
      <section class="well">
        <h3 class="well-title">Картинка в библиотеке</h3>
        <p class="prose">
          Страница это сетка 80 на 40 ячеек, ровно 3200 символов. Картинка сжимается
          до такой сетки и квантуется в 64 цвета алфавита, а страница находится
          обратным преобразованием. Картинка остаётся в библиотеке навсегда, по
          тому же закону, что и текст.
        </p>
        <div class="btn-row">
          <button class="btn btn-primary" data-action="pick-image">Выбрать изображение</button>
          ${
            state.imagePath
              ? `<button class="btn" data-action="search-image-run">Найти в библиотеке</button>`
              : ""
          }
        </div>
        ${
          state.imagePath
            ? `<p class="prose file-line">Файл: <span class="mono">${esc(
                state.imagePath
              )}</span></p>`
            : ""
        }
        ${
          state.busy && !m
            ? `<p class="state-line" role="status">Ищем страницу для картинки</p>`
            : ""
        }
      </section>
      ${
        m
          ? `<section class="well">
              <h3 class="well-title">Результат</h3>
              <div class="img-compare">
                <figure>
                  <img src="${m.source_png}" alt="Исходное изображение" />
                  <figcaption>Исходник</figcaption>
                </figure>
                <figure>
                  <img src="${m.quantized_png}" alt="Страница библиотеки" />
                  <figcaption>Страница ${esc(shortenAddress(m.address.canonical, 16, 12))}</figcaption>
                </figure>
              </div>
              <dl class="spec spec-wide">
                <div><dt>совпадение с оригиналом</dt><dd>${(m.similarity * 100).toFixed(
                  1
                )}%</dd></div>
                <div><dt>адрес страницы</dt><dd class="mono" title="${esc(
                  m.address.full
                )}">${esc(shortenAddress(m.address.full, 16, 12))}</dd></div>
              </dl>
              <div class="btn-row">
                <button class="btn btn-primary" data-action="image-open-page">Открыть страницу</button>
                <button class="btn" data-action="image-save-png">Сохранить PNG</button>
              </div>
            </section>`
          : ""
      }
    </div>
    ${
      state.pageImage
        ? `<section class="well">
            <h3 class="well-title">Текущая страница как изображение</h3>
            <figure class="plate-figure">
              <img src="${state.pageImage}" alt="Страница как изображение" />
              <figcaption>Сетка 80 на 40, 64 цвета алфавита</figcaption>
            </figure>
            <div class="btn-row">
              <button class="btn" data-action="render-page-image">Перерисовать</button>
              <button class="btn" data-action="save-page-image">Сохранить PNG</button>
            </div>
          </section>`
        : ""
    }`;
}

function archiveView(): string {
  const a = state.archive;
  const info = state.archiveInfo;
  return `
    <div class="archive-layout">
      <section class="well">
        <h3 class="well-title">Архив изображений</h3>
        ${
          info
            ? `<p class="prose">
                Здесь вместо букв перебираются цвета. Изображение это сетка ${info.width} на ${info.height},
                каждый пиксель это 12 бит, всего ${info.colors.toLocaleString(
                  "ru-RU"
                )} цветов. Локация это целое число из ${info.location_digits.toLocaleString(
                  "ru-RU"
                )} десятичных цифр, а картинка получается умножением локации на нечётную константу.
              </p>`
            : `<p class="prose">Считаем параметры архива.</p>`
        }
        <div class="btn-row">
          <button class="btn btn-primary" data-action="archive-random">Случайное изображение</button>
          <button class="btn" data-action="archive-pick-image">Вписать свою картинку</button>
        </div>
        ${
          state.archivePath
            ? `<p class="prose file-line">Файл: <span class="mono">${esc(
                state.archivePath
              )}</span></p>
               <div class="coord-row coord-row-3">
                 <div class="field">
                   <label for="archive-orientation">Ориентация</label>
                   <select id="archive-orientation" class="input">
                     <option value="auto" ${
                       state.archiveOrientation === "auto" ? "selected" : ""
                     }>Авто</option>
                     <option value="landscape" ${
                       state.archiveOrientation === "landscape" ? "selected" : ""
                     }>Альбомная</option>
                     <option value="portrait" ${
                       state.archiveOrientation === "portrait" ? "selected" : ""
                     }>Книжная</option>
                   </select>
                 </div>
                 <div class="field">
                   <label for="archive-fill">Вписывание</label>
                   <select id="archive-fill" class="input">
                     <option value="bars" ${
                       state.archiveFill === "bars" ? "selected" : ""
                     }>С полями</option>
                     <option value="stretch" ${
                       state.archiveFill === "stretch" ? "selected" : ""
                     }>Растянуть</option>
                   </select>
                 </div>
                 <div class="field">
                   <label for="archive-border">Цвет полей</label>
                   <select id="archive-border" class="input">
                     ${archiveBorderOptions()}
                   </select>
                 </div>
               </div>
               <div class="btn-row">
                 <button class="btn btn-primary" data-action="archive-search-run">Вписать в архив</button>
               </div>`
            : ""
        }
      </section>
      ${
        a
          ? `<section class="well">
              <h3 class="well-title">Изображение</h3>
              <figure class="plate-figure archive-figure">
                <img src="${a.png}" alt="Изображение архива" id="archive-image" />
                <figcaption>Локация № ${esc(a.short)}</figcaption>
              </figure>
              <div class="btn-row">
                <button class="btn btn-quiet" data-action="archive-prev" title="Предыдущая локация">‹</button>
                <button class="btn btn-quiet" data-action="archive-next" title="Следующая локация">›</button>
                <button class="btn" data-action="archive-play" id="archive-play">${
                  state.archivePlaying ? "Стоп" : "Слайдшоу"
                }</button>
                <button class="btn" data-action="archive-copy">Копировать локацию</button>
                <button class="btn" data-action="archive-save">Сохранить PNG</button>
              </div>
              <div class="field">
                <label for="archive-location">Локация, только цифры</label>
                <textarea id="archive-location" class="input mono" rows="3" spellcheck="false">${esc(
                  a.location
                )}</textarea>
              </div>
              <div class="btn-row">
                <button class="btn btn-primary" data-action="archive-goto">Перейти к локации</button>
              </div>
            </section>`
          : `<section class="well well-empty">
              <p class="sheet-lead">Архив пуст на экране</p>
              <p class="sheet-note">Возьмите случайное изображение или впишите свою картинку.</p>
            </section>`
      }
    </div>
    ${
      info
        ? `<section class="well">
            <h3 class="well-title">Параметры архива</h3>
            <dl class="spec spec-grid">
              <div><dt>сетка</dt><dd>${info.width} на ${info.height}</dd></div>
              <div><dt>пикселей</dt><dd>${info.pixels.toLocaleString("ru-RU")}</dd></div>
              <div><dt>цветов</dt><dd>${info.colors.toLocaleString("ru-RU")}</dd></div>
              <div><dt>бит на пиксель</dt><dd>12</dd></div>
              <div><dt>всего бит</dt><dd>${(info.pixels * 12).toLocaleString("ru-RU")}</dd></div>
              <div><dt>цифр в локации</dt><dd>${info.location_digits.toLocaleString("ru-RU")}</dd></div>
              <div><dt>изображений</dt><dd>${esc(info.total_images)}</dd></div>
            </dl>
          </section>`
        : ""
    }`;
}

/**
 * Опции цвета полей (12 бит: r4 g4 b4).
 *
 * Список приходит из движка (`archive_info.border_colors`): индексы палитры
 * не дублируются на стороне интерфейса.
 */
function archiveBorderOptions(): string {
  const options = state.archiveInfo?.border_colors ?? [];
  if (!options.length) return "";
  return options
    .map(
      ([label, value]) =>
        `<option value="${value}" ${
          state.archiveBorder === value ? "selected" : ""
        }>${esc(label)}</option>`
    )
    .join("");
}

/**
 * Подпись скорости обхода: «шаг за тик», «шага за тик», «шагов за тик».
 */
function speedLabel(steps: number): string {
  const n = Math.abs(steps) % 100;
  const last = n % 10;
  if (n > 10 && n < 20) return `${steps} шагов за тик`;
  if (last === 1) return `${steps} шаг за тик`;
  if (last >= 2 && last <= 4) return `${steps} шага за тик`;
  return `${steps} шагов за тик`;
}

function coloroscopeView(): string {
  const c = state.color;
  const modes = state.colorModes;
  return `
    <div class="narrow">
      <section class="well">
        <h3 class="well-title">Колоскоп</h3>
        <p class="prose">
          Перестановщик цветов: обход всех 16 777 216 оттенков RGB. Самый быстрый
          канал идёт пилообразно, от нуля до 255 и обратно, а средний и медленный
          считаются как разряды смешанной системы счисления. Шесть режимов это все
          перестановки трёх каналов.
        </p>
        <div class="btn-row">
          <button class="btn btn-primary" data-action="color-new">Начать с нуля</button>
          <button class="btn" data-action="color-random">Случайный цвет</button>
          <button class="btn" data-action="color-play" id="color-play">${
            state.colorPlaying ? "Стоп" : "Пуск"
          }</button>
          <button class="btn" data-action="color-step">Шаг вперёд</button>
        </div>
        <div class="field field-range">
          <label for="color-speed">Скорость: <span id="color-speed-label">${speedLabel(
            state.colorSpeed
          )}</span></label>
          <input id="color-speed" class="input" type="range" min="1" max="64" value="${state.colorSpeed}" />
        </div>
      </section>
      ${
        c
          ? `<section class="well">
              <h3 class="well-title">Текущий цвет</h3>
              <div class="color-swatch" id="color-swatch" style="background:${esc(c.hex)}"></div>
              <dl class="spec spec-grid">
                <div><dt>цвет</dt><dd class="mono" id="color-hex">${esc(c.hex)}</dd></div>
                <div><dt>RGB</dt><dd class="mono" id="color-rgb">${esc(c.rgb)}</dd></div>
                <div><dt>красный</dt><dd id="color-r">${c.r}</dd></div>
                <div><dt>зелёный</dt><dd id="color-g">${c.g}</dd></div>
                <div><dt>синий</dt><dd id="color-b">${c.b}</dd></div>
                <div><dt>режим</dt><dd>${esc(c.mode_name)}</dd></div>
                <div><dt>шаг в цикле</dt><dd id="color-step">${c.step_in_cycle.toLocaleString(
                  "ru-RU"
                )} из ${c.cycle_steps.toLocaleString("ru-RU")}</dd></div>
                <div><dt>всего цветов</dt><dd>${c.distinct_colors.toLocaleString("ru-RU")}</dd></div>
              </dl>
              <div class="btn-row">
                <button class="btn" data-action="color-copy">Копировать цвет</button>
              </div>
            </section>`
          : `<section class="well well-empty">
              <p class="sheet-lead">Колоскоп молчит</p>
              <p class="sheet-note">Начните с нуля или возьмите случайный цвет.</p>
            </section>`
      }
      ${
        modes.length
          ? `<section class="well">
              <h3 class="well-title">Режимы</h3>
              <div class="mode-grid">
                ${modes
                  .map(
                    (m) => `
                <button class="mode-btn ${
                  c && c.mode === m.index ? "active" : ""
                }" data-mode="${m.index}">
                  <span class="mode-name">${esc(m.name)}</span>
                  <span class="mode-channels mono">${esc(m.channels[0])} → ${esc(
                    m.channels[1]
                  )} → ${esc(m.channels[2])}</span>
                </button>`
                  )
                  .join("")}
              </div>
            </section>`
          : ""
      }
    </div>`;
}

function contentsView(): string {
  if (!state.contents.length) {
    return `
    <section class="well well-empty">
      <p class="sheet-lead">Оглавление пусто</p>
      <p class="sheet-note">Откройте любой том, и заголовки всех его 410 страниц появятся здесь.</p>
    </section>`;
  }
  const items = state.contents
    .map(
      (c) => `
      <button class="contents-item ${
        state.page && state.page.address.page === c.page ? "active" : ""
      }" data-page="${c.page}">
        <span class="pg mono">${c.page}</span>
        <span class="ttl">${esc(c.title)}</span>
      </button>`
    )
    .join("");
  return `
    <section class="well">
      <h3 class="well-title">Страницы тома</h3>
      <div class="contents-list">${items}</div>
    </section>`;
}

function bookmarksView(): string {
  if (!state.bookmarks.length) {
    return `
    <section class="well well-empty">
      <p class="sheet-lead">Закладок нет</p>
      <p class="sheet-note">На открытой странице нажмите «В закладки».</p>
    </section>`;
  }
  const items = state.bookmarks
    .map(
      (b) => `
      <div class="bm-item">
        <div class="bm-main">
          <p class="bm-addr mono" title="${esc(b.full)}">${esc(
            shortenAddress(b.full, 16, 12)
          )}</p>
          <p class="bm-meta">${esc(formatDate(b.saved_at))} · ${esc(b.hash)}</p>
        </div>
        <button class="btn" data-bm-open="${esc(b.hash)}">Открыть</button>
        <button class="icon-btn danger" data-bm-del="${esc(b.hash)}" title="Удалить закладку" aria-label="Удалить закладку">✕</button>
      </div>`
    )
    .join("");
  return `
    <section class="well">
      <h3 class="well-title">Закладки</h3>
      <div class="bm-list">${items}</div>
    </section>`;
}

function aboutView(): string {
  const info = state.info;
  const letters = info
    ? [...info.alphabet]
        .map((c) => {
          const cls = /[а-яё]/i.test(c) ? "cyr" : /[ ,.!-]/.test(c) ? "punc" : "";
          return `<span class="${cls}">${esc(c)}</span>`;
        })
        .join("")
    : "";
  return `
    <div class="narrow">
      <section class="well">
        <h3 class="well-title">Что это</h3>
        <p class="prose">
          Вавилонская библиотека Хорхе Луиса Борхеса: бесконечная библиотека,
          в которой есть каждая возможная комбинация символов. Здесь библиотека
          вычисляется на лету, ни одна страница не хранится на диске. Адрес
          страницы и её содержимое связаны обратимой математической операцией,
          поэтому содержимое однозначно определяет адрес и наоборот.
        </p>
      </section>
      <section class="well">
        <h3 class="well-title">Структура</h3>
        <dl class="spec spec-grid">
          <div><dt>символов в алфавите</dt><dd>${info ? info.alphabet_size : "..."}</dd></div>
          <div><dt>символов на странице</dt><dd>${info ? info.page_symbols : "..."}</dd></div>
          <div><dt>строк на странице</dt><dd>${info ? info.lines_per_page : "..."}</dd></div>
          <div><dt>символов в строке</dt><dd>${info ? info.chars_per_line : "..."}</dd></div>
          <div><dt>страниц в томе</dt><dd>${info ? info.pages_per_book : "..."}</dd></div>
          <div><dt>стен, полок, томов</dt><dd>${
            info ? `${info.walls} / ${info.shelves} / ${info.volumes}` : "..."
          }</dd></div>
          <div><dt>страниц в гексе</dt><dd>${
            info
              ? (info.pages_per_book * info.walls * info.shelves * info.volumes).toLocaleString(
                  "ru-RU"
                )
              : "..."
          }</dd></div>
        </dl>
      </section>
      <section class="well">
        <h3 class="well-title">Алфавит</h3>
        <div class="alphabet-strip">${letters}</div>
        <p class="prose">
          Кириллица добавлена в базовый набор: кириллические страницы это
          полноправные части библиотеки, а не транслитерация.
        </p>
      </section>
      <section class="well">
        <h3 class="well-title">Источник</h3>
        <p class="prose">
          Оригинал: <a href="https://libraryofbabel.info" data-action="open-site">libraryofbabel.info</a>,
          реализация Джонатана Базиле. Эта версия самостоятельная, на Rust и Tauri.
        </p>
      </section>
    </div>`;
}

function errorBlock(): string {
  if (!state.error) return "";
  return `<div class="state-panel state-error" role="alert">
    <p class="state-lead">Не получилось</p>
    <p class="state-note">${esc(state.error)}</p>
  </div>`;
}

function viewBody(): string {
  switch (state.view) {
    case "reader":
      return readerView();
    case "browse":
      return browseView();
    case "search":
      return searchView();
    case "images":
      return imagesView();
    case "archive":
      return archiveView();
    case "coloroscope":
      return coloroscopeView();
    case "contents":
      return contentsView();
    case "bookmarks":
      return bookmarksView();
    case "about":
      return aboutView();
  }
}

/** Полная перерисовка интерфейса. */
export function render(): void {
  const root = app();
  root.innerHTML = `
    <div class="shell">
      ${sidebar()}
      <main class="main">
        ${topbar()}
        <div class="content">
          <div class="content-inner">
            ${errorBlock()}
            ${viewBody()}
          </div>
        </div>
      </main>
    </div>
    <div class="scrim" id="scrim" hidden></div>`;
  restoreFocus();
  syncShell();
}

/**
 * Восстанавливает состояние узлов, которые переживают полную перерисовку.
 *
 * Надписи кнопок «Пуск» и «Стоп» живут в только что пересозданных узлах,
 * а открытое меню разделов задаётся классом на body.
 */
export function syncShell(): void {
  const play = document.getElementById("color-play");
  if (play) play.textContent = state.colorPlaying ? "Стоп" : "Пуск";
  const arch = document.getElementById("archive-play");
  if (arch) arch.textContent = state.archivePlaying ? "Стоп" : "Слайдшоу";
  const toggle = document.getElementById("nav-toggle");
  if (toggle) {
    toggle.setAttribute(
      "aria-expanded",
      document.body.classList.contains("nav-open") ? "true" : "false"
    );
  }
}

/**
 * Точечное обновление кадров колоскопа.
 *
 * Полный render() во время обхода недопустим: он переписывает innerHTML,
 * и кнопка «Стоп» пересоздаётся между mousedown и mouseup, click не
 * срабатывает. Здесь меняются только значения, кнопки остаются теми же
 * узлами DOM.
 */
export function updateColorFrame(): void {
  const c = state.color;
  if (!c) return;
  const set = (id: string, text: string): void => {
    const el = document.getElementById(id);
    if (el) el.textContent = text;
  };
  const swatch = document.getElementById("color-swatch");
  if (swatch) swatch.style.background = c.hex;
  set("color-hex", c.hex);
  set("color-rgb", c.rgb);
  set("color-r", String(c.r));
  set("color-g", String(c.g));
  set("color-b", String(c.b));
  set(
    "color-step",
    `${c.step_in_cycle.toLocaleString("ru-RU")} из ${c.cycle_steps.toLocaleString("ru-RU")}`
  );
  set("color-speed-label", speedLabel(state.colorSpeed));
}

/**
 * Точечное обновление подписи скорости обхода.
 *
 * Слайдер меняет скорость во время обхода, а полная перерисовка в этот
 * момент пересоздаёт кнопку «Стоп» между нажатием и отпусканием, и клик
 * не срабатывает. Поэтому подпись обновляется на месте.
 */
export function updateSpeedLabel(): void {
  const el = document.getElementById("color-speed-label");
  if (el) el.textContent = speedLabel(state.colorSpeed);
}

/**
 * Точечное обновление слайдшоу архива по той же причине, что и
 * updateColorFrame: полный render() пересоздаёт кнопку «Стоп» между
 * mousedown и mouseup, и клик не срабатывает.
 */
export function updateArchiveView(): void {
  const a = state.archive;
  if (!a) return;
  const img = document.getElementById("archive-image") as HTMLImageElement | null;
  if (img && img.getAttribute("src") !== a.png) img.setAttribute("src", a.png);
  const cap = document.querySelector<HTMLElement>(".archive-figure figcaption");
  if (cap) cap.textContent = `Локация № ${a.short}`;
  const loc = document.getElementById("archive-location") as HTMLTextAreaElement | null;
  if (loc && loc.value !== a.location) loc.value = a.location;
}

/** Возвращает фокус в поле ввода, если пользователь в нём печатал. */
let lastFocusId: string | null = null;

export function rememberFocus(): void {
  const el = document.activeElement as HTMLElement | null;
  if (el && el.id) lastFocusId = el.id;
}

function restoreFocus(): void {
  if (!lastFocusId) return;
  const el = document.getElementById(lastFocusId) as HTMLInputElement | null;
  if (el) {
    el.focus();
    const len = el.value.length;
    try {
      el.setSelectionRange(len, len);
    } catch {
      /* не текстовое поле */
    }
  }
}
