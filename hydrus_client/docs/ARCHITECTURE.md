# Архитектура `hydrus_client`

Порт повторяет подсистемы Python-клиента, но без глобального
`CG.client_controller`: контроллер передаётся явно.

## Слои

```text
hydrus_core            константы, serialize, БД-примитивы, файлы, изображения
  ↑
hydrus_client::client_*    константы, данные, пути, время, потоки, опции
  ↑
hydrus_client::metadata / media / files    теги, MediaResult, хранилища
  ↑
hydrus_client::db          ClientDB (сейчас на DBBase), сервисы/hashes/files_info
  ↑
hydrus_client::networking / importing / parsing / duplicates
  ↑
hydrus_client::controller  boot модели, локальные сервисы, сохранение опций
  ↑
hydrus_client::gui::app    состояние клиента: дерево страниц, поиск, импорт,
                           теги, вьюеры, команды UI
  ↑
hydrus_client::gui::qt     Qt-виджеты и C++-фасад, только рендер и ввод
```

## GUI

`gui::app` — единственный владелец состояния. Он не знает про Qt и не вызывает
виджеты напрямую. Событие из C++ приходит в `app::handle_action`, состояние
меняется, затем возвращается список `UiCommand` (`BuildPages`, `UpdatePage`,
`UpdateDownloader`, `UpdateDuplicates`, `OpenViewer`, `Status`, …).
`gui::qt::panels` исполняет эти команды над `HydrusMainWindow`.

Долгие операции не выполняются в UI-потоке. `gui::app::worker::AsyncRuntime`
держит фоновый диспетчер и канал событий: импорт файлов, прямые URL и разбор
gallery/watcher/subscription страниц ставятся в очередь. Скачивание файлов
раскладывается в per-domain пулы (scoped threads), а глобальный `PoolGate`
ограничивает одновременные запросы. `app::handle_tick` вызывается Qt-таймером
каждые 120 мс, забирает накопившиеся события и обновляет страницы/статус-бар.
Фоновые потоки получают `DBBase` и `ClientFilesManager` копией, поэтому им не
нужен `Controller` и они не пересекаются с borrow-ами UI.

Это важно для модальных диалогов: пока открыт tag-dialog или options, Rust-состояние
уже отпущено, поэтому повторный вызов `app_action` из диалога не вызывает
deadlock/повторный borrow. Менеджеры сервисов и сети немодальные: команда
`ServicesDialog`/`NetworkDialog` обновляет уже открытое окно, а не открывает второе.

C++-фасад (`gui/qt`) использует обычные Qt Widgets. Через границу cxx идут либо
типизированные структуры (статус-бар, about, статистика просмотров — см. шаблон
ниже), либо пока JSON: дерево страниц, состояние query/downloader/
duplicates/subscriptions, меню и данные вьюера. Никакой бизнес-логики
в C++ нет.

### Структура `gui/qt` (WS-15.1)

Монолит `bridge.cpp`/`bridge.hpp` разобран: `bridge.cpp` удалён, осталось
65 `.cpp` + 40 `.hpp` (+11 в `support/`). Каждый файл ≤ 600 строк (жёсткий
предел R3), один файл — одна ответственность (экран или слой).

- `support/` — общий клей: строки/JSON (`strings`), иконки (`icons`), виджеты
  (`widgets`), пары дубликатов (`pairs`), сортировки и системные предикаты
  (`sort_options`); `support.hpp` — их агрегатор.
- Страницы: `canvas`, `notebook`, `viewer`, `autocomplete`, `metadata_panel`,
  `thumbnail_grid(+_dnd)`, `query_page_(sidebar|results|state)`,
  `downloader_page_(layout|gallery|importers|state)`,
  `duplicates_page_(sidebar|media)`.
- Диалоги: по файлу на диалог (`options_dialog`, `services_*`, `network_*`,
  `import_options_*`, `import_folder_*`, `export_folder_*`, …).
- Менеджеры: `subscriptions_page` / `subscriptions_dialog`,
  `import_folders_manager`, `export_folders_manager` (диалог + legacy-страница).
- `main_window_(shell|shortcuts|dialogs)` — методы `HydrusMainWindow` по темам;
  сам класс один, объявлен в `main_window.hpp`, методы лежат в файле экрана.
- `bridge_api.cpp` / `bridge_api.hpp` — свободный клей cxx
  (`hydrus_*`, splash, qapp); `screens.hpp` + `bridge.hpp` — тонкие агрегаторы.

Граница cxx — один модуль `#[cxx::bridge] mod ffi` в `gui/qt/api.rs` с одним
`include!("bridge.hpp")`: агрегатор подтягивает все заголовки, поэтому
разделять мост на `ffi_pages`/`ffi_dialogs` не понадобилось. Rust-шимы
`app_action`/`app_tick` живут рядом с мостом в том же файле; `bridge.rs`
остался только bootstrap (`run_application`). Новые экраны
добавляют свой `.hpp`/`.cpp` и строчку в `build.rs` (`.cpp_files([...])`),
а не код в общий файл.

Правила R1–R3 в применении к `gui/qt`: R1 — новые швы только типизированные
(варианты `UiCommand` со структурами из `gui/qt/api.rs`, без `QJsonObject` в
сигнатурах; число JSON-швов не растёт); R2 — чистые форматирование/предикаты/
метрики живут в `gui/app/**` с unit-тестами, C++ только применяет значения;
R3 — новый файл не больше 600 строк, иначе делится по экрану или слою.

