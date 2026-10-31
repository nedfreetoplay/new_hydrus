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
7. Задача не добавила новых JSON-швов через cxx; тронутые швы переведены на
   типизированный мост (R1) либо в задаче записано, почему нет.
8. Чистая логика текста, предикатов и метрик тронутого экрана живёт в Rust (R2)
   и покрыта тестом; C++ только применяет готовые значения.
9. Ни один `.cpp`/`.hpp` в `src/gui/qt` не превышает 600 строк (R3).

## Правила фасада Qt

Обязательны и для новых задач, и для доработки уже написанных экранов.
Не отменяют «Правило визуального паритета», а дополняют его.

### R1. Типизированный мост вместо JSON-строк

Сейчас через cxx ходят JSON-строки: 59 вызовов `parse_object(` в C++ и 62
`json_payload(`, 114 разных action-строк, 68 из 79 вариантов `UiCommand` несут
`String`. Для новых фич это запрещено.

- Передача Rust → Qt: вариант `UiCommand` содержит типизированную структуру из
  `gui/qt/api.rs` (`#[cxx::bridge]`), а не `serde_json::Value`/`&str`.
  `panels.rs` больше не сериализует в JSON.
- Передача Qt → Rust: вместо `notify(QString action, QString target, QString
  payload)` — типизированное событие (`UiEvent`) или отдельная функция на
  команду.
- В C++ нет `QJsonObject` в сигнатурах методов: приходит структура, поля
  читаются напрямую.
- Исключение — данные принципиально произвольной формы (дерево страниц, словари
  опций, которые и в Python живут как `dict`). Но и там JSON остаётся внутри
  одного поля структуры, а не вместо типа.

Инвариант: число JSON-швов не растёт. На каждом экране, который трогаем по
плану, швы уменьшаются.

### R2. Чистая логика раскладки живёт в Rust

C++ делает только `setText`/`setEnabled`/`setVisible`/`resize`/`setColumnWidth`
по готовым значениям. В `gui/app/**` обязано уходить всё, что является функцией
состояния:

- тексты: форматирование размеров/времени/дат, склонения (`1 file` / `2 files`),
  статусы, empty-state, подписи кнопок;
- предикаты: доступность кнопок, видимость секций и вкладок, включённость полей;
- метрики: ширины колонок, высоты строк, отступы, минимальные размеры, порядок
  и состав меню;
- арифметика: проценты, лимиты, счётчики, дедупликация, сортировка.

Критерий: если это записывается как `fn f(&State) -> String/bool/i32/Size`,
значит это Rust и unit-тест. В C++ остаётся только координация QObject
(`connect`, `popup`, `exec`).

### R3. Один файл — одна ответственность, ≤ 500 строк

Для `src/gui/qt/**`: цель 500 строк, жёсткий предел 600. Не влезает — делится по
экрану или слою (layout / state / actions), а не «по первым N строк».
`bridge.cpp`/`bridge.hpp` разбираются на файлы по карте WS-15.1; общий
`bridge.hpp` остаётся только точкой подключения cxx.

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
  highlight, полноценные multi-query/GUG-редакторы,
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
  поведение disabled-панелей. Заодно меню `pending`
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
  реальный multi-job парсинг.
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
  групповой режим.
- 2026-10-31 — media results panel страницы duplicates реализован (WS-08 /
  WS-14.8): правая часть теперь использует существующий thumbnail grid
  (`HydrusThumbnailGrid`) и показывает уникальные хэши пар в порядке
  отфильтрованной очереди; статус `media_empty_status` / `media_count_status`,
  двойной клик открывает viewer, действия same/not duplicates/skip/delete both
  сохранены. Скриншоты: `docs/images/duplicates_media_panel.png` (Rust) и
  `docs/images/python_duplicates_media_panel.png` (Python). Осталось:
  preparation/auto-resolution tabs и групповой режим остаются заглушками.
- 2026-10-31 - WS-08.5 preparation tab: pure logic for batch splitting, progress percentages, status strings, auto-commit decisions (688 unit tests). Preparation tab is no longer a placeholder; full Python parity (maintenance discovery) not reached, status stays [~].
- 2026-10-31 - WS-08.6 auto-resolution sidebar tab: replaced placeholder with rule list (name/progress/status), refresh/cog, pause-play/review/edit buttons wired to existing dialogs; pure helpers for status labels/enabled-state (689 unit tests). Visual parity partial; status stays [~] (WS-14.8).
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
  `docs/images/python_import_folders_dialog.png` (Python). Осталось: подменю check-now по папкам.
