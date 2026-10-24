# Портирование `hydrus_python/hydrus/client` в `hydrus_client`

Источник: `hydrus_python/hydrus/client` (≈300 тыс. строк Python, из них ≈171 тыс. — GUI).
Цель: перенести клиент Hydrus на Rust с сохранением структуры подсистем и поведения,
GUI — на Qt 6 (Widgets) через `cxx-qt`.

## Легенда

- `[ ]` — не начато.
- `[~]` — частично (есть каркас/подмножество API).
- `[x]` — готово, есть тесты и коммит.
- `[!]` — заблокировано (нужна внешняя зависимость или решение).

Правила: после каждого закрытого пункта — отдельный коммит с обновлением флагов.
Файлы держим до 200 строк, крупные модули разбиваем на `parts/`.

## 0. Инструменты и каркас

- [x] Зафиксировать тулчейн: Qt 6.12.0 (`C:\Qt\6.12.0\msvc2022_64`), MSVC 2022,
  `cxx-qt` 0.10, `cargo`, CMake/Ninja; описать переменные окружения сборки.
- [x] Настроить `hydrus_client/Cargo.toml`, `build.rs` (`cxx-qt-build`), зависимости
  (`hydrus_core`, `cxx`, `cxx-qt`, `cxx-qt-lib`, `chrono`, `serde`, `serde_json`, `rand`).
- [x] Создать структуру `src/` (`lib.rs`, `main.rs`, подмодули по подсистемам).
- [x] Реализовать инициализацию `hydrus_core` в точке входа (dirs, globals, logger).
- [x] Smoke-тест: приложение поднимает `QApplication` и пустое `QMainWindow`.

## 1. Инфраструктура клиента

- [x] `ClientGlobals` — глобальные флаги/ссылка на контроллер (без глобального мутабельного
  состояния там, где можно передать контекст).
- [x] `ClientConstants` — canvas types, сортировки, статусы импорта, цвета, page states,
  zoom modes, media actions, network contexts.
- [x] `ClientData` — `merge_counts`, `convert_zoom_to_percentage`, ASCII-хелперы,
  `resolution_to_pretty_string`, `Booru`, `Credentials`, `Imageboard`.
- [x] `ClientEnvironment` — настройка `REQUESTS_CA_BUNDLE`/путей CA.
- [x] `ClientPaths` — удаление в корзину, открытие файла/каталога, нативные свойства файла.
- [x] `ClientTime` — `CalendarDelta`, `ParseDate`, `MergeModifiedTimes`, pretty-форматтеры.
- [x] `ClientThreading` — `JobStatus`, `ThreadCall`, повторяющиеся задачи, pub/sub-мост в Qt.
- [x] `ClientLocation` — `LocationContext`, `LocationType`, сериализация.
- [~] `ClientApplicationCommand` — enum команд, горячие клавиши, обработчики.
  Портировано 48 основных команд и value-object; остальные добавляются вместе с GUI-обработчиками.
- [x] `ClientMacIntegration`/`ClientWindowsIntegration` — заглушки и Windows-вызовы.

## 2. Настройки

- [x] `ClientDefaults` — legacy options dict, дефолтные checker/media/duplicate options.
- [~] `ClientDefaults` — дефолтные URL classes, simple downloader formulae, bandwidth rules,
  domain manager data, favourite searches.
  Портированы bandwidth rules и custom headers; URL classes/formulae/favourites ждут
  PNG-загрузчик и менеджеры.
- [x] `ClientOptions` — `SerialisableObject` (type 22, version 8), группы booleans/integers/
  strings/keys/colours, typed get/set.
- [x] `ClientOptions` — миграции версий 1→8 и значения по умолчанию.
  Портированы шаги 1→8 (media_view, portable paths, routers, tag sorts, launch paths).
- [x] `ClientSerialisable` — импорт/экспорт объектов через PNG (после GUI-фазы).
  Реализован совместимый пиксельный payload (header height + длина + compressed bytes).

## 3. Метаданные

- [x] `ClientTags` — `TagStatus`, `ServiceKeysToTags`, операции объединения/фильтрации.
- [x] `ClientTagSorting` — `SortType`, `TagSort`, применение к тегам.
- [x] `ClientRatings` — `Rating`, `RatingsManager`, числовые/лайк-рейтинги.
- [x] `ClientContentUpdates` — `ContentUpdate`, `ContentUpdatePackage`, слияние.
- [x] `ClientMetadataConditional` — условные правила метаданных.
- [~] `ClientFileMigration`/`ClientMetadataMigration*` — маршрутизация метаданных.
  Портирована генерация content updates для move/duplicate; metadata routers дальше.

