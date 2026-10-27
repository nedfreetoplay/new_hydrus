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
4. `cargo build -p hydrus_client` без предупреждений, тесты зелёные.
5. Обновлён этот файл и, при необходимости, `docs/ARCHITECTURE.md`.

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
| WS-10 | Экспорт | `[ ]` | P1 | L | WS-06 |
| WS-11 | Обслуживание файлов и БД | `[~]` | P1 | L | WS-09 |
| WS-12 | Options framework | `[~]` | P1 | XL | — |
| WS-13 | UX, уведомления, системная интеграция | `[~]` | P2 | L | WS-01, WS-12 |
| WS-14 | Финальный parity-аудит | `[ ]` | P0 | M | все |

Текущее покрытие: базовые сценарии есть, но из ~225 GUI-модулей Python прямой
аналог имеют единицы. До полной идентичности — все фазы ниже.

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
- [ ] 02.12 Системные предикаты: duplicate relationships, similar files.
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
- [ ] 03.7 Masonry layout, scroll direction, thumbnail sizing options.
- [ ] 03.8 Loading panel/progress и file count.
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
- [ ] 04.7 Hover frames: top archive/delete/filter, top-right, tags, notes,
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
- [ ] 05.15 Tag sorting options.
- [ ] 05.16 Tag actions: copy/paste/apply к выделению.
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
- [ ] 06.6 Import options framework: file filtering, location, notes,
  presentation, prefetch, tags, external programs.
- [ ] 06.7 Default/specific import options containers.
- [ ] 06.8 Filename tagging options.
- [ ] 06.9 Local import review panel.
- [ ] 06.10 File seed cache UI.
- [ ] 06.11 Gallery seed log UI.
- [ ] 06.12 Importer sidebars для HDD/gallery/watcher/simple/URLs.
- [x] 06.13 «Check all» и полное меню import folders.
- [ ] 06.14 Parsing scripts/legacy import scripts.
- [ ] 06.15 Review local imports и repair import locations.

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
- [ ] 07.12 Watcher editor/review.
- [ ] 07.13 Subscription editor/query editor.
- [ ] 07.14 Gallery seed log.
- [ ] 07.15 Downloader export/import.
- [ ] 07.16 Login credentials и login scripts.
- [ ] 07.17 Per-downloader import options.
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
- [ ] 08.4 Duplicate filter page/sidebar: search context и пороги.
- [ ] 08.5 Batch size/preparation panel.
- [ ] 08.6 Auto-resolution editor: AND/OR, metadata conditions, pair selectors.
- [ ] 08.7 Auto-resolution preview/review и pending/taken/denied списки.
- [ ] 08.8 Content merge options по tag/rating/file services.
- [ ] 08.9 Thumbnail pair list и сортировка.
- [ ] 08.10 Полные duplicate actions/menu.

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
- [ ] 09.4 Client service editor со всеми sub-panels.
- [ ] 09.5 Server-side services manager.
- [ ] 09.6 Accounts и account types.
- [ ] 09.7 Repository sync, update periods, petitions.
- [ ] 09.8 Tag filter и purge tags.
- [ ] 09.9 IPFS и local booru shares.
- [ ] 09.10 API permissions capture/edit.
- [ ] 09.11 Login credentials/scripts.
- [ ] 09.12 Pending commit/forget и petition UI.
- [ ] 09.13 Service specifier и review panels.

Выход фазы: сервисы и репозитории управляются из GUI, API-права настраиваются,
petitions доступны.

## WS-10. Экспорт

Python: `exporting/ClientGUIExport`, `panels/options/ExportingPanel`.

Rust-цели: новый `gui/app/export.rs`, `gui/qt/bridge.*`, `exporting/*`.

- [ ] 10.1 Export folders: CRUD и периодический запуск.
- [ ] 10.2 Export file patterns/formats.
- [ ] 10.3 Review export files и выбор файлов.
- [ ] 10.4 Sidecar export.
- [ ] 10.5 Exporting options panel.
- [ ] 10.6 Меню File → export (run all / run one).

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
- [ ] 11.4 Backup/restore БД и backup location.
- [ ] 11.5 Database locations/migration.
- [x] 11.6 Analyze database.
- [x] 11.7 Review vacuum data.
- [x] 11.8 Review deferred delete tables.
- [ ] 11.9 Job scheduler и file maintenance jobs.
- [ ] 11.10 Clear orphan files.
- [ ] 11.11 Fix missing archive times.
- [ ] 11.12 File history chart.
- [x] 11.13 «How boned am I?».
- [ ] 11.14 Embedded metadata review.
- [ ] 11.15 Repair file system.
- [ ] 11.16 Move media files/physical storage.
- [ ] 11.17 Review local imports.
- [ ] 11.18 File viewing statistics.

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
- [ ] 12.21 file viewing statistics.
- [ ] 12.22 gui pages.
- [ ] 12.23 gui sessions.
- [ ] 12.24 importing.
- [ ] 12.25 import options.
- [ ] 12.26 maintenance and processing.
- [ ] 12.27 media viewer hovers.
- [ ] 12.28 media playback.
- [ ] 12.29 notes.
- [ ] 12.30 popup notifications.
- [ ] 12.31 ratings.
- [ ] 12.32 regex favourites.
- [ ] 12.33 shortcuts.
- [ ] 12.34 speed and memory.
- [ ] 12.35 style.
- [ ] 12.36 system.
- [ ] 12.37 system tray.
- [ ] 12.38 tag editing.
- [ ] 12.39 tag presentation.
- [ ] 12.40 tag sort.
- [ ] 12.41 tag suggestions.
- [ ] 12.42 tag autocomplete tabs.
- [ ] 12.43 thumbnails.
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
- [ ] 13.5 Popup message manager и job status queue.
- [x] 13.6 System tray icon и меню.
- [ ] 13.7 Splash screen UI.
- [ ] 13.8 Exception handling window.
- [ ] 13.9 Charts (file history, statistics).
- [ ] 13.10 Полноценный About window.
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
results panel selection (WS-03.5), а per-downloader import options (WS-07.17)
ждут import options (WS-06.6).

## Как обновлять флаги

- В коммите указывать ID задачи: `WS-02.7: system size predicate`.
- `[~]` ставить, если есть UI, но нет полного поведения/сохранения/теста.
- `[x]` ставить только после DoD; при регрессе возвращать `[~]`.
- В конце каждой фазы обновлять таблицу «Текущий статус» и README.