- 2026-10-31 — диалог редактирования одной import folder сверен с живым Python v692. `add`, `edit` и двойной клик в менеджере import folders теперь открывают панель Python: заголовок `edit import folder`, блок `folder options` (`name:`, `folder path:` + `browse`, `search subdirectories:`, `currently paused (if set, will not ever do any work):`, `check regularly?:`, `check period:` с кнопкой периода, `recent modified time skip period:` с кнопкой периода с секундами, `check on manage dialog ok:`, `show a popup while working:`, `publish presented files to a popup button:` и `publish presented files to a page:`), `source file actions` (`when a file imports successfully:` и т.д. с combo `delete the source file` / `leave the source file alone, do not reattempt it` / `move the source file` и путь назначения, включаемый только при `move`) и `metadata import` (`filename tagging:` со списком и `add`/`edit`/`remove`, `sidecar importing:`). Периоды редактируются отдельным диалогом дней/часов/минут (и секунд для skip), минимум 3 минуты и 1 секунда. Сохранение идёт через прежний `import_folder_save`: пустой путь даёт `You must enter a path to import from!`, пустые пути move-назначений дают соответствующие сообщения Python, имя проходит через `GetNonDupeName` (`import folder (1)`). Модель получила `check_now`; `check_regularly` выключает кнопку периода. Скриншоты: `docs/images/import_folder_editor.png` (Rust) и `docs/images/python_import_folder_editor.png` (Python). Осталось: кнопка `import options` (specific container) не выведена, `review currently cached import paths` (file seed cache) и `sidecar importing` (metadata routers) не реализованы; иконки — реальные из static/ (WS-14.8).
- 2026-10-31 — менеджер export folders сверен с живым Python v692. В Python это
  не страница, а диалог `file -> import/export folders -> manage export
  folders…` с заголовком `edit export folders` (`EditExportFoldersPanel`).
  Rust теперь открывает немодальный диалог из того же пункта меню: вступление
  `Here you can set the client to regularly export a certain query to a
  particular location.`, таблица `name/path/type/query/period/phrase/
  recent error?` (7 колонок; `path` по умолчанию шириной в 3 символа, как
  зарегистрировано в Python, последняя колонка растягивается), сортировка по
  имени, выделение строки и двойной клик. Справа `add/edit/delete`, внизу
  `apply/cancel`; `edit`/`delete` выключены без выделения. `type` показывает
  `regular`/`synchronise` (+ ` and deleting from the client!`), `period` —
  `1 day`/`2 hours`/`not running regularly` по Python-формату. После
  add/edit/delete диалог обновляется командой `UpdateExportFoldersDialog`.
  Страница export folders осталась только для старых сессий: из меню и
  chooser страниц она убрана. Скриншоты:
  `docs/images/export_folders_dialog.png` (Rust) и
  `docs/images/python_export_folders_dialog.png` (Python). Осталось: подменю
  `run export folder now`/пауза, цвет выделения строки (нативный синий против
  бледно-голубого в Python) и отдельная колонка индикатора сортировки.
