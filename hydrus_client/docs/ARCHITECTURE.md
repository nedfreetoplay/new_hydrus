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

C++-фасад (`bridge.hpp`/`bridge.cpp`) использует обычные Qt Widgets. JSON
передаётся только через границу cxx: дерево страниц, состояние query/downloader/
duplicates/subscriptions, меню, статус-бар и данные вьюера. Никакой бизнес-логики
в C++ нет.

Вёрстка обязана повторять Python-экран, а не быть «похожей формой»: тот же набор
и порядок контролов, дословные подписи и заголовки, те же размеры/минимальные
размеры/отступы/spacing, колонки таблиц и их сортировка, иконки с теми же именами,
inline help и тема Hydrus по умолчанию (светлая нативная палитра; тёмный только
media canvas). Источник истины — Python-модуль экрана
(`hydrus_python/hydrus/client/gui/...`) и скриншот оригинала; подробный чек-лист —
в `docs/GUI_PORT_PLAN.md`, раздел «Правило визуального паритета». Экран, который
работает, но выглядит иначе, задачу не закрывает.

## Страницы

- `PageNode::Query` — файловая страница: запрос, хэши, фокус, статус. Страницы,
  открытые кнопкой «files» у загрузчика, используют `fixed_hashes`.
- `PageNode::Notebook` — «page of pages» с собственным current.
- `PageNode::Downloader` — urls / gallery / watcher / simple downloader: поля,
  очередь URL, статусы, лог, результаты, настройки watcher и per-page import
  option overrides.
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
Help → about собирает `about_json` (SOFTWARE_VERSION, NETWORK_VERSION,
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
