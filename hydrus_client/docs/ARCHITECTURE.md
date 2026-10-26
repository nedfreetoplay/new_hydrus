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
`OpenViewer`, `Status`, …). `gui::qt::panels` исполняет эти команды над
`HydrusMainWindow`.

Это важно для модальных диалогов: пока открыт tag-dialog, Rust-состояние уже
отпущено, поэтому повторный вызов `app_action` из диалога не вызывает
deadlock/повторный borrow.

C++-фасад (`bridge.hpp`/`bridge.cpp`) использует обычные Qt Widgets. JSON
передаётся только через границу cxx: дерево страниц, состояние query-страницы,
меню, статус-бар и данные вьюера. Никакой бизнес-логики в C++ нет.

## Страницы

- `PageNode::Query` — файловая страница: запрос, хэши, фокус, статус.
- `PageNode::Notebook` — «page of pages» с собственным current.
- `PageNode::Placeholder` — ещё не портированные типы (downloaders,
  duplicates); текст честно говорит об этом.

Дерево сериализуется в `gui session.json`. При старте активная страница
вычисляется по цепочке `current` вложенных notebook'ов.

## Поиск

Строка поиска парсится в предикаты. Слова, образующие известный тег, склеиваются
(`blue eyes` — один тег), поддерживаются `-tag`, wildcard `*`, namespace `ns:`,
`system:inbox`, `system:archive`, `system:everything`, `system:limit=N`.
Результат пересекается с текущими файлами выбранного file domain.

## Импорт

`gui::app::import` выполняет полный путь: SHA-256, sniffing MIME, `files_info`,
копирование в file storage, запись в локальный file domain, inbox, thumbnail.
URL-импорт скачивает байты через `reqwest::blocking` и передаёт их тому же
пайплайну. Это синхронная основа; очередь/асинхронность — следующий шаг.

## Что дальше

1. Реальные страницы downloaders (urls/gallery/watcher/subscriptions/simple).
2. Duplicates processing и merge UI.
3. Options / manage services / network dialogs.
4. Асинхронные очереди импорта и загрузки; Client API сервер.
