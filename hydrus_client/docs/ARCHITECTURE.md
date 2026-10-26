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
держит один фоновый поток и канал событий: импорт файлов, прямые URL и разбор
gallery/watcher/subscription страниц ставятся в очередь. `app::handle_tick`
вызывается Qt-таймером каждые 120 мс, забирает накопившиеся события и обновляет
страницы/статус-бар. Фоновый поток получает `DBBase` и `ClientFilesManager` копией,
поэтому ему не нужен `Controller` и он не пересекается с borrow-ами UI.

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
- `PageNode::Placeholder` — только для старых сессий и неизвестных типов.

Дерево, очереди загрузчиков, результаты и подписки сериализуются в
`gui session.json` (версия 2). При старте активная страница вычисляется по цепочке
`current` вложенных notebook'ов; дубликаты id из старых сессий исправляются при
загрузке.

## Поиск

Строка поиска парсится в предикаты. Слова, образующие известный тег, склеиваются
(`blue eyes` — один тег), поддерживаются `-tag`, wildcard `*`, namespace `ns:`,
`system:inbox`, `system:archive`, `system:everything`, `system:limit=N`.
Результат пересекается с текущими файлами выбранного file domain.

## Импорт и загрузка

`gui::app::import::ImportContext` выполняет полный путь: SHA-256, sniffing MIME,
`files_info`, копирование в file storage, запись в локальный file domain, inbox,
thumbnail и индексацию perceptual/pixel hash. Тот же контекст используется
фоновым worker'ом.

`gui::app::download` создаёт задания, а `worker` качает прямые URL или страницы,
извлекает `href`/`src`/`data-src`, скачивает найденные медиа и импортирует их через
общий пайплайн. Gallery подставляет `{query}` в URL, watcher периодически
перепроверяет страницу, subscriptions хранят период и последний запуск. Pause
сети и прокси применяются к worker'у через `Controller`.

## Client API

`api::ClientApiBackend` работает поверх `DBBase` и `ClientFilesManager`, а
`api::ApiServer` — маленький локальный HTTP-сервер на `127.0.0.1`. Поддержаны
`/api_version`, `/get_services`, `/get_files/search_files`, `/get_files/file_metadata`,
`/get_files/file`, `/add_tags/add_tags`, `/add_tags/delete_tags` и access key в
query или заголовке `Hydrus-Client-API-Access-Key`. Сервер запускается/останавливается
из Options/Network manager и хранит ключ в `ClientOptions`.

## Что дальше

1. Site-specific gallery parsers и редактор URL classes вместо универсального
   разбора ссылок.
2. Полные tag-панели, рейтинги, notes, импорт-папки и auto-resolution rules.
3. Отдельные пулы загрузки по доменам и более точные bandwidth rules.
