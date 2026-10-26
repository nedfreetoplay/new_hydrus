# hydrus_client

Rust-порт `hydrus_python/hydrus/client`.

## Требования

- Rust 1.85+ (edition 2024), MSVC toolchain.
- Visual Studio 2022 с C++ workload (MSVC 14.4x).
- Qt 6.12.0 MSVC 2022 x64 в `C:\Qt\6.12.0\msvc2022_64`.
- CMake и Ninja не обязательны для сборки: используется `cxx-qt-build` + `qmake`.

## Сборка

Переменные окружения (или `.cargo/config.toml` в корне workspace):

```powershell
$env:QMAKE = 'C:\Qt\6.12.0\msvc2022_64\bin\qmake.exe'
$env:CMAKE_PREFIX_PATH = 'C:\Qt\6.12.0\msvc2022_64'
$env:PATH = 'C:\Qt\6.12.0\msvc2022_64\bin;' + $env:PATH
cargo build -p hydrus_client
```

Запуск (Qt DLL и плагины должны быть в `PATH`):

```powershell
cargo run -p hydrus_client
```

Если `cl.exe` не находится, запускайте сборку через обёртку (загружает VS 2022 и Qt):

```powershell
.\scripts\cargo.ps1 test -p hydrus_client
.\scripts\cargo.ps1 run -p hydrus_client
```

## Статус портирования

Фундамент готов: константы, настройки (миграции 1→8), метаданные, поиск, медиа-модель,
файлы, БД-слой, сеть, импорт/парсинг/дубликаты, PNG-сериализация и восстановление сессии.
Поведение файлового хранилища, URL-кодирования и системных предикатов сверено с
`hydrus_python/hydrus/test`; интеграционный тест `python_db_compat` открывает
Python-shaped `client.db`.

### Что уже работает в GUI

Qt-интерфейс больше не заглушка: `hydrus_client/src/gui/app` — это настоящее состояние
клиента, а `hydrus_client/src/gui/qt` только рисует его.

- вкладки страниц как в Hydrus: закрытие, переименование, контекстное меню,
  вложенные «page of pages» и вкладка `+` с выбором типа страницы;
- файловые страницы: реальный поиск по локальной БД (теги, `-tag`, wildcard,
  namespace, `system:inbox`, `system:archive`, `system:limit=N`), автодополнение тегов;
- сетка миниатюр, предпросмотр выбранного файла, статус страницы;
- медиа-вьюер: зум, панорамирование, поворот, следующая/предыдущая, теги,
  управление тегами, archive/inbox/delete, F11;
- импорт локальных файлов через меню и drag-and-drop: хэширование, копирование в
  файловое хранилище, `files_info`, сервис файлов, inbox и генерация миниатюр;
- импорт локальных файлов и загрузка URL идут через фоновую очередь: прогресс виден
  в статус-баре, интерфейс не блокируется;
- страницы загрузчиков: полноценная `urls`-очередь с повтором/очисткой, `gallery`
  (разбор страницы и постов по ссылкам, подстановка `{query}` в URL), `watcher`
  с автоматическими проверками по периоду, `subscriptions` с менеджером подписок и
  `simple downloader` для страниц со ссылками на файлы;
- duplicates processing: пары из `potential_duplicate_pairs`, две стороны с превью,
  тегами и URL, «this is better», «they are the same», «not duplicates», skip,
  удаление обоих файлов; при merge переносятся теги и URL, худший файл уходит в trash;
- Options с вкладками general/files/downloaders/network/client API, менеджер сервисов
  (добавление, переименование и удаление пустых локальных сервисов) и менеджер сети
  (прокси, таймауты, пользовательские заголовки, URL classes, пауза сети, запуск API);
- Client API сервер на `127.0.0.1`: `/api_version`, `/get_services`,
  `/get_files/search_files`, `/get_files/file_metadata`, `/get_files/file`,
  `/add_tags/add_tags`, `/add_tags/delete_tags`, проверка access key;
- импорт одного файла по URL через `File → import url…`;
- редактирование тегов (локальный сервис), archive/inbox/перемещение в trash;
- меню Hydrus (`file`, `undo`, `pages`, `database`, `download`, `network`, `services`, `tags`,
  `pending`, `help`), сегментированный статус-бар и сохранение дерева страниц,
  загрузчиков и подписок в `gui session.json`.

### Что ещё не портировано

- полноценные site-specific gallery parsers и редактор URL classes (сейчас работает
  универсальный разбор `href`/`src`/`data-src` для страниц и постов);
- полный набор tag-панелей, рейтингов, notes и горячих клавиш Hydrus;
- импорт-папки, дедупликация по всем типам медиа и расширенные auto-resolution rules.

Сейчас проходит 478 unit-тестов и 2 integration-теста. Архитектура:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Проверка GUI без ручного клика

Для скриншот-проверок окно можно запустить в отдельной БД. Переменные окружения только
для разработки:

```powershell
$env:HYDRUS_QT_SCREENSHOT = 'C:\Temp\gui.png'
$env:HYDRUS_QT_IMPORT = 'C:\images\a.png;C:\images\b.jpg'
$env:HYDRUS_QT_QUERY = 'blue eyes'
$env:HYDRUS_QT_FOCUS_FIRST = '1'
.\target\debug\hydrus_client.exe --db_dir C:\Temp\hydrus-test
```

Также доступны `HYDRUS_QT_OPEN_FIRST`, `HYDRUS_QT_ADD_TAG`, `HYDRUS_QT_SCRIPT`
(`action|target|payload;...`), `HYDRUS_QT_DEFER` (одно отложенное действие после
показа окна) и `HYDRUS_QT_SCREENSHOT_DELAY_MS`. Для проверки сети можно поднять
локальный `python -m http.server` и запускать `urls`/`gallery` против `127.0.0.1`.
