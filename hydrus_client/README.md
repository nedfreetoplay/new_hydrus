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
  вложенные «page of pages» и вкладка `+` с выбором типа страницы; drag & drop
  вкладок, история, менеджер страниц и именованные сессии;
- файловые страницы: реальный поиск по локальной БД (теги, `-tag`, wildcard,
  namespace, `system:inbox`, `system:archive`, `system:limit=N`) и системные
  предикаты размера, разрешения, ratio, mime, hash, duration, fps, frames,
  пикселей, слов, числа тегов/URL/заметок, рейтингов и дат; known URL/domain/regex/
  URL class, file viewing stats/file service, best quality of group и
  num file relationships (duplicates/alternates/potential/false positive), а также
  similar to files/data с distance; OR-группы `{tag1 | tag2}` с
  диалогом OR...; выпадающее автодополнение с вкладками tags/namespaces/wildcards/
  system и счётчиками файлов; селектор file domain (my files / all local files /
  trash), tag service; избранные поиски сохраняют запрос, домен и tag service и
  доступны из меню; recent searches и undo/redo поиска хранят историю запросов в
  `search history.json`;
- сетка миниатюр, предпросмотр выбранного файла, статус страницы; выделение
  нескольких файлов и archive/inbox/delete для всей группы; сортировка по hash,
  времени импорта, размеру, mime, разрешению, ratio, пикселям, duration, frames и
  словам (по возрастанию/убыванию) с сохранением в сессии; поверх миниатюр
  рисуются маркеры inbox, dup и рейтинг; context menu умеет copy tags,
  paste tags из клипборда и apply tags to selection (строки с `-` удаляют тег);
  metadata-панель по умолчанию показывает display-теги, а options → tag presentation
  управляют сортировкой (lexicographic/namespace/subtag), показом namespace’ов и
  заменой `_` на пробелы; масштаб миниатюр, masonry-раскладка и
  горизонтальный скролл настраиваются в options → thumbnails; в шапке результатов
  видны число файлов и прогресс импорта, а во время поиска показывается loading
  panel; миниатюры можно перетаскивать в другие страницы или наружу как файлы, а
  файлы — бросать на страницу для импорта;
- медиа-вьюер: зум (fit/100%), панорамирование (pan), поворот, следующая/предыдущая,
  выбор фона (dark/black/mid/light/checkerboard), полная metadata-панель (теги по
  сервисам, рейтинги, заметки), archive/inbox/delete, F11; GIF/WebP-анимации с
  pause/play (кнопка и P); open externally, copy hash/url; hover-панель с
  archive/inbox/delete/tags/notes появляется при наведении на верх вьюера;
  prefetch соседних файлов и восстановление геометрии окна;
- импорт локальных файлов через меню и drag-and-drop: хэширование, копирование в
  файловое хранилище, `files_info`, сервис файлов, inbox и генерация миниатюр;
- импорт локальных файлов и загрузка URL идут через фоновую очередь: прогресс виден
  в статус-баре, интерфейс не блокируется;
- страницы загрузчиков: полноценная `urls`-очередь с повтором/очисткой, `gallery`
  (встроенные парсеры danbooru, gelbooru, e621, rule34, safebooru, pixiv и generic,
  подстановка `{query}`, переходы по next-страницам), `watcher` с автоматическими
  проверками по периоду, `subscriptions` с менеджером подписок и `simple downloader`;
  определения загрузчиков экспортируются/импортируются как JSON из меню Download;
- загрузки идут через per-domain пулы: глобальный лимит, лимит по умолчанию и
  переопределения для домена, плюс bandwidth rules (requests/data) с ожиданием;
- duplicates processing: пары из `potential_duplicate_pairs`, две стороны с превью,
  тегами и URL, «this is better», «they are the same», «not duplicates», skip,
  удаление обоих файлов; при merge переносятся теги и URL, худший файл уходит в trash;
  auto-resolution редактор поддерживает AND/OR, metadata conditions, поиск по
  очереди, preview и pending/denied/taken review; правила выполняются при
  включённом флаге;
