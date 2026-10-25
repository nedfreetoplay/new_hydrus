# Архитектура `hydrus_client`

Порт повторяет слои Python-клиента, но без глобального `CG.client_controller`
там, где можно передать контроллер явно.

## Слои

```text
hydrus_core            константы, serialize, БД-примитивы, файлы, изображения
  ↑
hydrus_client::client_*    константы, данные, пути, время, потоки, опции
  ↑
hydrus_client::metadata    теги, рейтинги, content updates, сортировка
hydrus_client::media       MediaResult, менеджеры, списки, сортировка
hydrus_client::files       физические хранилища
  ↑
hydrus_client::db          ClientDB (сейчас на DBBase), services/hashes/files_info
  ↑
hydrus_client::controller  boot модели, локальные сервисы, сохранение опций
  ↑
hydrus_client::gui::view_model  ViewModel: данные и состояние страниц без Qt
  ↑
hydrus_client::gui::qt      View: рендер ViewModel через cxx-qt + C++-фасад
```

## Ключевые решения

- **MVVM.** Контроллер и модели панелей собираются в
  `gui::view_model::MainViewModel` (заголовок, статус, вкладки, меню и
  описания страниц). `gui::qt` только превращает эти значения в виджеты, без
  логики приложения и без знания о сессии/БД.
- **Qt 6.12 + cxx-qt 0.10.** `cxx-qt-lib` не оборачивает QtWidgets, поэтому
  используется собственный минимальный C++-фасад (`src/gui/qt/bridge.*`),
  который растёт вместе с портируемыми виджетами.
- **Опции — словарь.** `ClientOptions` хранит те же группы, что Python
  (`booleans`, `integers`, `noneable_strings`, `colours`, …). Неизвестные ключи
  переживают загрузку/сохранение.
- **БД.** Начальная схема создаётся в `client.db`; фасад синхронный, на
  `hydrus_core::DBBase`. Следующий шаг — `HydrusDB` и миграции 619→689.
- **Сборка.** Qt ищется через `QMAKE` (`build.rs`), для сборки нужен MSVC
  2022; `scripts/cargo.ps1` загружает окружение и Qt в `PATH`.

## Что дальше

Актуальные флаги и порядок работ — в [PORTING_TODO.md](../PORTING_TODO.md).
Крупные оставшиеся блоки: полный DB-слой, сеть/сервисы, импорт/парсинг,
дубликаты и остальные GUI-панели.