### Как добавить новое UI-действие (WS-15.2.4)

1. Добавить вариант в `enum UiAction` в `hydrus_client/src/gui/qt/api.rs`
   (`#[repr(i32)]`, имя — PascalCase от snake_case ключа: `close_page` → `ClosePage`),
   а также строку в `UiAction::as_str` и (если нужно для данных) в
   `crate::gui::app::action_from_str`.
2. Добавить ветку в `AppState::dispatch` (или соответствующий `handle_*`) в
   `hydrus_client/src/gui/app/mod.rs`, переиспользуя существующее ядро.
   Строковое сопоставление действий внутри диспетчера запрещено: `dispatch`
   принимает `UiAction` и матчит варианты напрямую. `ui_action_from_str`
   удалён; `as_str` остаётся только для логов/тестов.
3. Добавить строковый ключ в C++-меню/команды не нужно: `MenuEntry.action` и
   `UiCommand::Defer/PromptText/ConfirmAction.action` несут `UiAction`.
   В C++ вызывать `notify(UiAction::X, target, payload)` или
   `app_action(window, UiAction::X, ...)`; строковых action-литералов в C++
   и в Rust-диспетчере не должно оставаться.
4. Добавить/обновить table-driven тест полноты (каждый вариант `UiAction`
   имеет ветку в `dispatch` или явно помечен неиспользуемым и удалён).
5. Проверить `cargo build/test -p hydrus_client`.

### Шаблон типизированного шва (WS-15.2.1)

Пилот: `StatusBar` (статус-бар), `AboutInfo` (about), `ViewingStats` +
`ViewingStatRow` (статистика просмотров). Типы живут в `gui/qt/api.rs` в
единственном `#[cxx::bridge] mod ffi` (делить мост на два нельзя: cxx считает
одинаковые `type HydrusMainWindow` в разных мостах разными типами Rust);
C++ видит структуры из сгенерированного
`<hydrus_client/src/gui/qt/api.cxx.h>`. В сигнатурах типизированных функций
нет `QJsonObject`, Rust для них ничего не сериализует.

Как конвертировать шов:

1. Struct в `api.rs` (`#[derive(Clone, Debug, PartialEq)]`): только `String`,
   целые, `bool`, `f64`, `Vec<...>` shared-структур; без `Option` (отсутствие —
   пустая строка/0, не JSON null).
2. Вариант `UiCommand::X(Struct)` в `commands.rs` вместо `X(String)`; продьюсер
   в `gui/app` возвращает struct; все места `UiCommand::X(render::x_json(...))`
   обновить (проверка — grep по старому имени).
3. `extern "C++"` decl в том же мосту; C++-объявление в `bridge_api.hpp`,
   определение-обёртка в `bridge_api.cpp`; метод `HydrusMainWindow` объявить в
   `main_window.hpp` (для объявления достаточно forward-struct).
4. Тело метода, читающее поля, — в `typed_seams.cpp`, который включает ТОЛЬКО
   сгенерированный заголовок (+ Qt и `support/strings.hpp` с объявлениями).
   Причина: сгенерированный заголовок тянет экспортированные копии всех
   заголовков, и смешивать их с локальными в одном TU нельзя (двойные
   определения классов). По мере конверсии тела складываются сюда (или в
   `typed_<экран>.cpp`, если файл растянется за 500 строк).
5. Форматирование/предикаты шва — чистые `pub fn` в Rust с unit-тестом
   (образец: `query_page_media_text`, `downloader_page_media_text`,
   `busy_text`, `idle_text`, `bandwidth_text` в `render.rs` + тесты в
   `render/tests.rs`); поведение C++ не меняется, включая quirks рендеринга
   (например, `QString::number(double)` для целых секунд viewtime).

Как проверить: `parse_object(`/`json_payload(` не растут (по мере конверсии
убывают), новых `&str`-payload нет; сборка без предупреждений, тесты зелёные;
ни один файл `gui/qt` не больше 600 строк (`typed_seams.cpp` тоже).

Когда JSON допустим внутри поля структуры (WS-15.2.2d, уточнено в WS-15.2.3c):
только для данных принципиально произвольной формы — словари опций клиента,
импорт-опции целиком, произвольные настройки диалогов. В таком случае JSON
живёт внутри одного явно названного поля (`import_options_json: String`,
`PageRow.state` для import/export-страниц), а не вместо типа; фиксированные
поля (id, заголовок, флаги, счётчики, тексты) остаются типизированными.
**Запрещено** добавлять `json: String` (или безымянный `state: String`) в
структуру шва, если набор полей фиксирован и известен заранее: состояние
страницы, строки таблиц, статусы и проценты объявляются отдельными полями
структуры. Правильная форма шва (WS-15.2.3c, downloader):

```rust
struct DownloaderPageState {
    page_id: String,
    kind: String,
    status_top: String,        // текст готовит Rust
    files_progress: i32,       // процент считает Rust
    pending_jobs: Vec<String>,
    sites: Vec<GallerySite>,
    // ... только поля, которые C++ действительно читает
}
```

Rust-продьюсер заполняет её из `AppState` напрямую (`downloader_page_state` в
`render.rs`), без `json!` и промежуточной строки; C++ читает `state.status_top`
и `state.files_progress`. Чистые вычисления (статусы, проценты, склонения,
относительное время) живут в `pub fn` с unit-тестом, а не в обработчике C++.

