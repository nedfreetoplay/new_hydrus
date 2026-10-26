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
- импорт одного файла по URL через `File → import url…`;
- редактирование тегов (локальный сервис), archive/inbox/перемещение в trash;
- меню Hydrus (`file`, `undo`, `pages`, `database`, `network`, `services`, `tags`,
  `pending`, `help`), сегментированный статус-бар и сохранение дерева страниц в
  `gui session.json`.

### Что ещё не портировано

- страницы загрузчиков: gallery / watcher / subscriptions / simple downloader / urls
  (сейчас это честные страницы-заглушки);
- duplicates processing (пары и merge UI);
- окна Options и менеджеры сервисов/сети;
- асинхронные очереди импорта и загрузки, Client API сервер;
- полный набор tag-панелей и горячих клавиш Hydrus.

Сейчас проходит 465 unit-тестов и 2 integration-теста. Архитектура:
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

Также доступны `HYDRUS_QT_OPEN_FIRST`, `HYDRUS_QT_ADD_TAG` и `HYDRUS_QT_SCRIPT`
(`action|target|payload;...`).