- 2026-10-31 — диалог одной export folder сверен с живым Python v692
  (`EditExportFolderPanel`). `add`, `edit` и двойной клик в менеджере export
  folders снова открывают не старую Rust-форму, а панель Python: заголовок
  `edit export folder`, блоки `name and location` (`name:`, `folder path:` +
  `browse`), `type of export` (help-текст `regular - ... be careful!`, combo
  `regular`/`synchronise`, красные `trash files in hydrus client after export:`
  и `EXPERIMENTAL: export symlinks`), `export period` (`run regularly?:` +
  кнопка `1 day`, `show popup when working regularly?:`, `run on dialog ok:`),
  `query to export`, `filenames` (`{hash}` и `pattern shortcuts`, который
  копирует термы в буфер), `sidecar exporting` (тот же help-текст,
  `overwrite all sidecars on next run:`, `DANGEROUS: always overwrite all
  sidecars on every run:`, кнопка `no sidecars`, `update test example files`) и
  `apply`/`cancel` справа внизу. Диалог растягивается по содержимому до
  доступной высоты, как Python `DialogThatResizes`.
  Сохранение идёт через прежний `export_folder_save`: пустое имя, пустой путь и
  битая фраза дают сообщения Python (`You must enter a folder path to export
  to!`, `Could not parse that export phrase! ...`), имя проходит через
  `GetNonDupeName` (`export folder (1)`). `synchronise` выключает и снимает
  `trash...`, `run regularly?` выключает кнопку периода и popup. Модель
  получила `export_symlinks`, `show_working_popup`, `run_now` и
  `always_overwrite_sidecars`: symlink-экспорт действительно создаёт ссылки,
  `run on dialog ok` ставит папку в очередь, `show popup` даёт уведомление,
  always-overwrite перезаписывает сайдкары на каждом запуске. Скриншоты:
  `docs/images/export_folder_editor.png` (Rust) и
  `docs/images/python_export_folder_editor.png` (Python). Осталось: блок
  query — упрощённый редактор предикатов (нет автодополнения, кнопок
  `include current/pending tags`, `searching immediately`/`OR`,
  `my files`/`all known tags` и вкладок `results`/`favourites`/`children`;
  star/lock сообщают `not implemented yet`); кнопка сайдкаров — выбор из трёх
  форматов вместо редактора metadata routers, а `update test example files`
  только считает подходящие файлы; иконки paste/star/clear/lock — реальные из static/
  приближения.

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
| WS-10 | Экспорт | `[~]` | P1 | L | WS-06 |
| WS-11 | Обслуживание файлов и БД | `[~]` | P1 | L | WS-09 |
| WS-12 | Options framework | `[~]` | P1 | XL | — |
| WS-13 | UX, уведомления, системная интеграция | `[~]` | P2 | L | WS-01, WS-12 |
| WS-14 | Финальный parity-аудит | `[ ]` | P0 | M | все |
| WS-15 | Фасад: разбиение, типизация, раскладка | `[ ]` | P0 | L | — |

Текущее покрытие: базовые сценарии есть, но из ~225 GUI-модулей Python прямой
аналог имеют единицы. До полной идентичности — все фазы ниже.

Отдельный долг: ранее закрытые экраны проверялись на функционал, но не на внешний
вид. Их нужно прогнать по «Правилу визуального паритета» и только потом оставлять
`[x]`; до этого такие пункты считать `[~]`.

WS-15 выполняется раньше остальных фаз: пока фасад не разобран по файлам и не
типизирован, каждая новая панель снова раздувает `bridge.cpp` и добавляет
JSON-швы.

---

## WS-15. Фасад: разбиение, типизация, раскладка

Технический долг Qt-фасада. Правила R1–R3 обязательны для всех задач плана;
этот раздел переводит на них уже написанный код.

Основание (2026-10-31): `bridge.cpp` — 18 721 строка, `bridge.hpp` — 1 395,
`bridge.rs` — 344 (109 функций). 121 JSON-шов (59 `parse_object(`, 62
`json_payload(`), 114 action-строк, 79 вариантов `UiCommand`, из них 68 несут
`String`.

### 15.1 Разбиение bridge.cpp / bridge.hpp  `[ ]` P0

Карта переноса (строки текущего `bridge.cpp`; суммы приблизительные, цель —
каждый файл ≤ 600 строк):

