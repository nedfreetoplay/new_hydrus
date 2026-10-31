# План портирования GUI Hydrus

Документ фиксирует разрыв между `hydrus_python/hydrus/client/gui` и текущим
Rust-портом и служит чек-листом. Флаги обновляются только после выполнения
Definition of Done для задачи.

## Легенда

- `[x]` — портировано и проверено.
- `[~]` — частично: UI/поведение есть, но не покрывает Hydrus.
- `[ ]` — не начато.
- `P0` — без этого GUI не воспринимается как Hydrus.
- `P1` — важные ежедневные сценарии.
- `P2` — полировка и редкие сценарии.

## Definition of Done

Задача переводится в `[x]`, когда:

1. Поведение совпадает с Python-версией для основного и ошибочного пути.
2. Состояние сохраняется в session/options или соответствующем JSON.
3. Есть unit/integration-тест на логику; UI проверен скриншотом.
4. Внешний вид совпадает с Python-экраном по правилу ниже: те же контролы, порядок,
   подписи, размеры, отступы, иконки и тема.
5. `cargo build -p hydrus_client` без предупреждений, тесты зелёные.
6. Обновлён этот файл и, при необходимости, `docs/ARCHITECTURE.md`.

## Правило визуального паритета

Функционал без внешнего вида — не выполненная задача. Перенос повторяет не
«похожий» экран, а конкретный Python-экран.

Перед вёрсткой любого экрана открыть его Python-модуль
(`hydrus_python/hydrus/client/gui/...`) и выписать:

1. Заголовок окна/вкладки и текст кнопок дословно, включая регистр, многоточия
   и скобки.
2. Порядок и группировку контролов: какие строки формы, сплиттеры, вкладки,
   аккордеоны, панели и в каком порядке.
3. Размеры: стартовый размер окна, минимальные размеры контролов, высоту строк
   таблиц, ширину колонок, отступы (margin) и промежутки (spacing).
4. Таблицы/списки: порядок и текст заголовков, выравнивание, сортировку,
   режим выделения, поведение двойного клика.
5. Иконки (те же имена из `ClientGUIResources.py`), подсказки, placeholder'ы,
   inline help и тексты пустых состояний.
6. Меню и контекстные меню: состав, порядок, разделители, горячие клавиши.
7. Цвета и стиль: тема Hydrus по умолчанию (светлая нативная палитра Windows;
   тёмный фон только у media canvas), стили таблиц/кнопок из `ClientGUIStyle.py`,
   а не собственная палитра.

Затем сверить с оригиналом: запустить Python-клиент на тестовой БД (venv:
`python hydrus_python/setup_venv.py -i a`, затем
`hydrus_python\venv\Scripts\python.exe hydrus_python\hydrus_client.py -d <db>`;
подробности — skill `hydrus-python-run`) или взять скриншот из справки Hydrus и
сравнить по списку выше. Только после этого верстать Qt — с теми же числами,
текстами и иконками.

Сдача задачи: приложить скриншот Rust-экрана рядом с Python-экраном и перечислить
оставшиеся расхождения (или написать «расхождений нет»).

Не считается выполнением:

- другой порядок или группировка контролов, «универсальная» форма вместо
  Hydrus-диалога;
- свои подписи, пропущенные иконки, заголовки, подсказки или help-текст;
- другие размеры окна, отступы, высота строк и ширина колонок;
- светлая тема или дефолтный вид виджетов вместо стиля Hydrus;
- произвольная палитра вместо темы Hydrus по умолчанию (тёмный фон панели
  миниатюр там, где в Hydrus он светлый, и наоборот);
- таблица без сортировки, выделения и двойного клика, как в оригинале.

### Журнал сверок

- 2026-10-29 — главное окно (страница files). Меню приведено к порядку Python
  (`file, undo, pages, database, network, services, tags, pending, help`):
  downloader-страницы ушли в `pages -> download`, подписки — в `network`, лишний
  верхний `download` удалён. Статус-бар получил шесть секций Python (media,
  bandwidth, idle, hydrus busy, CPU busy, db) с теми же фиксированными ширинами и
  size grip. Sidebar пересобран из `StaticBox`: `sort by ...`, `no collections`,
  `search` (file/tag domain, autocomplete, predicates, search), `selection tags`;
  убраны вкладки-«+», крестики закрытия и лишняя панель metadata (теги теперь через
  `manage tags`/viewer). Сетка миниатюр стала белой с рамками Hydrus и голубым
  выделением, статус пустой страницы — `no search done yet`.
  Скриншот: `docs/images/main_window.png`.
  Осталось (снято следующей сверкой): иконки Hydrus, сворачивание панели
  `search`, полноценные file/tag domain селекторы, точная ширина вкладок.
- 2026-10-29 — Python-клиент v692 запущен локально: venv собран через
  `setup_venv.py -i a` (Python 3.14.7, PySide6 6.11.1, dev-зависимости). Снят
  реальный скриншот главного окна `docs/images/python_main_window.png`; теперь
  эталон можно переснимать на любой странице, а не полагаться только на старые
  картинки из `hydrus_python/docs/images`. Инструкция — skill `hydrus-python-run`.
- 2026-10-29 — главное окно сверено с живым Python v692 в одинаковом окне
  (1296x839, клиентская область 1280x800). Светлая тема переведена с Fusion и
  самодельного белого QSS на нативный `windowsvista` + `default_hydrus.qss`
  (зелёные/красные `HydrusOnOffButton`); заголовок окна стал
  `main - hydrus client 689` (application display name, как в Python).
  Sidebar: `sort by file: filesize` / `smallest first` кнопками с меню, `no
  collections` + шестерёнка; `search` — раскрывающийся `StaticBox` с полем
  предикатов, голубым полем ввода и иконками paste/star/clear/lock, кнопками
  `include current tags` / `include pending tags`, `searching immediately` / `OR`,
  `my files` / `all known tags`, вкладками `results` / `favourites` / `children`
  и списком system-предикатов. `selection tags` показывается при сворачивании
  `search`, как в Python. Пустой статус — `no search done yet`, после поиска —
  `no files found for this search`; центрированный текст с холста убран, в
  статус-баре `0B` и `idle`. Панель preview белая и показывается только при
  высокой клиентской области. Скриншоты: `docs/images/main_window.png` (Rust,
  полное окно) и `docs/images/python_main_window_1280.png` (Python, то же окно).
  Осталось: точный набор иконок Hydrus вместо нарисованных приближений, полные
  меню file/tag domain, sort и collect, поведение `favourites`/`children` и
  drop-down автодополнения, пиксельная ширина сплиттера.