- import folders: сканирование папок по периоду, ручная проверка, пауза и удаление,
  команда «check all» и меню всех папок в File, фильтры (расширения, min/max размер,
  пропуск недавно изменённых), а после успешного импорта файл можно оставить,
  переместить в указанную папку или удалить;
- редактор tag siblings/parents по локальным tag-сервисам: пары добавляются и
  удаляются с проверкой циклов, а metadata-панели показывают display tags с учётом
  siblings и tooltip'ом исходных storage-тегов;
- Options с вкладками general/files/media viewer/file maintenance/downloaders/network/
  client API, менеджер сервисов
  (добавление, переименование и удаление пустых локальных сервисов) и менеджер сети
  (прокси, таймауты, заголовки, полноценный редактор URL classes с проверкой URL,
  per-domain пулы, bandwidth rules, пауза сети, запуск API);
- Client API сервер на `127.0.0.1`: `/api_version`, `/get_services`,
  `/get_files/search_files`, `/get_files/file_metadata`, `/get_files/file`,
  `/add_tags/add_tags`, `/add_tags/delete_tags`, проверка access key;
- импорт одного файла по URL через `File → import url…`;
- редактирование тегов по локальным tag-сервисам: массовый manage tags для
  выделения, счётчики тегов по выбранным файлам и выбор любого локального сервиса;
  рейтинги (like/dislike, inc/dec и числовые), заметки, archive/inbox/перемещение в trash;
  редактор archived/modified timestamps;
- меню Hydrus (`file`, `undo`, `pages`, `database`, `download`, `network`, `services`, `tags`,
  `pending`, `help`), сегментированный статус-бар и единый `gui session.json`
  (legacy `GuiSession` + дерево страниц, загрузчики и подписки) с миграциями старых форматов.
- system tray с show/hide и exit; уведомления дублируются в трей; история
  сообщений доступна в Help → popup messages... с очисткой, лимит задаётся в
  options → popup notifications; command
  palette (Ctrl+K) запускает основные действия и maintenance-команды; Help →
  about показывает версии software/network/API/DB, SQLite, Qt, Rust, пути и
  статистику сервисов/локаций/шорткатов с вкладками license и credits; при
  старте показывается splash-окно с версиями и статусами загрузки страниц.
- дерево страниц: drag & drop вкладок и перенос между notebook’ами, история
  назад/вперёд (Alt+Left/Right), менеджер страниц с фильтром и массовым закрытием,
  дублирование страницы и именованные сессии с загрузкой, добавлением, удалением и
  резервными копиями в `sessions/`.
- media viewer следует настройкам zoom step/recenter/always-on-top/frameless, поддерживает
  поворот и полный набор основных горячих клавиш; database check/vacuum выполняют
  реальные `PRAGMA quick_check` и `VACUUM`; analyze/optimize, обзор свободных
  страниц, deferred deletes и статистика «how boned am I?» доступны из меню Database;
  fix missing archive times проставляет archive timestamps для архивных файлов;
  file maintenance jobs после импорта (modified time, transparency) выполняются
  фоновым тиком и вручную из меню с обзором очереди и счётчиками;
  file viewing statistics показывает views/viewtime/last viewed по media и preview,
  пишется только при включённой опции options → file viewing statistics.

### Что ещё не портировано

- полноценный tag display manager (порядок namespace/subtag, decorators) за пределами
  редактора relations;
- расширенные file maintenance jobs (regenerate thumbnails, re-scan file metadata) и
  оставшиеся горячие клавиши Hydrus.

Сейчас проходит 611 unit-тестов и 2 integration-теста. Архитектура:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). План портирования GUI с
чек-листами и флагами: [docs/GUI_PORT_PLAN.md](docs/GUI_PORT_PLAN.md).

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