| Файл | Что | Строк |
| --- | --- | --- |
| `support/strings.cpp` | `to_qstring`, `parse_*`, `json_payload` | ~95 |
| `support/icons.cpp` | `hydrus_small_icon` + таблица имён файлов | ~330 |
| `support/widgets.cpp` | `make_static_box`, `make_icon_button`, `make_on_off_button`, `field_row` | ~180 |
| `support/pairs.cpp` | `pair_hash_item`, `fill_pair_table`, `install_pair_list_actions` | ~160 |
| `support/sort_options.cpp` | `hydrus_sort_options`, системные предикаты | ~120 |
| `canvas.cpp` | `HydrusCanvas` | 215 |
| `thumbnail_grid.cpp` / `thumbnail_grid_dnd.cpp` | сетка + drag&drop и контекстное меню | ~270 / ~300 |
| `metadata_panel.cpp` | `HydrusMetadataPanel` | 355 |
| `autocomplete.cpp` | `HydrusAutocompletePopup` | 199 |
| `query_page_sidebar.cpp` / `query_page_results.cpp` / `query_page_state.cpp` | `HydrusQueryPage` | ~400 / ~400 / ~390 |
| `downloader_page_layout.cpp` / `_gallery.cpp` / `_importers.cpp` / `_state.cpp` | `HydrusDownloaderPage` | 4 файла |
| `viewer.cpp` | `HydrusViewerWindow` | 379 |
| `notebook.cpp` | `HydrusTabBar` + `HydrusNotebook` | 376 |
| `duplicates_page_sidebar.cpp` / `duplicates_page_media.cpp` | `HydrusDuplicatesPage` | ~450 / ~430 |
| `auto_resolution_comparators.cpp` / `_rule.cpp` / `_rules.cpp` / `_review.cpp` / `_merge.cpp` / `_custom.cpp` | WS-08 диалоги | 277…340 |
| `services_helpers.cpp` / `services_dialog.cpp` / `services_server.cpp` | WS-09 | 3 файла |
| `network_dialog.cpp` / `network_tables.cpp` | WS-09 сеть | 2 файла |
| `import_options_dialog.cpp` / `import_options_editor.cpp` / `import_options_read.cpp` | 1 188 строк | 3 файла |
| `import_folder_editor.cpp` / `import_folder_tagging.cpp` | `edit import folder` | 2 файла |
| `export_folder_editor.cpp` / `export_folder_sidecar.cpp` | `edit export folder` | 2 файла |
| `main_window_shell.cpp` / `main_window_shortcuts.cpp` / `main_window_dialogs.cpp` | `HydrusMainWindow` | 3 файла |
| `options_dialog.cpp`, `url_class_dialog.cpp`, `tag_relations_dialog.cpp`, `seed_cache_dialog.cpp`, `command_palette.cpp`, `or_predicate_dialog.cpp`, `page_manager.cpp`, `manage_tags_dialog.cpp`, `timestamps_dialog.cpp`, `notifications_dialog.cpp`, `about_notes_stats.cpp`, `local_import_review.cpp`, `local_import_tagging.cpp`, `filename_tagging.cpp`, `repair_locations.cpp`, `tag_filter.cpp`, `api_permissions.cpp`, `account_types.cpp`, `orphan_files.cpp`, `database_backup.cpp`, `export_review.cpp`, `page_chooser.cpp` | по файлу на диалог | ≤ 350 каждый |
| `bridge_api.cpp` | свободные `hydrus_*` функции cxx | 498 |

Итог: ≈55 файлов вместо одного. `HydrusMainWindow` остаётся одним классом; его
методы лежат в файле своего экрана.

- [x] 15.1.1 Общие хелперы → `gui/qt/support/*`.
- [ ] 15.1.2 Страницы и экраны → файлы по карте.
- [ ] 15.1.3 Диалоги → по файлу на диалог.
- [ ] 15.1.4 `bridge.hpp` → заголовки по тем же группам, `bridge.hpp` — агрегатор.
  Проверить, что cxx принимает несколько `include!` в одном модуле; если нет —
  отдельные модули `ffi_pages` / `ffi_dialogs`.
- [ ] 15.1.5 `hydrus_client/build.rs`: `.cpp_files([...])` на новый набор файлов.
- [ ] 15.1.6 Проверка: ни один файл > 600 строк, сборка без предупреждений, тесты
  зелёные, поведение не изменилось.

Правила переноса: в одном коммите только перемещение кода, без правок логики;
после каждого шага сборка и тесты; проверять `git diff -M` на случайные изменения.
Перед правкой `bridge.cpp` трогать `src/gui/qt/bridge.rs`, иначе cxx не пересоберёт
C++.

### 15.2 Типизированный мост  `[ ]` P0

- [ ] 15.2.1 `gui/qt/api.rs`: общие структуры (`PageNode`, `PageState`,
  `MediaItem`, `StatusBar`, `MenuNode`, `OptionsField`, …), один тип на шов.
- [ ] 15.2.2 `UiCommand`: 68 `String`-вариантов → типизированные, по экранам,
  начиная с тех, что правим сейчас.
- [ ] 15.2.3 `notify(action, target, payload)` → `UiEvent` или отдельная функция
  на команду.
- [ ] 15.2.4 Удалить `parse_object`/`json_payload` из C++ и `serde_json` с шва.
- [ ] 15.2.5 Правило на будущее: новый экран не добавляет `&str`-payload.

### 15.3 Чистая логика раскладки в Rust  `[ ]` P1

Что обязано уехать в Rust уже сейчас (найденные примеры):

- `"just now"` / `"%1 minutes ago"` — строки 4437 и 4492, продублированы;
- `"%1 successful, %2 failed"` — 4541 и далее, собирается в C++;
- ширины колонок `{56, 78, 30, 30, 56, 64, 56}`, `{80, 38, 38, 48, 38, 46}`,
  `{132, 20, 79, 106, 106, 132}` — 3162, 3550, 5649;