- 2026-10-29 — gallery downloader сверен с живым Python v692 в окне 1296x839.
  Страница перестроена из формы в Python-sidebar: `sort by file: filesize` /
  `smallest first`, `no collections`, `gallery downloader` (`waiting for new
  queries`, таблица `query/source/files status/search status/status/items/added`
  с сортировкой по query, ряд из шести иконок highlight/clear/pause files/pause
  search/retry/remove), поле query с paste/gear, `no downloader set` (меню GUG),
  `stop after this many files: 2000` + `no limit`, `import options (all default)`;
  ниже `highlighted query` с вложенными `imports` и `search` (статус, seed log,
  прогресс, network job), тем же limit и import options; внизу `selection tags`.
  Пустой статус-бар — `no highlighted query`; для watcher-страницы —
  `no highlighted watcher`. File limit теперь реально ограничивает число
  найденных ссылок. Скриншоты: `docs/images/gallery_page.png` (Rust) и
  `docs/images/python_gallery_page.png` (Python). Осталось: точная ширина колонок
  и элизия заголовков, включённое состояние disabled-панелей при пустом
  highlight, точные иконки Hydrus, полноценные multi-query/GUG-редакторы,
  а также отдельная сверка urls/simple/watcher страниц, subscriptions,
  duplicates, import/export folders и options dialogs.
- 2026-10-30 — urls downloader сверен с живым Python v692 (окно 1296x839).
  Страница `url import` перестроена в Python-sidebar: `url downloader` с
  вложенными `imports` (pause, file seed cache с `file log ▾`, gauge, network
  job `0%`/stop/cog) и `search` (`search log ▾`, gauge, network job), ниже
  однострочное поле `any url hydrus recognises, or a raw file url` с paste,
  `import options (all default)` с меню и `selection tags`. Кнопки start нет;
  Enter/paste пендит URL и очищает поле. Пустой статус-бар — `empty page`;
  вкладка называется `url import`, а не `urls`. Скриншоты:
  `docs/images/urls_page.png` (Rust) и `docs/images/python_urls_page.png`.
  Осталось: добавление новых URL во время работы (сейчас страница занята),
  точные иконки Hydrus, поведение disabled-панелей. Заодно меню `pending`
  скрывается, когда нет tag/file repository или IPFS-сервиса, как в Python.
- 2026-10-30 — simple downloader сверен с живым Python v692 (окно 1296x839).
  Страница перестроена в Python-sidebar: `simple downloader` с вложенными
  `imports` (current action/pause, file seed cache, network job) и `parsing`
  (parser status/pause, `parsing log ▾`, network job, очередь page-задач
  8 строк с кнопками ↑/X/↓, однострочный `url to be parsed by the selected
  formula` с paste, formula combo + cog), ниже `import options (all default)`
  и `selection tags`. Очередь хранится в `page.queue`; порядок и удаление
  сохраняются действием `downloader_jobs_set`. Списки seed cache/log теперь
  пустые при отсутствии элементов, как в Python. Sidebar скроллится, когда
  контент выше окна. Скриншоты: `docs/images/simple_page.png` (Rust) и
  `docs/images/python_simple_page.png`. Осталось: редактор формул (WS-07.11),
  реальный multi-job парсинг, точные иконки Hydrus.
- 2026-10-30 — watcher сверен с живым Python v692 (окно 1296x839).
  Страница `watcher` перестроена из одиночной формы в Python-sidebar:
  `watchers` со статусами (`waiting for new watchers` / `1 watchers - x/y`),
  таблицей subject/files status/checking status/status/items/added, рядом
  иконок highlight/clear highlight/pause files/pause checking/`check now`/
  retry/trash, полем `watcher url` с paste, `checker options` и
  `import options (all default)`; ниже `highlighted watcher` с `no subject`,
  URL и вложенными `imports`/`search` (`check log ▾`), внизу `selection tags`.
  Пустой статус-бар — `no highlighted watcher`, после добавления URL —
  `no files for this watcher and its publishing settings`. Добавление URL
  создаёт строку watcher (`* unknown`, `just added`, `now`) и заполняет
  highlight; `check now` запускает проверку через `watcher_check`.
  Скриншоты: `docs/images/watcher_page.png` (Rust) и
  `docs/images/python_watcher_page.png` (Python). Осталось: несколько watcher
  в таблице, диалог checker options, highlight → media panel, retry ignored,
  точные иконки.
- 2026-10-31 — менеджер подписок сверен с живым Python v692. В Python нет
  страницы подписок: `network -> subscriptions…` открывает диалог
  `manage subscriptions` (`EditSubscriptionsPanel`). Rust теперь делает то же:
  пункт меню `subscriptions…` с описанием Python, окно `manage subscriptions`,
  таблица `name/source/status/last new file time/last checked/error/delay?/
  items/paused/custom import options`, справа сверху `help for this panel -->`,
  ниже строки кнопок `add/edit/delete/export/import/duplicate`,
  `merge/separate/deduplicate/lowercase`, `pause/resume` + retry-меню,
  `scrub delays/check queries now/reset`, `select subscriptions/overwrite
  downloader/overwrite checker options`, `import options:` с copy/paste/star/
  clear и внизу `apply`/`cancel`. Выбор строки включает batch-кнопки, диалог
  обновляется после каждого действия. Работают add, edit, delete, pause/resume,
  check now, scrub delays, reset, lowercase, duplicate, clear import options,
  copy/paste import options; сетевой экспорт/импорт и overwrite-команды пока
  сообщают о нереализованности. Скриншоты: `docs/images/subscriptions_dialog.png`
  (Rust) и `docs/images/python_subscriptions_dialog.png`.