Уточнения из WS-15.2.2a (страницы/вкладки/дерево/меню):

- Общий `enum` в мосту (`PageKind`) отображается в C++ как `enum class`;
  ему нужен явный `#[derive(Debug)]` (этот cxx его сам не даёт, но атрибут
  поддерживает) — иначе структуры с ним не соберутся под `UiCommand: Debug`.
- Поля `String` не приводятся к `rust::Str` неявно: для парсинга blob-полей
  (R1-исключение, см. ниже) используется хелпер вида
  `parse_string(const rust::String&)` → `parse_object(rust::Str(data, size))`.
- Рекурсивные UI (меню, дерево notebook'ов, дерево менеджера страниц)
  передаются плоскими preorder-строками с `depth`: Rust разворачивает дерево,
  C++ держит стек (`QMenu*`/`QTreeWidgetItem*`); submenu/sub-notebook владеет
  строками на `depth + 1`. Один тип-обёртка + тип строки на шов, общий enum
  переиспользуется между швами одной группы.
- Обратный канал: новая `extern "Rust"`-функция объявляется в `api.rs` и
  дублируется вручную в `bridge_api.hpp` рядом с `app_action`/`app_tick`
  (та же `noexcept`-форма); C++ вызывает её напрямую вместо
  `notify(action, target, json)`, а Rust-сторона переиспользует то же ядро,
  что и ветка `dispatch` (`page_move_request`, `close_pages`).

Уточнения из WS-15.2.3a (страницы/вкладки/сессия): обратный канал — отдельные
функции на команду (не `UiEvent`-структура): каждое событие получает свою
`extern "Rust"`-функцию `app_<domain>_<verb>(...)` с типизированными аргументами,
C++ вызывает её напрямую вместо `notify(action, target, payload)`, а Rust-сторона
переиспользует то же ядро, что и ветка `dispatch` (через `handle_*` → `with_state_mut`).
Как добавить новое событие: (1) объявить функцию в `gui/qt/api.rs` в `extern "Rust"`;
(2) реализовать шим там же, вызывающий `crate::gui::app::handle_*`; (3) реализовать
`handle_*` в `gui/app/mod.rs`, переиспользуя метод `AppState` (при необходимости
вынести логику из `dispatch` в метод `*_command`); (4) продублировать объявление
в `gui/qt/bridge_api.hpp` с `noexcept`; (5) в C++ заменить `notify(...)`/`app_action(...)`
на вызов типизированной функции. Строковые ветки `dispatch` для группы пока остаются
для меню/командной палитры, но прямые UI-источники их больше не используют.

Уточнения из WS-15.2.3c (page-команды): начальное состояние страницы больше не
едет строкой в `PageRow.state`. `PageRow` несёт типизированные поля `query`,
`downloader`, `subscriptions`, `duplicates`; `state: String` остался только у
import/export-папок (их шов — отдельная подзадача) и у placeholder-сообщения
(обычная строка, не JSON). Один тип состояния используется и в `UpdateX`, и в
`build_pages`, поэтому C++ не конвертирует JSON в структуру на старте. Если
диалог всё же отдаёт произвольный вложенный dict (например, import options),
это ровно одно поле с суффиксом `_json` и комментарием, почему схема не
фиксирована.

### Остаточные JSON-швы (WS-15.2.5)

Инвентаризация на `81e20d5` (15.2 ещё не закрыт: строковый диспетчер удалён,
но JSON-payload в обратном канале и часть forward-швов диалогов остались).
Счётчики — вхождения в `src/gui/qt/**/*.cpp|hpp`, без тестов.

| Шов | Где | Кол-во | Почему JSON остаётся | Статус |
| --- | --- | --- | --- | --- |
| Forward: состояние диалога | `api.rs`: 3 структуры с единственным полем `json: String` (остались из 17; закрыты 92b4391: tag filter/relations/manage tags/export review/local import review; партия 3: services/network/account types/API permissions; партия 4: duplicates rules/merge/custom/preview/review; `LocalImportTaggingDialogState` — следующий заход: произвольно вложенные `FilenameTaggingOptions`) | 3 | Содержимое диалога в Python — фиксированный `JObject`, но шов ещё не разобран по полям | долг 15.2.2/15.2.3 |
| Forward: стартовое состояние страницы | `PageRow.state: String` + `PageRow.folder: FolderPageState` | 1+1 | `state` — только placeholder-сообщение (не JSON); folder-страницы несут типизированный `FolderPageState` | закрыто (WS-15.2.4b) |
| Forward: произвольный dict | `SubscriptionDialogRow.import_options_json`, `AdminBlobs.options_json`, `AdminBlobs.service_info_json` | 3 | Словари опций/сервера произвольной формы, показываются или копируются как есть; `dict` и в Python | допустимое исключение R1 |
| Forward: C++-точки входа | `bridge_api.cpp`: 0 `parse_object(...)` (было 11: import/export folders 6, notes, tag dialog, autocomplete) | 0 | Все швы переведены на `hydrus_show_*` со структурой | закрыто (WS-15.2.4b) |
| Reverse: payload события | `notify(UiAction, target, payload)` + 21 `json_payload(...)` в 16 файлах | 21 | `UiAction` уже enum, но данные события всё ещё сериализуются в строку | долг 15.2.3 |
| Инфраструктура | `support/strings.{hpp,cpp}` (объявления/определения `parse_object`) | 2 | Плумбинг; удаляется вместе с последним швом, который его использует | — |
| C++-параметры | `QJsonObject` в заголовках | 49 | Следствие строк выше: оставшиеся диалоги принимают `const QJsonObject &state` или типизированную структуру | долг |

Итого: типизированы статус-бар/about/stats, меню/дерево/страницы, media/
selection/viewer/metadata, downloader/subscriptions/seed cache, duplicates,
page-команды, обратный канал services/network и tags/duplicates/subscriptions
(WS-15.2.4c) и часть диалогов (15 структур / диалоги timestamps, notifications,
repair locations, orphan files, database backup, tag filter, tag relations,
manage tags, export review, local import review, services, network,
account types, API permissions, duplicates rules/merge/custom/preview/review).
Не типизированы: 3 диалога-обёртки (options, import options, filename tagging)
и весь reverse-payload `notify`. Закрыты в WS-15.2.4b: import/export folders
(страницы, менеджеры, редакторы), notes, tag dialog, autocomplete.

Правило 15.2.5 (обязательно для новых экранов и фич):

1. В сигнатурах cxx и C++-заголовках нет `String`/`&str` JSON-payload и
   `QJsonObject`: состояние передаётся структурой из `api.rs` с отдельными
   полями (`hash`, `status_text`, `files_progress`, `Vec<String>` и т.д.).
2. Обратный канал — отдельная `extern "Rust"`-функция на событие с
   типизированными аргументами, а не `notify(action, target, json)`.
3. Единственное исключение — один JSON-строковый ресурс внутри типизированной
   структуры для принципиально произвольных данных (пример:
   `import_options_json`). Поле называется явно (`*_json`), имеет комментарий,
   почему схема не фиксирована, и не заменяет остальные поля.
4. Страж: `cargo test -p hydrus_client seam_guard` (`gui/qt/seam_guard.rs`)
   падает, если в `src/gui/qt` появился `notify(QStringLiteral(`,
   выросло число `parse_object(`/`json_payload(` сверх явного allowlist,
   в `api.rs` появилось поле `json`/`*_json`/`state: String` вне allowlist
   или новая cxx-функция с аргументом `json:`. Перевод шва только уменьшает
   allowlist.

Вёрстка обязана повторять Python-экран, а не быть «похожей формой»: тот же набор
и порядок контролов, дословные подписи и заголовки, те же размеры/минимальные
размеры/отступы/spacing, колонки таблиц и их сортировка, иконки с теми же именами,
inline help и тема Hydrus по умолчанию (светлая нативная палитра; тёмный только
media canvas). Источник истины — Python-модуль экрана
(`hydrus_python/hydrus/client/gui/...`) и скриншот оригинала; подробный чек-лист —
в `docs/GUI_PORT_PLAN.md`, раздел «Правило визуального паритета». Экран, который
работает, но выглядит иначе, задачу не закрывает.

Python-клиент v692 запускается локально из `hydrus_python` (venv через
`setup_venv.py`, skill `hydrus-python-run`), поэтому эталонный скриншот снимается с
живого окна на тестовой БД, а не только из старых `docs/images`.

Светлая тема — это нативный Qt-стиль `windowsvista` (как `windows11`/`windowsvista`
в `ClientGUIStyle.py`) и только базовые правила `default_hydrus.qss`
(`HydrusOnOffButton` и т.п.); произвольный `QWidget { background: white }` поверх
нативного стиля не используется. Тёмная тема остаётся отдельным QSS в
`gui/style.rs`.

## Сборка и поставка

Qt подключается динамически, как в Python-клиенте: exe не содержит Qt внутри и
находит `Qt6Core/Gui/Widgets` рядом с собой. Поэтому для запуска на другой машине
Qt и MSVC runtime кладутся в ту же папку, а `PATH` пользователя не важен.

Скрипты (все пути Qt/MSVC можно переопределить через окружение):

- `scripts/qt-env.ps1` — общий поиск `vcvars64.bat` и Qt;
- `scripts/cargo.ps1 build|test|run …` — cargo с MSVC и Qt на `PATH`;
- `scripts/qt-runner.bat` — runner из `.cargo/config.toml`, чтобы `cargo run` и
  `cargo test` находили Qt DLL;
- `scripts/deploy.ps1 [-Profile release|debug] [-OutDir PATH] [-Zip]` —
  переносимая сборка: `windeployqt` + `qt.conf` + runtime DLL.

Переменные окружения: `HYDRUS_VS_VCVARS`, `HYDRUS_QT_BIN`, `QT_ROOT`, `QMAKE`.
Без них Qt ищется в `C:\Qt\<версия>\msvc*_64` (сначала `build.rs`, затем
`qt-env.ps1`). Прямой `cargo build` без `QMAKE`/Qt в `PATH` не работает — для
этого есть `scripts/cargo.ps1`.

`deploy.ps1` создаёт `dist/hydrus_client` (в git не попадает):

```text
hydrus_client.exe
Qt6Core.dll, Qt6Gui.dll, Qt6Widgets.dll, Qt6Network.dll, Qt6Svg.dll
vcruntime140*.dll, msvcp140*.dll, concrt140.dll   (release)
qt.conf                  Prefix = ., Plugins = plugins
plugins/platforms/qwindows.dll и остальные плагины
```

`qt.conf` задаёт плагины относительно exe, поэтому папку можно запускать из
любого каталога. `-Zip` дополнительно собирает `dist/hydrus_client.zip`.

## Vendored static assets

В корне репозитория лежит папка `static/` — копия `hydrus/static` Python-клиента
(иконки, QSS, формы звёзд и т.д.), нужна для визуального паритета. `deploy.ps1`
копирует её рядом с exe (`dist/hydrus_client/static`).

Разрешение пути: `gui::app::static_assets::resolve_static_dir()` ищет в порядке:
1. переменная окружения `HYDRUS_STATIC_DIR`;
2. `<каталог exe>/static`;
3. `<корень репозитория>/static` (dev-сборки через `CARGO_MANIFEST_DIR`).

Qt-слой получает путь через `ffi::hydrus_static_dir()`. `hydrus_small_icon(kind)`
и `HydrusPageManager::icon_for(kind)` сначала пытаются загрузить реальный файл
по таблице соответствий (например `paste` → `paste.png`, `star` → `star.svg`,
`gear` → `cog.svg`), кэшируя `QIcon` по пути; если файл или каталог отсутствует,
используется прежний нарисованный fallback, поэтому приложение работает и без
`static/`. SVG требует плагин `imageformats/qsvg.dll` (deploy.ps1 копирует его,
если `windeployqt` не положил).
Профиль `debug` кладёт debug-DLL и годится только для локальной проверки:
debug CRT не распространяется.

## Страницы

- `PageNode::Query` — файловая страница: запрос, хэши, фокус, статус. Страницы,
  открытые кнопкой «files» у загрузчика, используют `fixed_hashes`.
- `PageNode::Notebook` — «page of pages» с собственным current.
- `PageNode::Downloader` — urls / gallery / watcher / simple downloader: поля,
  очередь URL, статусы, лог, результаты, настройки watcher и per-page import
  option overrides. Gallery-страница повторяет Python-sidebar (`gallery
  downloader` + `highlighted query` с `imports`/`search`, `selection tags`);
  file limit ограничивает число найденных ссылок на странице.
- `PageNode::Subscriptions` — общий менеджер периодических подписок.
- `PageNode::Duplicates` — очередь потенциальных пар и решения merge/false-positive;
  очередь готовится батчами по `duplicate_filter_max_batch_size`, размер
  меняется на панели страницы, кнопка next batch перечитывает остаток.
  `DuplicateSearchContext` задаёт область поиска: два file search context
  (query + локальный file domain), режим one file/both files/different searches,
  pixel dupes required/allowed/excluded и max hamming distance. Контекст
  фильтрует очередь до auto-resolution и сохраняется в `gui session.json`
  вместе со страницей. Там же хранится сортировка пар (larger/smaller filesize,
  similarity, random; asc/desc); она применяется до auto-resolution и батча.
  Auto-resolution preview и review показывают thumbnail pair list: миниатюры A/B,
  дистанцию и действие, а контекстное меню/Enter открывает выбранные пары в новой
  странице.
  Панель решений покрывает весь набор duplicate actions: this is better with
  delete/keep both, same quality, related alternates, false positive, delete both
  и custom action. Custom action — отдельный диалог с типом действия, выбором
  «какой файл лучше», delete neither/A/B/both и собственными merge options для
  этой пары, которые не сохраняются в options. Кнопка actions... и контекстное
  меню на любой стороне выполняют операции над связями: remove potentials,
  reset potential search, clear all/internal false positives, remove from
  duplicate/alternates group, dissolve duplicate/alternates group. Они меняют
  БД, перечитывают очередь и показывают результат в статусе страницы.
  Auto-resolution правила проверяют пару в обе стороны: selector состоит из
  one-file условий, относительных A/B сравнений, filetype-проверок и AND/OR
  групп; правило может сузить очередь поисковым предикатом (`both`/`either`)
  и максимальной дистанцией. Semi-automatic правила складывают совпадения в
  `duplicate auto-resolution pending.json`, fully automatic применяют сразу.
  Preview считает совпадения, не меняя БД; review-диалог показывает
  pending/denied/taken и умеет approve/deny выбранных или всех пар.
  Duplicate merge options задают, что переносится из худшего файла: архивный статус,
  urls, notes, modified date и действия copy/move/two-way/none по каждому tag и
  rating сервису отдельно для «this is better» и «same quality». Они хранятся в
  options в группе `duplicate_action_options` и используются ручными решениями,
  auto-resolution и review.
- `PageNode::ImportFolders` — менеджер папок и периодическое сканирование.
- `PageNode::Placeholder` — только для старых сессий и неизвестных типов.

`HydrusCanvas` проигрывает статичные изображения и анимированные GIF/WebP через
`QMovie`; кнопка pause/play и клавиша `P` управляют анимацией. Вьюер умеет
открывать файл системным приложением и копировать hash/исходный URL. Соседние
файлы заранее кладутся в `QPixmapCache`, а размер/позиция окна сохраняются в
`QSettings`. Масштаб переключается fit/100% и колесом, панорамирование включается
кнопкой `pan`, фон — dark/black/mid/light/checkerboard; выбранные вьюером режимы
хранятся в `QSettings` (`viewer/background`, `viewer/pan`). При наведении курсора
на верхнюю часть канваса появляется hover-панель с archive/inbox/delete/tags/notes,
действия уходят в `app_action` с page id и hash.

Миниатюры можно перетаскивать как локальные файлы (экспорт наружу) или как
`application/x-hydrus-files`; бросок на другую страницу добавляет хэши в её
результаты (`add_hashes_to_page`), бросок локальных файлов на сетку запускает
обычный импорт.

Раскладка миниатюр (`HydrusThumbnailGrid`) читает `thumbnails` из `query_state`:
`thumbnail_scale_percent` масштабирует иконки, `thumbnail_layout_masonry` включает
variable-height masonry (QListWidget с неоднородными size hint), а
`thumbnail_scroll_horizontal` переключает flow на `TopToBottom` с горизонтальным
скроллбаром. Шапка результатов показывает число файлов; `importing` в state
рисует determinate QProgressBar, `loading` — overlay loading panel. Поиск
помечает страницу `loading = true`, отдаёт `UpdatePage` и через `UiCommand::Defer`
выполняет `run_query` на следующем цикле событий, поэтому панель успевает
отрисоваться до синхронного запроса к БД.

Дерево, очереди загрузчиков, результаты и подписки сериализуются в единый
`gui session.json` (версия 3): `app` хранит дерево страниц, `legacy` — старый
`GuiSession` контроллера, плюс `subscriptions`. Загрузчик читает и старые плоские
сессии (`pages`) и app-v2 (`root`). При старте активная страница вычисляется по
цепочке `current` вложенных notebook'ов; дубликаты id исправляются.

Меню Help → popup messages... открывает историю `NotifyMessage` (время, уровень,
текст) с кнопкой очистки; лог ограничен 200 записями и живёт в `AppState`.
Лимит истории читается из `popup_message_max_messages`, а запись статистики
просмотров отключается опцией `file_viewing_statistics_active`.
Help → about собирает `render::about` (SOFTWARE_VERSION, NETWORK_VERSION,
CLIENT_API_VERSION, db version, SQLite/Qt/Rust, пути и счётчики сервисов) и
показывает вкладки overview/license/credits с кнопкой открытия сайта.
`run_application` показывает `SplashModel` через QSplashScreen: версии, статусы
«loading pages…», «ready», и закрывает его через QTimer после старта.

Меню Database вызывает реальные SQLite-операции: `PRAGMA quick_check`, `VACUUM`,
`PRAGMA optimize` + `ANALYZE`, обзор `page_count`/`freelist_count` и подсчёт
файлов в trash. `fix missing archive times` проходит по архивным файлам
«my files» без записи в `archive_timestamps` и проставляет текущее время.
`clear orphan files` обходит файловые и thumbnail-хранилища, находит имена с
хэшем, которого нет в `master`, и позволяет удалить их или перенести в
выбранный каталог; перед каждым действием путь и отсутствие записи
перепроверяются, а файлы с обычными именами не трогаются.

`back up database` пишет `client.db` в каталог из опции `backup_path` через
`VACUUM INTO`, поэтому копия консистентна без закрытия клиента; путь и время
последнего бэкапа (`last_backup_time`) сохраняются в options, а живой файл БД
нельзя выбрать целью. Восстановление копии пока выполняется вручную заменой
`client.db` при закрытом клиенте.

После импорта в очередь `FilesMaintenanceManager` ставятся задания modified time и
transparency. `maintenance_tick` выполняет до двух заданий за тик (если включены
`file_maintenance_during_active/idle`), а меню Database показывает очередь и умеет
прогнать её вручную. Неподдерживаемые задания удаляются, чтобы не зацикливать тик.

Страницы можно перетаскивать между notebook'ами (`move_page`), дублировать
(`duplicate_page` с новыми id для всего поддерева) и выбирать через менеджер
страниц. Навигация хранится в `PageHistory` и доступна через Alt+Left/Right.
Закрытые страницы попадают в `closed_pages` и восстанавливаются через Undo
(Ctrl+Z) на прежнее место.
Именованные сессии лежат в `sessions/<name>.json`, резервные копии — в
`sessions/backups/`; загрузка заменяет дерево, добавление перенумеровывает id и
дополняет текущее дерево, не трогая открытые страницы.

## Поиск

Строка поиска парсится в предикаты. Слова, образующие известный тег, склеиваются
(`blue eyes` — один тег), поддерживаются `-tag`, wildcard `*`, namespace `ns:`,
`system:inbox`, `system:archive`, `system:everything`, `system:limit=N`.
Результат пересекается с текущими файлами выбранного file domain.

`gui::app::search` строит `SearchPlan`: теги/namespace/wildcard и системные
предикаты через `search::parse_system_predicate_with_services`. Теги фильтруются
индексами `db::tag_search`, а свойства файлов (размер, ширина/высота, ratio, mime,
hash, duration, fps, frames, пиксели, слова, число тегов/URL/заметок, рейтинги,
даты импорта/архивации/изменения/просмотра, known URL/domain/regex/URL class,
file viewing stats, file service, duplicate relationships и similar files)
проверяются по `files_info`, таблицам времени, `url_map`, `file_viewing_stats`,
`db::files_storage`, `db::duplicates` и `db::similar_files`. Неподдерживаемые
системные предикаты пропускаются, а неизвестный `system:*` возвращает ошибку
разбора в статус страницы.

`system:best quality of group` сравнивает hash с king группы, `system:num file
relationships` считает duplicates/alternates/potential duplicates/false positives,
а `system:similar to files|data ... distance N` ищет по perceptual/pixel-хэшам
через `similar_files::search_similar`.

OR-группы записываются как `{predicate | predicate}` (вложенные группы
поддерживаются, `-{...}` исключает объединение). `FileSystemPredicates.or_predicates`
оценивается по каждому hash: тег-предикаты проверяются по mappings, системные —
тем же `matches_system`. Диалог `OR...` собирает по одному предикату на строку и
вставляет группу в строку поиска.

Автодополнение запрашивается действием `autocomplete` по текущему хвосту строки
(до трёх слов). `db::tag_search::suggest_tags`/`suggest_namespaces` отдают теги и
namespace’ы с числом файлов; Qt рисует вкладки tags/namespaces/wildcards/system и
при выборе заменяет только этот хвост. `file viewing stats` пополняется при
открытии/закрытии вьюера и переходе к соседнему файлу.

`metadata::copy_tags` копирует display-теги focused-файла в клипборд, а
`apply_tags_selection` применяет список (по строке или через запятую) ко всем
selected-файлам; префикс `-` удаляет тег. Обе операции идут через те же
`mappings`, что и одиночное редактирование, и обновляют все страницы.

`metadata::sort_display_tags` сортирует display-теги по lexicographic, namespace
или subtag, а `display_tag_text` убирает namespace’ы и заменяет `_` на пробелы по
опциям `show_namespaces`/`replace_tag_underscores_with_spaces`; выбор хранится в
`tag_display_sort`.

Избранные поиски хранятся в `favourite searches.json` рядом с сессией: имя,
запрос, file domain и tag service. Меню `Pages → favourite searches` умеет
сохранить текущий контекст, применить его к активной странице и удалить запись.
Последние запросы лежат в `search history.json`; `undo_search`/`redo_search`
ходят по per-page стеку предикатов, а меню `recent searches` повторяет запрос.

## Импорт и загрузка

`gui::app::import::ImportContext` выполняет полный путь: SHA-256, sniffing MIME,
`files_info`, копирование в file storage, запись в локальный file domain, inbox,
thumbnail и индексацию perceptual/pixel hash. Тот же контекст используется
фоновым worker'ом.

`importing::options` содержит import options framework: prefetch, file filtering,
tag filtering, locations, tags, notes, presentation и external programs. Каждый
контейнер хранит только явно заданные типы и source label; `ImportOptionsManager`
держит default-контейнер на каждый caller type плюс именованные favourites.
Эффективные опции собираются по приоритету specific importer -> caller default ->
global, поэтому более конкретные настройки перекрывают общие. Менеджер сохраняется
в options group `import_options`; диалог File → import options... редактирует
контейнеры по вкладкам и позволяет сбросить тип на наследование. Локальный импорт
применяет file filtering, destinations и automatic archive из эффективных опций
`local_import`.

`FilenameTaggingOptions` превращает путь файла в теги: общие теги, имя файла с
namespace, до шести directory-слотов (с начала и с конца пути), обычные regex'ы и
regex'ы с namespace. Import folder хранит такую настройку на каждый локальный tag
service (ключ — hex service key) и передаёт её в worker; после успешного импорта
теги добавляются в соответствующий локальный tag service.

Локальный импорт идёт через review-панель (`gui/app/local_import.rs`): пути
классифицируются как good, empty, missing, unsupported, occupied или sidecar,
папки раскрываются с учётом `search subdirectories`, а sidecar'ы определяются по
совпадению basename с обычным файлом (`.txt`, `.json`, `.xml`). Импортируются
только good-файлы; опции берутся из effective `local_import` контейнера, filename
tagging задаётся отдельно для каждого локального tag service, а
`delete_after_success` удаляет оригинал после успешного нового импорта (уже
известный хеш не удаляется). Панель открывается из File → import files..., из
database → maintenance → review local imports... и при drag/drop.

`gui/app/file_locations.rs` обслуживает repair file locations: показывает prefix,
текущую и исправленную base, проверяет наличие подпапки, умеет подобрать
`client_files`/`thumbnails` внутри выбранной папки, создать пустые thumbnail
папки и записать `client_files_subfolders` плюс current locations в client.db.
При старте с отсутствующей subfolder панель открывается автоматически; свежая
база создаёт и сохраняет дефолтные `files`/`thumbs`.

Downloader-страницы хранят файловые seed'ы в `DownloaderPage.queue` и журнал
gallery/post URL в `DownloaderPage.gallery_seeds`; обе структуры сериализуются в
`gui session.json`. Диалоги `file seed cache` и `gallery seed log` показывают
статусы, таймстампы и заметки, умеют фильтровать по статусу/тексту, сортировать,
копировать URL, повторять неудачные, удалять выбранные и очищать завершённые.
Worker присылает `GallerySeed` событие на каждый обработанный page/post, поэтому
журнал заполняется по ходу gallery/watcher импорта.

Downloader-страницы и подписки хранят `import_options` как контейнер overrides.
Эффективные опции собираются как specific -> caller (`post_urls` для
urls/gallery/simple, `watcher_urls` для watcher, `subscription` для подписок) ->
global. Диалог `import options...` на странице редактирует только overrides
(не заданные вкладки наследуются), summary показывается на кнопке, а worker
получает готовый effective-контейнер в `DownloadUrls`/`ProcessPages` и применяет
его к каждому скачанному файлу. Контейнеры переживают перезапуск через
`gui session.json`.

Подписку можно переименовать, сменить URL/query/site/filter/period и paused в
`subscription editor`; per-subscription import options живут в том же контейнере.
Полноценный multi-query editor Python-подписок пока не перенесён.

`gui::app::download` создаёт задания, а `worker` качает прямые URL или страницы
через site-specific парсеры (`parsing::gallery`: danbooru, gelbooru, e621, rule34,
safebooru, pixiv, generic). Generic-парсер следует ссылкам страницы и собирает
`href`/`src`/`og:image`/JSON-медиа. Gallery подставляет `{query}`, переходит по
next-страницам, watcher периодически перепроверяет страницу, subscriptions хранят
период и последний запуск.

Скачивание файлов идёт через `networking::download_pools`: URL группируются по
домену, домен получает свой пул (лимит из `max_network_jobs_per_domain` или
переопределение), глобальный `PoolGate` держит `max_network_jobs`. Перед запросом
worker ждёт `NetworkBandwidthManager` (requests/data rules), после ответа сообщает
размер; правила и лимиты пулов сохраняются в `network settings.json`.

## Client API

`api::ClientApiBackend` работает поверх `DBBase` и `ClientFilesManager`, а
`api::ApiServer` — маленький локальный HTTP-сервер на `127.0.0.1`. Поддержаны
`/api_version`, `/get_services`, `/get_files/search_files`, `/get_files/file_metadata`,
`/get_files/file`, `/add_tags/add_tags`, `/add_tags/delete_tags` и access key в
query или заголовке `Hydrus-Client-API-Access-Key`. Сервер запускается/останавливается
из Options/Network manager и хранит ключ в `ClientOptions`.

## Теги и relations

`gui::app::tag_relations` — редактор tag siblings/parents поверх
`db::tag_relations`: пары хранятся по локальному tag-сервису, `current`-статус,
добавление отклоняет self-пары и циклы (sibling resolution и parent ancestors).
`metadata::metadata_json` отдаёт и storage-теги, и `display_tags` с учётом siblings
плюс карту `display_tag_sources`, поэтому metadata-панель умеет переключаться на
Hydrus-подобное отображение display tags без второй реализации.

`metadata::manage_tags_json` агрегирует теги выделения по каждому локальному
tag-сервису со счётчиками. Диалог manage tags выбирает сервис и отправляет
`apply_tags` с массивом `hashes`, поэтому добавление/удаление применяется ко всем
выбранным файлам одной транзакцией UI-команды.

## Import folders

`gui::app::import_folders` хранит entry в `import folders.json`: период, paused,
check regularly, subdirectories, skip recent, расширения, min/max размер и
пост-импортное действие. Сканирование применяет фильтры до постановки в очередь;
`AsyncJob::ImportPaths` несёт имя папки, а `ImportFinished` — результаты по каждому
файлу, поэтому move/delete выполняется только для успешно импортированных файлов.
Move не перезаписывает существующие имена.

## Services, server administration and logins

`gui::app::services` owns the extended service dialog. Local service entries
keep their type and dictionary in the client DB; remote repository and
administration services store `host`, `port`, `access_key`, repository periods
and file-repository storage limits in the same dictionary. The service editor
and the service specifier are rendered by Qt, while validation and persistence
stay in Rust.

`networking::hydrus_admin` speaks the Hydrus server protocol: compressed
`ParsedRequestArguments` bodies, the `Hydrus-Key` header, and the same
`ServerService`/`Account` objects as `hydrus_server`. It can list, add, edit and
delete server services, lock/unlock, regenerate service info, set update and
nullification periods, and review/modify accounts. Account types can be added,
edited and deleted; the editor changes the title, per-content-type permissions
and auto-creation velocity while preserving bandwidth rules and creation
history. A blank access key performs the one-time `/access_key` bootstrap
through the `init` registration key. Remote connections accept the server's
self-signed certificate; that exception is limited to this client.

Login credentials are persisted in `logins.json` and edited from the logins tab.
The server connection itself is saved in `server admin.json`, including the
access keys returned when a service is created. Account actions and service
options are only cached in memory and refreshed on demand.

Tag filters are edited from the services dialog, the tags menu or the
services menu. Rules live in the service dictionary as `tag_filter` so local
tag services and offline repositories keep them; tag repositories additionally
receive a `POST /tag_filter`. The same dialog can purge tags from a local tag
service, deleting its current and pending mappings.

Client API access keys and their permissions are stored in
`api permissions.json` and edited from services → client api permissions. Each
entry carries a name, 64 hex key, optional basic permissions and a search tag
filter. The API server reloads the file whenever an entry changes; `/api_version`
and `/get_services` need only a valid key, while file search/fetch and tag edits
check their specific permission and the search filter hides files whose tags do
not pass.

## Export folders

`gui::app::export` хранит entry в `export folders.json`: destination, поисковый
запрос, filename phrase, copy/synchronise, sidecar json/text, overwrite,
delete-from-client, период и paused. `execute` на worker-потоке выбирает файлы
тем же `query::hashes_for`, что и страница поиска, строит имя через
`[namespace] {hash} {file_id} {#} {tags} {nn tags} (tag)` и никогда не выпускает
путь за пределы destination. Synchronise удаляет файлы, которых больше нет в
результате поиска, и пустые каталоги. Review-диалог показывает план экспорта и
позволяет исключить отдельные файлы; выбранные хеши идут в
`AsyncJob::ExportFolder`. Ошибки и итог возвращаются через
`AsyncEvent::ExportFinished` и попадают в статус папки и popup-историю.

## Что дальше

1. Полный tag display manager (namespace/subtag ordering/decorators).
2. File maintenance jobs (thumbnails/metadata regeneration) и оставшиеся hotkeys.
3. Синхронизация с tag repositories и pending content.