## 4. Поиск

- [x] `ClientNumberTest` — `NumberTest`, `NumberTestCollect`, парсинг/сравнение.
- [x] `ClientSearchPredicate` — `Predicate`, `SystemPredicate`, сериализация, `ToString`.
- [x] `ClientSearchTagContext` — контекст тегов и сервисов.
- [x] `ClientSearchFileSearchContext` — `FileSearchContext`, системные предикаты.
- [x] `ClientSearchAutocomplete` — autocomplete-теги и сортировка.
- [~] `ClientSearchParseSystemPredicates` — мост к `hydrus_external`.
  Портированы simple/number/filetype/hash/url/limit-предикаты; rating/file-service/date
  требуют менеджера сервисов и календарных значений.

## 5. Медиа-модель

- [x] `ClientMediaResult` — `MediaResult` и ленивые менеджеры.
- [x] `ClientMediaManagers` — `FileInfoManager`, `TimesManager`, `LocationsManager`,
  `NotesManager`, `RatingsManager`, `TagsManager`, `FileViewingStatsManager`.
- [x] `ClientMedia`/`ClientMediaSingle` — базовые медиа-объекты.
- [x] `ClientMediaList` — `MediaList`, `MediaCollection`.
- [x] `ClientMediaSort`, `ClientMediaCollect`, `ClientMediaFileFilter`.
  Сортировка, группировка и фильтрация по `FileSearchContext` портированы.
- [x] `ClientMediaResultCache`, `ClientMediaResultAPI`.

## 6. Файлы

- [x] `ClientFilesPhysical` — `FilesStorageBaseLocation`, `FilesStorageSubfolder`, префиксы.
- [x] `ClientFilesManager` — раскладка файлов/миниатюр, миграция хранилищ.
- [x] `ClientFiles` — операции чтения/записи, блокировки.
- [~] `ClientFilesMaintenance`/`ClientFilesMaintenanceManager` — очередь обслуживания.
  Портированы job ids, веса и очередь с планированием/отменой; сами regen-операции дальше.
- [x] `ClientTrashManager` — корзина и отложенное удаление.
- [~] `ClientImageHandling` и обёртки `hydrus_core` (метаданные, perceptual hashes).
  Портированы resize/thumbnail/blurhash/transparency и aHash-заглушка perceptual hash;
  точный Python DCT-хэш и EXIF/IPTC-обёртки дальше.

## 7. База данных

- [x] `ClientDBModule` — базовый trait/структура поверх `hydrus_core::DBModule`.
- [x] `ClientDBMaster` — hashes/namespaces/subtags/tags/texts/notes/urls/url_domains.
  Портированы hashes, namespaces/subtags/tags; texts/notes/urls — через отдельные модули.
- [x] `ClientDBServices` — services/service_info.
- [x] `ClientDBSerialisable` — json_dict/json_dumps/json_dumps_named/json_dumps_hashed.
  Портированы simple, named и hashed варианты.
- [x] `ClientDBFilesMetadataBasic` — files_info и флаги метаданных.
  Портированы `files_info`, forced mime и флаги icc/exif/xmp/iptc/transparency/etc.
- [x] `ClientDBFilesStorage` — current/deleted/pending/petitioned files, inbox.
- [x] `ClientDBFilesTimestamps`, `ClientDBFilesInbox`, `ClientDBFilesViewingStats`.
- [x] `ClientDBFilesPhysicalStorage` — конфигурация хранилищ.
- [~] `ClientDBMappingsStorage`/`ClientDBMappingsCounts`/кэши.
  Портированы mappings и counts; VR-кэши counts дальше.
- [~] `ClientDBTagSearch`, siblings/parents/display/suggestions.
  Портированы поиск по tag/namespace/wildcard, storage и применение siblings/parents;
  display/suggestions дальше.
- [x] `ClientDBDefinitionsCache`, `ClientDBMediaResults`, `ClientDBNotesMap`,
  `ClientDBRatings`, `ClientDBURLMap`, `ClientDBServicePaths`.