- 2026-10-31 — страница duplicates сверена с живым Python v692. Страница
  `duplicates processing` получила Python-sidebar с вкладками
  `preparation | filtering | auto-resolution` (по умолчанию `filtering`):
  `potential duplicate pairs search` (тип поиска, поле `system:everything`,
  paste/star/clear/lock, `maximum search distance of pair:`, `can be pixel dupes`,
  строка `no potential pairs in this file domain!` с иконками info/refresh/gear),
  `duplicate filter` (`edit default duplicate metadata merge options`, сортировка
  `sort pairs by: filesize of larger file` + `largest first` + `mixed pairs`,
  `launch the filter`), `quick and dirty processing` (`sort by file: filesize` /
  `smallest first`, `show some random potential duplicates`, три кнопки
  set-current-media) и `selection tags`. Панели preparation/auto-resolution пока
  содержат заглушки, справа остаётся сравнение пары (в Python там media panel).
  Скриншоты: `docs/images/duplicates_page.png` (Rust) и
  `docs/images/python_duplicates_page.png` (Python). Осталось: media panel
  результатов, полноценный preparation-панель, auto-resolution-редактор,
  групповой режим и точные иконки.
- 2026-10-31 — менеджер import folders сверен с живым Python v692. В Python это
  не страница, а диалог `file -> import/export folders -> manage import
  folders…` с заголовком `edit import folders` (`EditImportFoldersPanel`).
  Rust теперь открывает такой же диалог из меню `manage import folders…`:
  вступление, красное предупреждение, таблица `name/path/paused/check period`,
  строки `add/edit/delete` справа и `apply/cancel` внизу. Кнопки выключаются
  без выделения, двойной клик редактирует папку, таблица сортируется по имени.
  В глобальный QSS добавлены правила Python `HydrusWarning` (#800000),
  `HydrusIndeterminate` (#000080), `HydrusAccept` (#008000) и
  `HydrusCancel` (#800000); заголовки таблиц теперь выровнены влево, как в
  Python. Скриншоты: `docs/images/import_folders_dialog.png` (Rust) и
  `docs/images/python_import_folders_dialog.png` (Python). Осталось: сверка
  диалога редактирования одной папки, export folders (пока страница) и
  подменю check-now по папкам.

## Текущий статус

| ID | Направление | Статус | Приоритет | Размер | Зависит от |
| --- | --- | --- | --- | --- | --- |
| WS-01 | Каркас страниц и сессии | `[~]` | P0 | XL | — |
| WS-02 | Поиск и предикаты | `[~]` | P0 | XL | WS-01 |
| WS-03 | Результаты и миниатюры | `[~]` | P0 | L | WS-01 |
| WS-04 | Media viewer | `[~]` | P0 | XL | WS-03 |
| WS-05 | Теги и metadata | `[~]` | P1 | XL | WS-03 |
| WS-06 | Импорт | `[~]` | P1 | XL | WS-01 |
| WS-07 | Загрузчики и парсеры | `[~]` | P1 | XL | WS-06 |
| WS-08 | Дубликаты | `[~]` | P1 | L | WS-03 |
| WS-09 | Сервисы, репозитории, API | `[~]` | P1 | XL | — |
| WS-10 | Экспорт | `[x]` | P1 | L | WS-06 |
| WS-11 | Обслуживание файлов и БД | `[~]` | P1 | L | WS-09 |
| WS-12 | Options framework | `[~]` | P1 | XL | — |
| WS-13 | UX, уведомления, системная интеграция | `[~]` | P2 | L | WS-01, WS-12 |
| WS-14 | Финальный parity-аудит | `[ ]` | P0 | M | все |

Текущее покрытие: базовые сценарии есть, но из ~225 GUI-модулей Python прямой
аналог имеют единицы. До полной идентичности — все фазы ниже.

Отдельный долг: ранее закрытые экраны проверялись на функционал, но не на внешний
вид. Их нужно прогнать по «Правилу визуального паритета» и только потом оставлять
`[x]`; до этого такие пункты считать `[~]`.

---

## WS-01. Каркас страниц и сессии

Python: `pages/ClientGUIPages*`, `ClientGUIPagesTreeModel/View`,
`ClientGUIPageManager`, `ClientGUISession*`, `panels/options/GUIPagesPanel`,
`GUISessionsPanel`.

Rust-цели: `gui/app/model.rs`, `gui/app/session.rs`, `gui/pages.rs`,
`gui/app/render.rs`, `gui/qt/bridge.*`.

- [x] 01.1 Вкладки: создание, закрытие, переименование, «+».
- [x] 01.2 Сохранение текущего дерева страниц в `gui session.json` (v3).
- [x] 01.3 Вложенные notebook’и в модели.
- [x] 01.4 Drag & drop вкладок и страниц, перенос между notebook’ами.
- [x] 01.5 Дерево страниц (tree view) с иконками, счётчиками и сворачиванием.
- [x] 01.6 История навигации: назад/вперёд, список истории.
- [x] 01.7 Менеджер страниц: поиск, фильтр, массовое закрытие.
- [x] 01.8 Сессии: save as, load, append, append backup, delete, backup-файлы.
- [~] 01.9 Undo: закрытые страницы и добавление/удаление поисковых предикатов.
- [x] 01.10 Дублирование страницы и копирование поискового контекста.
- [ ] 01.11 Petition-страницы и sidebar (совместно с WS-09).
- [x] 01.12 Восстановление размеров/позиций окон и вкладок.

Выход фазы: можно построить произвольное дерево, перетащить страницы,
сохранить/загрузить/дополнить сессию и отменить закрытие.

## WS-02. Поиск и предикаты

Python: `search/ClientGUIACDropdown`, `ClientGUIPredicates*`,
`ClientGUISearch*`, `ClientGUILocation`, `ClientGUISearchPanels`,
`metadata/ClientGUIIncrementalTagging`.

Rust-цели: `gui/app/query.rs`, новый `gui/app/search.rs`,
`gui/qt/bridge.*`, `db/tag_search`, `db/files_metadata`.

- [x] 02.1 Текстовый поиск: теги, `-tag`, wildcard, namespace, `system:inbox`,
  `system:archive`, `system:everything`, `system:limit`.
- [x] 02.2 Простой autocomplete по тегам.
- [x] 02.3 Модель списка предикатов вместо разбора строки.
- [x] 02.4 File domain и tag service selector’ы.
- [x] 02.5 Autocomplete dropdown с вкладками и счётчиками.
- [x] 02.6 OR-предикаты и advanced OR editor.
- [x] 02.7 Системные предикаты: размер, разрешение, ratio, mime, hash.
- [x] 02.8 Системные предикаты: duration, fps, frames, num pixels, num words.
- [x] 02.9 Системные предикаты: рейтинги, число тегов/URL/заметок.
- [x] 02.10 Системные предикаты: даты импорта/архивации/просмотра/изменения.
- [x] 02.11 Системные предикаты: known URL/domain/regex/URL class.
- [x] 02.12 Системные предикаты: duplicate relationships, similar files.
- [x] 02.13 Системные предикаты: file viewing stats, file service.
- [x] 02.14 Favourite searches и их менеджер.
- [x] 02.15 История поиска и undo добавления/удаления предикатов.
- [x] 02.16 Сохранение поискового контекста в странице/сессии.

Выход фазы: поиск собирается из предикатов, поддерживает OR, домены и
системные фильтры; любимые поиски сохраняются.

## WS-03. Результаты и миниатюры

Python: `pages/ClientGUIMediaResultsPanel*`, `MediaResultsPanelThumbnails`,
`MediaResultsPanelSortCollect`, `ClientGUIThumbnailLayouts`,
`pages/ClientGUIMediaResultsPanelMenus`.

Rust-цели: `gui/app/query.rs`, `gui/app/render.rs`, `gui/qt/bridge.*`,
новый `gui/app/results.rs`.

- [x] 03.1 Сетка миниатюр и превью.
- [x] 03.2 Ctrl+wheel zoom миниатюр.
- [x] 03.3 Базовое контекстное меню (open, tags, archive, inbox, delete).
- [~] 03.4 Sort/collect controls и сохранение режима.
- [x] 03.5 Множественное выделение: действия над всеми выбранными файлами.
- [x] 03.6 Correct focused vs selected file semantics.
- [~] 03.7 Masonry layout, scroll direction, thumbnail sizing options
  (масштаб, горизонтальный скролл и variable-height masonry готовы; waterfall
  по колонкам — приближение на QListWidget).
- [x] 03.8 Loading panel/progress и file count.
- [x] 03.9 Rating overlays, duplicate indicators, archive/inbox markers.
- [x] 03.10 Drag & drop файлов, miniatures между страницами.
- [~] 03.11 Полные media menus: copy hash/URL, open externally, file services,
  timestamps, notes, ratings.
- [x] 03.12 Selection-aware статус-бар и счётчики.

Выход фазы: results panel ведёт себя как Hydrus при сортировке, выделении,
загрузке и действиях над группой файлов.

## WS-04. Media viewer

Python: `canvas/ClientGUICanvas*`, `ClientGUIMPV`, `ClientGUIQtMediaPlayer`,
`ClientGUIMediaControls`, `ClientGUIMediaMenus`, `ClientGUIEditTimestamps`,
`panels/ClientGUIScrolledPanelsEdit` (media view options).

Rust-цели: `gui/qt/bridge.*`, `gui/app/render.rs`, `gui/app/metadata.rs`,
`client_options`.

- [x] 04.1 Статичные изображения, prev/next, fit, zoom, rotate, fullscreen.
- [x] 04.2 Metadata sidebar в viewer.
- [x] 04.3 Базовые media options (zoom step, recenter, always-on-top, frameless).
- [ ] 04.4 MPV backend для видео.
- [ ] 04.5 QtMediaPlayer fallback и audio playback.
- [~] 04.6 GIF/WebP/animation bar и pause/play.
- [x] 04.7 Hover frames: top archive/delete/filter, top-right, tags, notes,
  ratings.
- [ ] 04.8 Duplicate filter canvas.
- [x] 04.9 Zoom modes menu, pan modes, background colour/checkerboard.
- [x] 04.10 Prefetch previous/next.
- [~] 04.11 Open externally и embed button.
- [x] 04.12 Восстановление размера/позиции viewer.
- [~] 04.13 Media actions: copy, file services, notes, timestamps, ratings.
- [ ] 04.14 Media playback options panel.

Выход фазы: viewer открывает изображения, видео, аудио и анимации; hover-панели
и prefetch соответствуют Hydrus.

## WS-05. Теги и metadata

Python: `metadata/ClientGUIManageTags`, `ManageTagSiblings`,
`ManageTagParents`, `ClientGUITagDisplayOptions`, `TagDisplayMaintenance*`,
`ClientGUITagFilter`, `TagSummaryGenerator`, `TagActions`, `MigrateTags`,
`EditTimestamps`, `MetadataConditional`, `MetadataMigration*`,
`ClientGUITagSuggestions`, `ClientGUIRatings`, `ClientGUIDialogsManage`.

Rust-цели: `gui/app/metadata.rs`, `gui/app/tag_relations.rs`,
`gui/app/options.rs`, `gui/qt/bridge.*`, `db/tag_display`, `db/tag_relations`.

- [x] 05.1 Теги по локальным tag-сервисам.
- [x] 05.2 Добавление/удаление тегов.
- [x] 05.3 Like/dislike, inc/dec и числовые рейтинги.
- [x] 05.4 Заметки: просмотр и редактирование.
- [x] 05.5 Siblings/parents: добавление и удаление пар с проверкой циклов.
- [x] 05.6 Display tags с учётом siblings и tooltip’ом storage-тегов.
- [x] 05.7 Manage Tags: несколько сервисов и массовое применение.
- [ ] 05.8 Pending/deleted/petitioned tags.
- [ ] 05.9 Tag filter (allow/deny) и tag summary.
- [ ] 05.10 Tag display manager: namespace/subtag order, colours, decorators.
- [ ] 05.11 Tag display maintenance edit/review.
- [ ] 05.12 Suggested tags: recent, related, most used, favourites.
- [ ] 05.13 File lookup script tags.
- [ ] 05.14 Incremental tagging.
- [x] 05.15 Tag sorting options: lexicographic/namespace/subtag, а также
  показ namespace’ов и замена `_` на пробелы в options → tag presentation.
- [x] 05.16 Tag actions: copy/paste/apply к выделению.
- [x] 05.17 Редактор timestamps.
- [ ] 05.18 Metadata conditional.
- [ ] 05.19 Metadata migration (routers, sidecars, test panel).
- [ ] 05.20 Migrate tags между сервисами.
- [ ] 05.21 Полный DialogManageRatings.

Выход фазы: управление тегами не уступает Hydrus по сервисам, фильтрам,
display-настройкам и массовым операциям.

## WS-06. Импорт

Python: `importing/ClientGUIImport*`, `ImportOptions*`, `ImportFolders`,
`ClientGUIFileSeedCache`, `ClientGUIGallerySeedLog`,
`pages/ClientGUISidebarImporters`, `panels/ClientGUILocalFileImports`,
`parsing/ClientGUIParsing*`.

Rust-цели: `gui/app/import.rs`, `gui/app/import_folders.rs`,
`gui/app/worker.rs`, новый `gui/app/import_options.rs`,
`gui/qt/bridge.*`, `importing/*`.

- [x] 06.1 Локальный импорт файлов.
- [x] 06.2 Импорт одного URL.
- [x] 06.3 Import folders: период, pause, subdirectories.
- [x] 06.4 Фильтры: расширения, min/max size, skip recent.
- [x] 06.5 Post-import move/delete/ignore.
- [x] 06.6 Import options framework: file filtering, location, notes,
  presentation, prefetch, tags, external programs. Каждый тип — отдельная структура
  с сериализацией; контейнер хранит только явно заданные типы, отсутствующие
  наследуются. Диалог редактирует контейнер по вкладкам и умеет сбрасывать тип на
  «use default». Локальный импорт применяет file filtering, destinations и
  automatic archive из эффективных опций.
- [x] 06.7 Default/specific import options containers: менеджер хранит default
  контейнер для каждого caller type (global, client api, local import, import
  folder, watcher urls, post urls, subscription, url class, specific importer) и
  именованные favourites; эффективные опции собираются по приоритету specific ->
  caller -> global. Менеджер сохраняется в options group `import_options`.
- [x] 06.8 Filename tagging options: `FilenameTaggingOptions` с тегами для всех
  файлов, добавлением имени файла, шестью directory-слотами (0,1,2,-3,-2,-1),
  regex'ами и namespace-regex'ами. Import folder хранит набор опций на каждый
  локальный tag service; диалог папки показывает список сервисов, редактирует
  правила с preview по примеру пути, а успешный импорт добавляет полученные теги
  в этот tag service.
- [x] 06.9 Local import review panel: меню «import files...» и drag/drop открывают
  review-диалог вместо немедленного импорта. Список путей показывает индекс, путь,
  результат парсинга (good/empty/missing/unsupported/occupied/sidecar) и размер;
  работают add files/add folder, поиск по подпапкам, remove, сортировка, счётчики
  и summary. Панель использует effective `local_import` import options, позволяет
  включить удаление оригиналов после успешного импорта и открыть filename tagging
  для локальных tag services. «import now» отправляет настоящий import job.
- [x] 06.10 File seed cache UI: диалог показывает очередь страницы (status, url,
  created, modified, note), фильтр по статусу и поиск, сортировку по колонкам,
  копирование URL, retry failed/selected, remove, clear finished и clear all.
  Seed-статусы и таймстампы хранятся в `DownloaderPage.queue` и переживают
  перезапуск через `gui session.json`.
- [x] 06.11 Gallery seed log UI: страница gallery/watcher ведёт журнал
  gallery/post URL со статусами pending/working/success/failed и заметкой о
  числе найденных файлов; worker присылает `GallerySeed` события на каждый
  обработанный page/post. Диалог повторяет фильтры, сортировку, копирование,
  retry, remove и clear, журнал сохраняется в сессии.
- [~] 06.12 Importer sidebars для HDD/gallery/watcher/simple/URLs.
  Sidebar-панели gallery, watcher, URLs и simple повторяют Python: `gallery
  downloader` / `watchers` / `url downloader` / `simple downloader` со
  вложенными `imports` и `search`/`parsing`, file seed cache (`file log ▾`),
  gallery seed log (`search log ▾` / `check log ▾` / `parsing log ▾`), network
  job (gauge, stop, cog), таблица watchers с иконками
  highlight/clear/pause files/pause checking/check now/retry/trash, очередь
  page-задач с ↑/X/↓, поля query/watcher url/url с paste, formula combo с cog,
  `checker options`, `import options (all default)`, `selection tags`.
  Осталось: multi-watcher (сейчас одна строка) и `highlighted watcher` →
  media panel, диалог checker options, HDD-панель.
- [x] 06.13 «Check all» и полное меню import folders.
- [ ] 06.14 Parsing scripts/legacy import scripts.
- [x] 06.15 Review local imports и repair import locations: тот же review-диалог
  доступен из `database -> maintenance -> review local imports...`; repair-диалог
  `repair file locations...` показывает prefix/base/status каждой file storage
  подпапки, позволяет задать новую base, автоматически подобрать
  client_files/thumbnails внутри выбранной папки, пересоздать пустые thumbnail
  папки и сохранить исправленные subfolders в БД. При старте с отсутствующими
  папками repair-диалог открывается автоматически.

Выход фазы: импорт настраивается через Hydrus import options, видны seed-логи,
работают sidebars и filename tagging.

## WS-07. Загрузчики и парсеры

Python: `ClientGUIDownloaders`, `parsing/ClientGUIParsing*`,
`ClientGUIParsingFormulae`, `ClientGUISubscriptions`, `ClientGUILogin`,
`importing/ClientGUIGallerySeedLog`.

Rust-цели: `gui/app/download.rs`, `gui/app/model.rs`,
`parsing/gallery.rs`, новый `gui/app/downloader_editor.rs`,
`networking/client_networking_url_class.rs`, `gui/qt/bridge.*`.

- [x] 07.1 URLs downloader.
- [x] 07.2 Gallery downloader с встроенными парсерами.
- [x] 07.3 Watcher с периодом.
- [x] 07.4 Simple downloader.
- [x] 07.5 Subscriptions: добавление, pause, delete, результаты.
- [x] 07.6 URL class editor.
- [x] 07.7 Per-domain pools и bandwidth rules.
- [ ] 07.8 Gallery URL downloader editor (GUG/NGUG).
- [ ] 07.9 URL class links и downloader display manager.
- [ ] 07.10 Content/page/subsidiary parser editors.
- [ ] 07.11 Formula editor и parser test panel.
- [x] 07.12 Watcher editor/review: watcher-страница редактирует URL, search/filter,
  site parser, subject, период и automatic checks, запускает `check now`, а
  gallery seed log и file seed cache показывают историю проверок и файлов.
- [~] 07.13 Subscription editor/query editor: диалог редактирования подписки
  меняет имя, URL, query, site parser, filter, период и paused; per-subscription
  import options хранятся отдельно. Полноценный multi-query editor и query logs
  Python-подписок ещё не перенесены (текущая модель — одна query на подписку).
- [x] 07.14 Gallery seed log: страница gallery/watcher ведёт журнал page/post URL
  со статусами, заметками и числом найденных файлов; диалог сохраняется в сессии
  (см. WS-06.11).
- [x] 07.15 Downloader export/import.
- [ ] 07.16 Login credentials и login scripts.
- [x] 07.17 Per-downloader import options: у downloader-страницы и подписки есть
  свой контейнер overrides; не заданные типы наследуются от caller
  (`post_urls`, `watcher_urls`, `subscription`) и global. Диалог specific import
  options редактирует только overrides, summary показывается на кнопке, а worker
  применяет effective options к скачанным файлам. Контейнеры сохраняются в
  `gui session.json`.
- [ ] 07.18 Thread slots/network job control UI.

Выход фазы: загрузчик можно создать и настроить из GUI, протестировать парсер
и экспортировать/импортировать определение.

## WS-08. Дубликаты

Python: `duplicates/ClientGUIDuplicate*`, `ClientGUIDuplicates*`,
`ClientGUIPotentialDuplicatesSearchContext`, `ThumbnailPairList`,
`pages/ClientGUISidebarDuplicates`.

Rust-цели: `gui/app/duplicates.rs`, `gui/app/auto_resolution.rs`,
`gui/app/model.rs`, `gui/qt/bridge.*`, `db/duplicates`.

- [x] 08.1 Очередь пар и просмотр двух файлов.
- [x] 08.2 Keep A/B, same, not duplicates, delete both, skip.
- [x] 08.3 Простые auto-resolution правила.
- [x] 08.4 Duplicate filter page/sidebar: search context и пороги: два file search context
  (теги + локальный file domain), one file/both files/different searches, pixel dupes
  required/allowed/excluded, max hamming distance; контекст сохраняется в `gui session.json`.
- [~] 08.5 Batch size/preparation panel: размер батча из `duplicate_filter_max_batch_size`,
  кнопка следующего батча и статус; auto-commit, проценты подготовки и очередь
  поиска ещё нет.
- [x] 08.6 Auto-resolution editor: AND/OR, metadata conditions, pair selectors.
- [x] 08.7 Auto-resolution preview/review и pending/taken/denied списки.
- [x] 08.8 Content merge options по tag/rating/file services: диалог для решений
  «this is better» и «same quality», действия copy/move/two-way/none по каждому tag и
  rating сервису, sync archive/urls/notes/modified date; настройки хранятся в options
  (`duplicate_action_options`) и применяются в merge, auto-resolution и review.
- [x] 08.9 Thumbnail pair list и сортировка: auto-resolution preview показывает пары с
  миниатюрами A/B и действиями, review-диалог — pending/denied/taken с миниатюрами;
  контекстное меню и Enter открывают выбранные пары в новой странице. Сортировка пар
  (larger/smaller filesize, similarity, random; asc/desc) входит в search context,
  сохраняется в `gui session.json` и применяется до auto-resolution и батча.
- [x] 08.10 Полные duplicate actions/menu: this is better with delete/keep both,
  same quality, related alternates, false positive, delete both, custom action
  (тип действия, выбор лучшего файла, delete neither/A/B/both и отдельные merge
  options для пары). Кнопка actions... и контекстное меню на стороне выполняют
  remove potentials, reset potential search, clear all/internal false positives,
  remove from duplicate/alternates group и dissolve duplicate/alternates group;
  очередь перечитывается, результат пишется в статус.

Выход фазы: duplicate filter и auto-resolution настраиваются визуально и
покрывают пакетные сценарии.

## WS-09. Сервисы, репозитории, API

Python: `services/ClientGUI*`, `networking/ClientGUIHydrusNetwork`,
`ClientGUILogin`, `ClientGUIAPI`, `ClientGUIModal*`, `ClientGUIServiceSpecifier`,
`ClientGUIPanels` (IPFS).

Rust-цели: `gui/app/options.rs`, `services.rs`, `controller.rs`,
`networking/*`, `api/*`, `gui/qt/bridge.*`.

- [x] 09.1 Менеджер локальных сервисов: add/rename/delete.
- [x] 09.2 Локальные file/tag/rating/notes services.
- [x] 09.3 Client API server и access key.
- [~] 09.4 Client service editor: локальные типы, remote host/port/key,
  repository periods и file-repository storage. Tag filter/IPFS панели ждут
  09.8/09.9.
- [x] 09.5 Server-side services manager: connect, add/edit/delete, lock/unlock,
  regen info и update/nullification periods.
- [x] 09.6 Accounts: list, ban/unban, expires, message, delete content,
  edit/delete account types, permissions по content type и auto-creation.
- [~] 09.7 Update/nullification periods настраиваются из GUI; repository sync и
  petitions ещё нет.
- [~] 09.8 Tag filter: whitelist/blacklist, тест тега, сохранение в сервисе и POST в tag repository; purge tags работает для локального tag-сервиса. Petitions/commit для remote purge ещё нет.
- [ ] 09.9 IPFS и local booru shares.
- [x] 09.10 API permissions: список access keys с именами, basic permissions и search tag filter, хранение в `api permissions.json`, проверки на `/get_files/*` и `/add_tags/*`.
- [~] 09.11 Credentials manager (`logins.json`) есть; login scripts и
  использование credentials в network engine ещё нет.
- [ ] 09.12 Pending commit/forget и petition UI.
- [x] 09.13 Service specifier и review panels: выбор сервиса по типу,
  копирование ключа, review options/service info и accounts.

Выход фазы: сервисы и репозитории управляются из GUI, API-права настраиваются,
petitions доступны.

## WS-10. Экспорт

Python: `exporting/ClientGUIExport`, `panels/options/ExportingPanel`.

Rust-цели: новый `gui/app/export.rs`, `gui/qt/bridge.*`, `exporting/*`.

- [x] 10.1 Export folders: CRUD и периодический запуск.
- [x] 10.2 Export file patterns/formats.
- [x] 10.3 Review export files и выбор файлов.
- [x] 10.4 Sidecar export.
- [x] 10.5 Exporting options panel.
- [x] 10.6 Меню File → export (run all / run one).

Выход фазы: экспорт полностью доступен из GUI и покрыт тестами.

## WS-11. Обслуживание файлов и БД

Python: `panels/ClientGUIRepairFileSystemPanel`,
`ClientGUIFilesPhysicalStoragePanels`, `ClientGUIScrolledPanelsReview`
(history, maintenance, vacuum, deferred deletes, threads),
`panels/options/MaintenanceAndProcessingPanel`, `ClientGUILocalFileImports`.

Rust-цели: `gui/app/mod.rs`, `db/maintenance.rs`, `files/maintenance_executor.rs`,
`gui/app/options.rs`, `gui/qt/bridge.*`.

- [x] 11.1 `PRAGMA quick_check`.
- [x] 11.2 `VACUUM`.
- [x] 11.3 Базовые option-флаги maintenance.
- [~] 11.4 Backup/restore БД и backup location: backup через `VACUUM INTO`,
  `backup_path`/`last_backup_time` в options; restore пока вручную заменой
  `client.db` при закрытом клиенте.
- [ ] 11.5 Database locations/migration.
- [x] 11.6 Analyze database.
- [x] 11.7 Review vacuum data.
- [x] 11.8 Review deferred delete tables.
- [x] 11.9 Job scheduler и file maintenance jobs.
- [x] 11.10 Clear orphan files.
- [x] 11.11 Fix missing archive times.
- [ ] 11.12 File history chart.
- [x] 11.13 «How boned am I?».
- [ ] 11.14 Embedded metadata review.
- [x] 11.15 Repair file system: `repair file locations...` в меню database,
  автоматический показ при старте, если file/thumbnail subfolder отсутствует;
  set correct location, probe client_files/thumbnails, создание пустых
  thumbnail-папок и запись исправленных subfolders/locations в client.db.
- [ ] 11.16 Move media files/physical storage.
- [x] 11.17 Review local imports: `database -> maintenance -> review local imports...`
  открывает review-панель с парсингом путей, фильтрами, filename tagging и
  импортом только good-файлов.
- [x] 11.18 File viewing statistics.

Выход фазы: maintenance-сценарии выполняются и отображаются в GUI, а не только
через PRAGMA.

## WS-12. Options framework

Python: `panels/options/*` (~37 панелей).

Rust-цели: `gui/app/options.rs`, `client_options`, `gui/qt/bridge.*`.

- [x] 12.1 Общий диалог options.
- [x] 12.2 general.
- [~] 12.3 files and import (частично).
- [~] 12.4 media viewer (частично).
- [~] 12.5 file maintenance (частично).
- [x] 12.6 downloaders.
- [x] 12.7 network.
- [x] 12.8 client API.
- [x] 12.9 Фреймворк динамических option-панелей с поиском.
- [ ] 12.10 external programs.
- [ ] 12.11 open externally.
- [ ] 12.12 audio.
- [ ] 12.13 command palette.
- [ ] 12.14 colours.
- [ ] 12.15 connection.
- [ ] 12.16 duplicates.
- [ ] 12.17 exporting.
- [ ] 12.18 files and trash (полностью).
- [ ] 12.19 file search.
- [ ] 12.20 file sort/collect.
- [x] 12.21 file viewing statistics.
- [ ] 12.22 gui pages.
- [ ] 12.23 gui sessions.
- [ ] 12.24 importing.
- [ ] 12.25 import options.
- [ ] 12.26 maintenance and processing.
- [ ] 12.27 media viewer hovers.
- [ ] 12.28 media playback.
- [ ] 12.29 notes.
- [x] 12.30 popup notifications.
- [ ] 12.31 ratings.
- [ ] 12.32 regex favourites.
- [ ] 12.33 shortcuts.
- [ ] 12.34 speed and memory.
- [ ] 12.35 style.
- [ ] 12.36 system.
- [ ] 12.37 system tray.
- [ ] 12.38 tag editing.
- [~] 12.39 tag presentation (namespaces, underscores и decorators;
  colours/subtag order ещё нет).
- [x] 12.40 tag sort.
- [ ] 12.41 tag suggestions.
- [ ] 12.42 tag autocomplete tabs.
- [~] 12.43 thumbnails (масштаб, masonry, scroll direction; остальные поля
  панели Hydrus ещё не перенесены).
- [ ] 12.44 advanced.

Выход фазы: options покрывают все панели Hydrus, поиск по настройкам работает,
значения сохраняются.

## WS-13. UX, уведомления, системная интеграция

Python: `ClientGUIPopupMessages`, `ClientGUISystemTray`, `ClientGUISplash`,
`ClientGUIExceptionHandling`, `ClientGUICharts`, `ClientGUIAboutWindow`,
`QLocator`, `ClientGUIApplicationCommand`, `ClientGUIShortcutControls`,
`ClientGUIStyle`, `ClientGUIDragDrop`, `ClientGUITopLevelWindows*`.

Rust-цели: `gui/splash.rs`, `gui/style.rs`, `gui/shortcuts.rs`,
`gui/top_level_windows.rs`, `gui/qt/bridge.*`, новый `gui/app/notifications.rs`,
`gui/app/commands.rs`.

- [x] 13.1 Базовый menubar и status bar.
- [x] 13.2 Фиксированные горячие клавиши основного окна и viewer.
- [x] 13.3 Базовый splash-модель.
- [x] 13.4 Базовые light/dark stylesheets.
- [x] 13.5 Popup message manager и job status queue.
- [x] 13.6 System tray icon и меню.
- [x] 13.7 Splash screen UI.
- [ ] 13.8 Exception handling window.
- [ ] 13.9 Charts (file history, statistics).
- [x] 13.10 Полноценный About window.
- [~] 13.11 Command palette/locator.
- [ ] 13.12 Application commands и настраиваемые shortcut sets.
- [ ] 13.13 Mouse shortcuts и press/release/double-click/drag.
- [ ] 13.14 Drag & drop страниц/вкладок/файлов между окнами.
- [ ] 13.15 Colours/style/theming как в Hydrus.
- [ ] 13.16 Полные меню: sessions, petitions, pending, services admin,
  database backup/maintenance, export, locator, debug.
- [ ] 13.17 Menu buttons и полные context menus.
- [ ] 13.18 Восстановление top-level frames и позиций окон.

Выход фазы: системные уведомления, tray, locator, настраиваемые шорткаты и
стили работают; меню покрывает Hydrus.

## WS-14. Финальный parity-аудит

- [ ] 14.1 Пройти по 225 Python GUI-файлам и проставить флаг каждому модулю.
- [ ] 14.2 Пройти по всем меню и проверить наличие каждого action.
- [ ] 14.3 Пройти по всем options-панелям и полям.
- [ ] 14.4 Проверить сохранение/загрузку всех JSON/сессий.
- [ ] 14.5 Прогнать сценарии: поиск → viewer → теги → импорт → экспорт →
  дубликаты → maintenance.
- [ ] 14.6 Проверить error paths и пустые БД.
- [ ] 14.7 Обновить README/ARCHITECTURE и закрыть план.
- [~] 14.8 Визуальный аудит: пройти по всем экранам, снять скриншоты Rust и Python
  и устранить расхождения по «Правилу визуального паритета» (порядок контролов,
  подписи, размеры, отступы, иконки, тема).
  - сверено: главное окно (файловая страница), gallery downloader,
    url downloader, simple downloader, watcher, менеджер подписок
    (`network -> subscriptions…`), duplicates (sidebar; media panel ещё нет),
    менеджер import folders (`file -> … -> manage import folders…`).
  - осталось: export folders, диалог одной import folder, options dialogs,
    media viewer, duplicates media panel и preparation/auto-resolution.
- [x] 14.9 Переносимая сборка: Qt DLL, плагины и MSVC runtime лежат рядом с exe
  (`scripts/deploy.ps1`, `qt.conf`, `-Zip`), запуск не зависит от `PATH` и
  установленного Qt. Проверено на release-сборке с очищенным `PATH`: окно
  открывается, Qt6Core/VCRUNTIME140 грузятся из папки сборки.

---

## Порядок выполнения

1. **M1 — Shell parity:** WS-01, WS-12.9, WS-13.5–13.7, WS-13.11–13.13.
2. **M2 — Search & results:** WS-02, WS-03.
3. **M3 — Metadata & media:** WS-05, WS-04.
4. **M4 — Import & download:** WS-06, WS-07, WS-10.
5. **M5 — Duplicates & services:** WS-08, WS-09.
6. **M6 — Maintenance & UX:** WS-11, WS-12 (остальное), WS-13 (остальное).
7. **M7 — Parity audit:** WS-14.

Правило: не начинать зависимую фазу, пока её вход не закрыт хотя бы до `[~]`
с работающим happy path. Например, media viewer hovers (WS-04.7) ждут
results panel selection (WS-03.5), а thread slots (WS-07.18) ждут registry
активных network jobs.

## Как обновлять флаги

- В коммите указывать ID задачи: `WS-02.7: system size predicate`.
- `[~]` ставить, если есть UI, но нет полного поведения/сохранения/теста.
- `[x]` ставить только после DoD; при регрессе возвращать `[~]`.
- `[x]` нельзя ставить, если экран визуально расходится с Python-версией
  (см. «Правило визуального паритета»); такой экран остаётся `[~]`.
- В конце каждой фазы обновлять таблицу «Текущий статус» и README.