- `pretty_delta` — 15656 и 16367, две копии одного форматтера;
- `qBound(25, scale, 400)` — 665.

- [ ] 15.3.1 Аудит: выписать все C++-функции и лямбды, которые являются чистыми.
- [ ] 15.3.2 Перенос форматирования: размеры, длительности, даты, склонения,
  статусы, empty-state.
- [ ] 15.3.3 Перенос предикатов `setEnabled`/`setVisible`.
- [ ] 15.3.4 Перенос метрик: ширины колонок, высоты строк, отступы.
- [ ] 15.3.5 На каждый перенос — unit-тест в Rust; C++ вызывает готовое значение.

Выход фазы: `bridge.cpp` больше нет, каждый файл фасада ≤ 600 строк, новые экраны
не добавляют JSON-швов, форматирование и предикаты экранов покрыты Rust-тестами.

## WS-01. Каркас страниц и сессии

Python: `pages/ClientGUIPages*`, `ClientGUIPagesTreeModel/View`,
`ClientGUIPageManager`, `ClientGUISession*`, `panels/options/GUIPagesPanel`,
`GUISessionsPanel`.

Rust-цели: `gui/app/model.rs`, `gui/app/session.rs`, `gui/pages.rs`,
`gui/app/render.rs`, `gui/qt/` (файлы по карте WS-15.1).

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
`gui/qt/` (файлы по карте WS-15.1), `db/tag_search`, `db/files_metadata`.

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

Rust-цели: `gui/app/query.rs`, `gui/app/render.rs`, `gui/qt/` (файлы по карте WS-15.1),
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

Rust-цели: `gui/qt/` (файлы по карте WS-15.1), `gui/app/render.rs`, `gui/app/metadata.rs`,
`client_options`.

- [~] 04.1 Статичные изображения, prev/next, fit, zoom, rotate, fullscreen.
- [~] 04.2 Metadata sidebar в viewer.
- [x] 04.3 Базовые media options (zoom step, recenter, always-on-top, frameless).
- [ ] 04.4 MPV backend для видео.
- [ ] 04.5 QtMediaPlayer fallback и audio playback.
- [~] 04.6 GIF/WebP/animation bar и pause/play.
- [~] 04.7 Hover frames: top archive/delete/filter, top-right, tags, notes,
  ratings.
- [ ] 04.8 Duplicate filter canvas.
- [~] 04.9 Zoom modes menu, pan modes, background colour/checkerboard.
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
`gui/app/options.rs`, `gui/qt/` (файлы по карте WS-15.1), `db/tag_display`, `db/tag_relations`.

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
`gui/qt/` (файлы по карте WS-15.1), `importing/*`.

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
`networking/client_networking_url_class.rs`, `gui/qt/` (файлы по карте WS-15.1).

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
`gui/app/model.rs`, `gui/qt/` (файлы по карте WS-15.1), `db/duplicates`.

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
`networking/*`, `api/*`, `gui/qt/` (файлы по карте WS-15.1).

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

Rust-цели: новый `gui/app/export.rs`, `gui/qt/` (файлы по карте WS-15.1), `exporting/*`.

- [~] 10.1 Export folders: CRUD и периодический запуск. Менеджер
  (`file -> import/export folders -> manage export folders…`) и диалог одной
  папки (`edit export folder`) сверены с Python; блок query остаётся
  упрощённым редактором предикатов без автодополнения, поэтому пункт
  остаётся частичным.
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
`gui/app/options.rs`, `gui/qt/` (файлы по карте WS-15.1).

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

Rust-цели: `gui/app/options.rs`, `client_options`, `gui/qt/` (файлы по карте WS-15.1).

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
- [x] 12.16 duplicates.
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

### WS-12 shell/general audit (2026-10-31)

- WS-12.1/12.2/12.9: options dialog shell and `general` panel audited against
  live Python v692. Shell: title `options`, search box placeholder
  `Search options... (Experimental!)` with top/bottom placement driven by
  `options_search_bar_top_of_window`, buttons `apply`/`cancel` with Hydrus accept/cancel
  object names, help tooltips on every general field. General tab rebuilt from a field
  registry (`GENERAL_FIELDS`) with verbatim labels/help from Python panels (gui pages,
  thumbnails, media playback, popup, advanced). Added `remember_options_window_panel`
  and `options_search_bar_top_of_window` fields. Pure helpers: `panel_order`,
  `panel_title`, `general_field`, `general_labels`, `general_help`,
  `format_field_value` with unit tests.
