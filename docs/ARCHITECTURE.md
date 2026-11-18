# Архитектура Factory Atlas

## 1. Общая структура

```
src/
├── App.svelte                       # Оболочка, пустой старт, вкладки, autosave, модалы, горячие клавиши
├── lib/
│   ├── types/atlas.ts               # TypeScript-схемы графа, рецептов, профилей, визуального индекса, NBT
│   ├── i18n/translations.ts         # Словари RU (по умолчанию) / EN
│   ├── data/
│   │   ├── createCatalog.ts         # createEmptyProject(), DEFAULT_PROFILE, опциональные формулы скорости Create 6.0.11
│   │   └── testFixtures.ts          # Синтетические фикстуры ТОЛЬКО для тестов и бенчмарка
│   ├── engine/
│   │   ├── machineSpeeds.ts         # Формулы тиков и SU машин Create 6.0.11 + ручной/неизвестный режимы
│   │   └── solver.ts                # Инкрементальный решатель графа + Тарьян SCC + балансы
│   ├── models/
│   │   ├── mcModelResolver.ts       # Резолвер Minecraft JSON моделей, parent, #texture, blockstates, LRU-кэш
│   │   ├── renderer3d.ts            # 3D-рендер кубоидов и сцен .nbt на HTML5 Canvas с аффинными UV
│   │   └── textureCache.ts          # Ограниченный LRU-кэш изображений HTMLImageElement
│   ├── assets/
│   │   ├── assetManager.ts          # 4-уровневая резолюция текстур и localStorage/диск оверрайды
│   │   └── visualMatcher.ts         # Детерминированный 5-ступенчатый поиск в локальном визуальном индексе
│   ├── io/
│   │   ├── projectFormat.ts         # Версионируемый .factoryatlas v2 и миграция v1 -> v2
│   │   ├── exporters.ts             # Генераторы SVG, PNG (HTML5 Canvas) и CSV (UTF-8 BOM)
│   │   ├── jarImporter.ts           # Защищённый разбор локальных .jar / .zip / .json с подтверждением
│   │   └── nbtParser.ts             # Чистый TS NBT-ридер + потоковый Gunzip с защитой от бомб
│   ├── state/history.ts             # Ограниченный стек Undo/Redo (до 60 снимков)
│   ├── storage/desktopBridge.ts     # Адаптер IPC Tauri Desktop (Installed / Portable) vs Web Preview
│   └── components/                  # Svelte 5 компоненты UI (включая ModelViewer3D, ResourceCreatorModal, SchematicViewer)
server/local-app-server.mjs          # Сервер ТОЛЬКО для dev / браузерного предпросмотра (127.0.0.1 по умолчанию)
src-tauri/                           # Нативная обёртка Tauri v2 + NSIS + изоляция portable ./data/
```

---

## 1.1. Режимы исполнения (Настольное приложение vs Браузерный предпросмотр)

- **Installed (Установленная версия Windows x64, NSIS `currentUser`):**
  - `src-tauri/src/lib.rs` хранит `autosave.json`, `projects/` и `assets-overrides/` в `%APPDATA%\dev.goshadows.factoryatlas\`, а кэш WebView2 (`data_directory`) — в `%LOCALAPPDATA%\dev.goshadows.factoryatlas\webview\`.
  - Встроенный `dist/` запускается напрямую через WebView2 без Node.js и без локальных HTTP-портов.
- **Portable (Портативная версия Windows x64 ZIP):**
  - Активируется флагом `--portable`, файлом `portable.flag` рядом с `factory-atlas.exe` или папкой `./data/` без `uninstall.exe`.
  - Хранит проекты, автосохранение, оверрайды и `webview/` строго внутри `<exe_dir>/data/`.
- **Browser Dev / Preview (`vite dev` или `server/local-app-server.mjs`):**
  - Предназначен исключительно для разработки и предпросмотра; привязывается к `127.0.0.1` по умолчанию (`HOST=0.0.0.0` только для облачного превью).

---

## 2. Пайплайн 3D-моделей и `.nbt` сцен

1. **Ленивая загрузка с ограниченным LRU-кэшем (`McModelResolver` + `textureCache.ts`):**
   - `rawModelCache` (240), `modelCache` (160), `blockstateCache` (160) и `imageCache` (160) автоматически вытесняют устаревшие записи и не допускают переполнения памяти.
2. **Разрешение геометрии:**
   - Поддерживаются `blockstates/*.json` (`variants` и `multipart` с `OR`/`AND` и `x`/`y` поворотами), цепочки `parent` (включая примитивы `cube_all`, `cube_bottom_top`, `cube_column`, `orientable`, `slab`, `carpet`), защита от циклов в `parent` и `#texture`, повороты отдельных элементов (`origin`, `axis`, `angle`) и авторасчёт пропущенных `uv`.
3. **Честная классификация:**
   - Статические модели (`3d-json-elements`) рендерятся аффинным текстурированием граней (`drawTexturedParallelogram` с `imageSmoothingEnabled = false`) и сортировкой глубины.
   - Плоские предметы (`2d-item-generated`), Wavefront OBJ (`unsupported-obj`) и процедурные/BlockEntity-блоки (`procedural-or-blockentity`) явно помечаются в UI и не маскируются под статическую 3D-геометрию.

---

## 3. Графовый расчётчик и производительность (`src/lib/engine/solver.ts`)

1. **Разбиение на компоненты связности (Union-Find) и `IncrementalSolverCache`:** пересчитывается только изменённая подцепочка графа.
2. **Обнаружение циклов (Tarjan SCC):** циклы (`CYCLE_DETECTED`) фиксируются без зависания.
3. **Строгая проверка единиц:** несовпадение `предм./мин` и `мБ/мин` блокирует поток с ошибкой `UNIT_MISMATCH`.
4. **Фактические измерения (`npm run benchmark` на 500 узлах / 1000 рёбрах):**
   - Холодный полный расчёт: `~35 ms`
   - Инкрементальное обновление 1 компоненты (49/50 из кэша): `~4 ms`
   - Сериализация + парсинг `.factoryatlas` (316 КБ): `~5 ms`
   - Экспорт SVG + CSV: `~7 ms`
