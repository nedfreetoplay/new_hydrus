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

Готовы: инфраструктура клиента, настройки (включая все миграции 1→8), метаданные,
поиск, медиа-модель, файлы, БД-слой, сеть (engine на reqwest), импорт/парсинг/дубликаты,
GUI-модели и Qt-каркас с canvas, PNG-сериализация и восстановление сессии.

Остались в основном Qt-виджеты для уже готовых моделей и серверный HTTP-транспорт
Client API.

БД: схема, сервисы, хэши, master-теги, mappings, files_info/storage/timestamps/inbox,
viewing stats, storage configuration, notes/ratings/urls/service paths, definitions cache,
сборка `MediaResult`, content updates, tag search/siblings/parents, duplicates/similar files
и maintenance.

Также начаты: networking (contexts/functions/bandwidth/sessions/jobs/engine на reqwest,
URL classes), importing (options, import folders, file seeds), parsing results, duplicates
UI-модели, list/pages/canvas панели, PNG-сериализация и восстановление GUI-сессии.

Актуальный план и флаги: [PORTING_TODO.md](PORTING_TODO.md).
Архитектура: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).