- Screenshots: docs/images/options_general.png (Rust), docs/images/python_options_general.png (Python).
- Remaining differences: Rust keeps the consolidated tab model instead of Python's
  left panel list/completer; other ~30 panels untouched; icons approximate.
- Tests: 697 unit + 2 integration + 1 ignored.

### WS-12.16 duplicates audit (2026-10-31)

- WS-12.16: duplicates options panel added to the consolidated options dialog as a
  `duplicates` tab, built from a `DUPLICATE_FIELDS` registry with verbatim labels/help
  from Python `DuplicatesPanel.py`. Fields: two checkboxes, two intro texts, ten score
  spinners (-100..100), max batch size (5..1024), auto-commit noneable spinner
  (1..50, "no, always confirm"), two noneable background intensity spinners
  (1..9, "do not change"), draw transparency checkbox. All fields save/load through
  `client_options` (booleans/integers/noneable_integers).
- Qt dialog extended to render `noneable_int` fields (spinbox + none checkbox) and to
  honour per-field min/max ranges; readonly labels now wrap.
- Pure helpers: `duplicates_field`, `duplicates_labels`, `duplicates_help`,
  `duplicates_range`, `duplicates_none_phrase`, `duplicates_clamp`,
  `duplicates_clamp_noneable` with unit tests.
- Screenshots: docs/images/options_duplicates.png (Rust), docs/images/python_options_duplicates.png (Python).
- Tests: 704 unit + 2 integration + 1 ignored.

## WS-13. UX, уведомления, системная интеграция

Python: `ClientGUIPopupMessages`, `ClientGUISystemTray`, `ClientGUISplash`,
`ClientGUIExceptionHandling`, `ClientGUICharts`, `ClientGUIAboutWindow`,
`QLocator`, `ClientGUIApplicationCommand`, `ClientGUIShortcutControls`,
`ClientGUIStyle`, `ClientGUIDragDrop`, `ClientGUITopLevelWindows*`.

Rust-цели: `gui/splash.rs`, `gui/style.rs`, `gui/shortcuts.rs`,
`gui/top_level_windows.rs`, `gui/qt/` (файлы по карте WS-15.1), новый `gui/app/notifications.rs`,
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
    менеджер import folders (`file -> … -> manage import folders…`), менеджер
    export folders (`file -> … -> manage export folders…`) и диалог одной
    export folder (`edit export folder`).
  - осталось: диалог одной import folder, options dialogs, media viewer,
    duplicates media panel и preparation/auto-resolution.
- [x] 14.9 Переносимая сборка: Qt DLL, плагины и MSVC runtime лежат рядом с exe
  (`scripts/deploy.ps1`, `qt.conf`, `-Zip`), запуск не зависит от `PATH` и
  установленного Qt. Проверено на release-сборке с очищенным `PATH`: окно
  открывается, Qt6Core/VCRUNTIME140 грузятся из папки сборки.

---

## Порядок выполнения

0. **M0 — Фасад:** WS-15.1 (разбиение), затем WS-15.2 (типизация) и WS-15.3
   (раскладка) — по мере касания экранов. Пока M0 не закрыт, новый экран не
   добавляет строки в общий файл и новые JSON-швы.
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
- `[x]` не ставится, если экран добавил JSON-швы, оставил форматирование/предикаты
  в C++ или раздул файл сверх 600 строк (R1–R3).
- В конце каждой фазы обновлять таблицу «Текущий статус» и README.

## WS-14.8 media viewer audit
- 2026-10-31: visual audit against Python v692; hover panels/background/status text reviewed. WS-04.1/04.2/04.7/04.9/04.13 marked [~]. Screenshots: docs/images/media_viewer.png, docs/images/python_media_viewer.png.
- 2026-10-31 — WS-14.8: vendored Hydrus static/ (315 files, v692) into repo root; resolver hydrus_client/src/gui/app/static_assets.rs (env HYDRUS_STATIC_DIR, <exe>/static, <repo>/static); hydrus_small_icon/icon_for now load real files with painted fallback; deploy.ps1 copies static/ and qsvg plugin. Tests 708+2+1ignored. Screenshots: docs/images/icons_real_main_window.png.
