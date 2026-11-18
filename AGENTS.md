# AGENTS.md — Правила разработки Factory Atlas

Этот файл фиксирует обязательные инженерные, лицензионные и UX-ограничения проекта **Factory Atlas**.

## 1. Назначение и старт нового проекта (Empty-Start)
- **При первом запуске и при создании нового проекта приложение всегда стартует полностью пустым** (`nodes: []`, `edges: []`, `customResources: []`, `customMachines: []`, `customRecipes: []`).
- В пользовательских проектах и интерфейсе не должно быть предзагруженных стартовых цепочек, встроенного игрового каталога рецептов или автоматического добавления узлов/связей.
- Готовые цепочки и тестовые наборы (`src/lib/data/testFixtures.ts`) допустимы **только** внутри автоматических тестов (`tests/`) и бенчмарка (`scripts/benchmark.ts`).
- Импорт рецептов из локальных `.jar` / `.zip` / `.json` выполняется **только** по явному действию и подтверждению пользователя.

## 2. Локальный визуальный индекс и сопоставление моделей/текстур
- Локальная база в `assets-local/` (`visual-index.json`, `upstream-originals/`) является **исключительно справочной базой визуального поиска** (`src/lib/assets/visualMatcher.ts`). Она никогда не создаёт рецепты, машины или ресурсы автоматически.
- Приоритет сопоставления по пользовательскому названию/ID:
  1. точный namespaced ID (`create:cogwheel`),
  2. точный путь модели/текстуры/blockstate,
  3. нормализованный ID/имя файла,
  4. точные локализованные названия из `lang` (`ru_ru.json` / `en_us.json`),
  5. частичное RU/EN совпадение.
- При однозначном совпадении визуал назначается автоматически с предпросмотром 3D; при нескольких кандидатах выбор **всегда** остаётся за пользователем через карточки с миниатюрами.
- Назначенная визуальная ссылка, ручной override или отключение сохраняются внутри `.factoryatlas` (`customResources[].visual`, `customMachines[].visual`).

## 3. Честный 3D-рендерер моделей Minecraft JSON и `.nbt`
- Модуль `src/lib/models/mcModelResolver.ts` разрешает цепочки `parent` (с защитой от циклов), переменные `#texture` (с защитой от циклов), `elements`, UV, повороты элементов, `blockstates` (`variants` и `multipart`) лениво и с ограниченным LRU-кэшем.
- **Никогда не выдавайте плоские 2D PNG за 3D-модели** и не имитируйте процедурные/BlockEntity-модели геометрией там, где статических `elements` нет.
- Статус каждой модели классифицируется честно: `3d-json-elements`, `2d-item-generated`, `unsupported-obj` (`neoforge:obj`), `procedural-or-blockentity` или `none`.

## 4. Политика интеллектуальной собственности и ассетов
- Папки `assets-local/` и `user-projects/` **строго исключены в `.gitignore`** и никогда не должны попадать в Git, PR, `dist/` или установщики Windows.
- Код и собственные SVG-заглушки (`public/assets-fallback/*`, `src-tauri/icons/*`) — **MIT**.
- Ассеты из апстрим-модов (особенно `All Rights Reserved` у Create / Aeronautics / Simulated и `CC BY-NC-SA 4.0` у CreateBigCannons) запрещено включать в публичный пакет.
- Сборка (`npm run build`), проверка типов (`npm run check`), тесты (`npm test`) и бенчмарк (`npm run benchmark`) **обязаны полностью проходить при отсутствии `assets-local/`**.

## 5. Разделение режимов исполнения и Windows-сборки
- Настольное приложение для Windows x64 (`src-tauri/`) использует нативный WebView2 + встроенный `dist/` без Node.js и без открытых HTTP-портов.
- Две формы распространения строго изолированы:
  - **Основная (Установщик NSIS, `currentUser`)**: `%APPDATA%\dev.goshadows.factoryatlas\` + отдельный WebView2 `data_directory` в `%LOCALAPPDATA%\dev.goshadows.factoryatlas\webview\`, ярлык в меню «Пуск», опциональный ярлык на Рабочем столе, деинсталлятор сохраняет пользовательские данные по умолчанию.
  - **Дополнительная (Портативная ZIP)**: активируется `portable.flag` / `--portable` / `./data/` без `uninstall.exe`, хранит проекты, оверрайды и `webview/` строго в `<exe_dir>/data/`.
- `vite` и `server/local-app-server.mjs` предназначены только для разработки / браузерного предпросмотра (`127.0.0.1` по умолчанию).
- Никогда не утверждайте, что Windows-установщик или портативный ZIP проверены в рантайме, если сборка не запускалась на Windows.

## 6. Обязательные команды проверки перед коммитом
```bash
npm run check      # svelte-check + tsc --noEmit (должно быть 0 errors / 0 warnings)
npm test           # vitest run
npm run benchmark  # 500-node / 1000-edge solver + export benchmark
npm run build      # vite build (dist/)
git status -s      # проверка отсутствия assets-local/ и user-projects/
```