- [x] `ClientDBFilesDuplicates*`, `ClientDBSimilarFiles`.
  Potential/false-positive пары, alternate/duplicate-группы, perceptual/pixel hashes и
  brute-force similar search; VP-tree индекс можно добавить позже.
- [~] `ClientDBRepositories`, `ClientDBContentUpdates`, `ClientDBMaintenance`.
  Портированы content updates и maintenance (timestamps, deferred deletes);
  repositories дальше.
- [~] `ClientDB` — фасад, `create_db`, `update_db` 619→689, интеграция с `HydrusDB`.
  Готов синхронный фасад на `DBBase` с начальной схемой; перевод на `HydrusDB` и цепочка
  миграций 619→689 — следующий шаг.
- [x] Тест совместимости: открыть копию Python `client.db` только на чтение.
  Интеграционный тест собирает Python-shaped fixture и читает version/options/services/hashes/files_info.

## 8. Сервисы и сеть

- [x] `ClientServices` — `Service`, `ServicesManager`, локальные file/tag-сервисы.
- [x] `ClientNetworkingContexts` — контексты сетевых запросов.
- [x] `ClientNetworkingBandwidth`, `ClientNetworkingSessions`.
- [~] `ClientNetworkingDomain`, `ClientNetworkingLogin`, `ClientNetworkingURLClass`.
  Портированы URLClass (matching/normalisation); domain/login managers дальше.
- [x] `ClientNetworkingFunctions`, `ClientNetworkingJobs`.
  URL/domain/encoding helpers и state-machine/queue сетевых job-ов портированы.
- [x] `ClientNetworking` — `NetworkEngine` на `reqwest`/`tokio`.
  `NetworkEngine` + `HttpTransport`, blocking reqwest-транспорт и тестовый mock.
- [~] `ClientDownloading` — quick download manager.
  Портированы `GalleryIdentifier` и очередь quick-download; сетевой слой дальше.
- [~] `ClientAPI` + `networking/api` — локальный Client API сервер.
  Портирован `ApiManager` с маршрутами api_version/get_services/search_files и access key;
  HTTP-транспорт сервера дальше.

## 9. Импорт, парсинг, дубликаты

- [~] `ClientParsing`, `ClientParsingResults`, `ClientParsingLegacy`.
  Портированы описания результатов и `ParsedPost`; формулы/парсеры и legacy дальше.
- [~] `ClientImporting`, `ClientImportFileSeeds`, `ClientImportFiles`.
  Портированы FileSeed/FileSeedCache и статусы; импорт-очереди и files-jobs дальше.
- [~] `ClientImportGallery*`, `ClientImportSimpleURLs`.
  Портированы GalleryImport/SimpleDownloaderImport, очереди и seed-логи; network work дальше.
- [~] `ClientImportWatchers`, `ClientImportSubscriptions`.
  Портированы WatcherImport/MultipleWatcherImport, Subscription и менеджер; работа с сетью дальше.
- [~] `ClientImportLocal` — импорт-папки.
  Портированы ImportFolder/ImportFoldersManager и планирование; HDDImport-очередь дальше.
- [~] `importing/options/*` — ImportOptions/TagImportOptions/FileFiltering.
  Портированы file filtering, tag import options и контейнер; manager/legacy дальше.
- [~] `ClientDuplicates*`, `ClientPotentialDuplicates*`.
  Портированы merge options, pair decisions и очередь пар; авто-разрешение и сравнение дальше.

## 10. GUI (Qt 6 Widgets, `cxx-qt`)

- [x] C++-слой `gui/qt`: `QApplication`, `QMainWindow`, `QWidget`, `QLabel`, кнопки,
  layout-ы, меню, статус-бар, splitter, tab-widget.
  Добавлены `HydrusPanel`, QLabel/QPushButton/QCheckBox/QLineEdit и layout-based страницы.
- [x] `QtInit`, `QtPorting`, `QtInitImportTest` — инициализация, стиль, логирование Qt.
  Qt 6-only (`is_supported_qt_version`), стиль и QSS применяются к `QApplication`.
- [~] `ClientGUIFunctions`, `ClientGUIFunctionsImage` — конвертация изображений/иконок.
  Портированы цветовые/текстовые/тултип-хелперы; image-часть дальше.
- [x] `ClientGUICore`, `ClientGUICallAfter`, `ClientGUIAsync` — мост потоков и call-after.
- [x] `ClientGUIStyle`, `ClientGUISplash`, `ClientGUIShortcuts`, `ClientGUIShortcutControls`.
  Портированы QSS-стиль, Shortcut/ShortcutSet, ShortcutEditor и SplashModel.
