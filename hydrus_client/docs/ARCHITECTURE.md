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

## Страницы

- `PageNode::Query` — файловая страница: запрос, хэши, фокус, статус. Страницы,
  открытые кнопкой «files» у загрузчика, используют `fixed_hashes`.
- `PageNode::Notebook` — «page of pages» с собственным current.
- `PageNode::Downloader` — urls / gallery / watcher / simple downloader: поля,
  очередь URL, статусы, лог, результаты и настройки watcher.
- `PageNode::Subscriptions` — общий менеджер периодических подписок.
- `PageNode::Duplicates` — очередь потенциальных пар и решения merge/false-positive.
- `PageNode::ImportFolders` — менеджер папок и периодическое сканирование.
- `PageNode::Placeholder` — только для старых сессий и неизвестных типов.

`HydrusCanvas` проигрывает статичные изображения и анимированные GIF/WebP через
`QMovie`; кнопка pause/play и клавиша `P` управляют анимацией. Вьюер умеет
открывать файл системным приложением и копировать hash/исходный URL. Соседние
файлы заранее кладутся в `QPixmapCache`, а размер/позиция окна сохраняются в
`QSettings`.

Дерево, очереди загрузчиков, результаты и подписки сериализуются в единый
`gui session.json` (версия 3): `app` хранит дерево страниц, `legacy` — старый
`GuiSession` контроллера, плюс `subscriptions`. Загрузчик читает и старые плоские
сессии (`pages`) и app-v2 (`root`). При старте активная страница вычисляется по
цепочке `current` вложенных notebook'ов; дубликаты id исправляются.

Меню Database вызывает реальные SQLite-операции: `PRAGMA quick_check`, `VACUUM`,
`PRAGMA optimize` + `ANALYZE`, обзор `page_count`/`freelist_count` и подсчёт
файлов в trash.

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
предикаты через `search::parse_system_predicate`. Теги фильтруются индексами
`db::tag_search`, а свойства файлов (размер, ширина/высота, ratio, mime, hash,
duration, fps, frames, пиксели, слова, число тегов/URL/заметок, рейтинги и даты
импорта/архивации/изменения/просмотра) проверяются по `files_info` и таблицам
времени. Неподдерживаемые системные предикаты пропускаются, а неизвестный
`system:*` возвращает ошибку разбора в статус страницы.

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

## Import folders

`gui::app::import_folders` хранит entry в `import folders.json`: период, paused,
check regularly, subdirectories, skip recent, расширения, min/max размер и
пост-импортное действие. Сканирование применяет фильтры до постановки в очередь;
`AsyncJob::ImportPaths` несёт имя папки, а `ImportFinished` — результаты по каждому
файлу, поэтому move/delete выполняется только для успешно импортированных файлов.
Move не перезаписывает существующие имена.

## Что дальше

1. Полный tag display manager (namespace/subtag ordering/decorators).
2. File maintenance jobs (thumbnails/metadata regeneration) и оставшиеся hotkeys.
3. Синхронизация с tag repositories и pending content.