- [~] `ClientGUITopLevelWindows`, `ClientGUITopLevelWindowsPanels`.
  Портирована модель top-level окна (типы/геометрия/accept-reject); Qt-диалоги дальше.
- [~] `ClientGUIScrolledPanels`, `ClientGUICommon` — базовые виджеты.
  Портирована модель edit/manage/review панели; Qt-виджеты и `ClientGUICommon` дальше.
- [~] `ClientGUIListCtrl`, `ClientGUIListBoxes`, `ClientGUIListManager`.
  Портированы `ListModel` (колонки/сортировка/выбор/фильтр) и `ListBox`; Qt-view дальше.
- [~] `ClientGUI` — `FrameGUI`: меню, панель страниц, статус-бар, popup-очередь.
  Готов каркас окна с меню, боковым деревом и вкладками; страницы и popup-очередь дальше.
- [~] `ClientGUIPages`, `ClientGUIPagesTreeView`, `ClientGUIPagesTreeModel`.
  Портирована модель `Page`/`PagesNotebook`; Qt tree view/model дальше.
- [~] `ClientGUIMediaResultsPanel`, `ClientGUIMediaResultsPanelThumbnails`.
  Готов Qt-панель со списком/иконками и вкладка Files; thumbnails-модель и выбор медиа дальше.
- [~] `ClientGUICanvas*`, `ClientGUIMPV`, `ClientGUIMediaModalActions`.
  Готов Qt `HydrusCanvas` (QGraphicsView, загрузка/масштабирование pixmap, вкладка Canvas);
  hover/media modal actions и mpv дальше.
- [~] `ClientGUIPopupMessages`, `ClientGUISubscriptions`, `ClientGUITime`.
  Портирована очередь popup-сообщений; subscriptions/time-панели дальше.
- [~] `ClientGUINetworking*`, `ClientGUILogin`.
  Портированы модели networking/login панелей (контексты, headers, login state); Qt-панели дальше.
- [~] `ClientGUIDuplicates*`, `ClientGUIImport*`, `ClientGUIParsing*`.
  Портированы модели duplicates/import/parsing панелей; Qt-панели дальше.
- [~] `panels/options/*` — панели настроек.
  Портирован реестр/поиск options-панелей; Qt-панели дальше.
- [~] `ClientGUIStringPanels`, строковые диалоги и редакторы.
  Портированы StringPanel и StringToStringDictPanel; Qt-диалоги дальше.

## 11. Контроллер и запуск

- [x] `App`/`PubSubEventCatcher` — QApplication и доставка pub/sub в Qt-поток.
  Добавлен потокобезопасный `PubSubEventCatcher`; контроллер публикует message-события.
- [x] `Controller::init_model` — БД, опции, кэши, сервисы, файловый менеджер.
  Загружаются БД/опции/сервисы, файловый менеджер, maintenance/trash и shortcuts.
- [~] `Controller::init_view` — менеджеры, стиль, shortcuts, `FrameGUI`.
  Стиль/shortcuts/pubsub готовы; реальные страницы и менеджеры в Qt ещё не связаны.
- [x] `Controller::run`/`thread_boot_everything`/shutdown.
  Синхронная boot-последовательность, сохранение опций и GUI-сессии при выходе.
- [~] `hydrus_client_boot` — CLI-аргументы, logger, temp dir, обработка сигналов.
  Готовы CLI, logger и БД; temp dir/сигналы дальше.
- [x] Восстановление сессии страниц и повторный запуск.
  `GuiSession` сохраняет страницы/индекс и подхватывается контроллером из `gui session.json`.

## 12. Финализация

- [x] Документация crate и архитектурные заметки.
- [~] `cargo test -p hydrus_client`, `cargo clippy`, `cargo fmt`.
  Тесты и `fmt` проходят; `clippy` упирается в уже существующие замечания
  `hydrus_core`, а не в `hydrus_client`.
- [~] Проверка лимита 200 строк на файл.
  Большинство файлов укладывается; `client_options/defaults.rs` (304),
  `client_constants.rs` (292), `client_options/mod.rs` (277),
  `client_threading.rs` (233) требуют разбиения при следующем проходе.
- [ ] Удалить `PORTING_TODO.md` после закрытия всех пунктов.
