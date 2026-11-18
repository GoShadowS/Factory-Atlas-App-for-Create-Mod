# Политика интеллектуальной собственности и ассетов (`ASSET-POLICY`)

## 1. Принцип строгого разделения

**Factory Atlas** чётко разграничивает три категории ресурсов:

1. **Оригинальный код, документация, иконки и заглушки Factory Atlas (MIT):**
   - Вся кодовая база, интерфейс, документация, SVG-иконка `public/assets-fallback/factory-atlas-icon.svg`, SVG-заглушки (`item-fallback.svg`, `fluid-fallback.svg`, `machine-fallback.svg`) и иконки установщика `src-tauri/icons/` распространяются по лицензии **MIT** (`LICENSE`).
2. **Оригинальные модели, blockstates, текстуры и переводы сторонних модов (Third-Party Assets):**
   - Текстуры и ручные 3D-модели **Create** (`Creators-of-Create/Create`, `src/main/resources/assets/`) и **Simulated-Project** (`aeronautics`, `simulated`, `offroad`) охраняются условием **All Rights Reserved**.
   - Ручные модели и текстуры **CreateBigCannons** (`src/main/resources/assets/createbigcannons`) распространяются под лицензией **CC BY-NC-SA 4.0 International** (запрещено коммерческое использование).
   - **Политика проекта:** сторонние ассеты **никогда** не коммитятся в Git-репозиторий, **никогда** не попадают в PR, `dist/` и Windows-установщики / портативный ZIP.
3. **Пользовательские замены (`assets-overrides/` и UI):**
   - Пользователь может локально подменить любую PNG-текстуру или 3D JSON модель в интерфейсе (сохраняется в `.factoryatlas` / `localStorage` / `%APPDATA%` / `./data/`).

---

## 2. Технические меры соблюдения политики

- Папки `assets-local/` и `user-projects/` строго исключены в `.gitignore`.
- CI workflow (`.github/workflows/ci.yml`) проверяет отсутствие закоммиченных файлов в `assets-local/` и `user-projects/`.
- Конфигурация установщика (`src-tauri/tauri.conf.json`) включает только `dist/` (собственный код + MIT SVG) и `assets-overrides/README.txt`.
- Сборка (`npm run build`), проверка типов (`npm run check`) и все unit-тесты (`npm test`) работают полностью без `assets-local/`.

---

## 3. Локальный справочный визуальный индекс (`assets-local/`)

На локальной машине разработчика gitignored-папка `assets-local/` может содержать извлечённые модели/blockstates/текстуры/`lang` (`assets-local/upstream-originals/` + `assets-local/visual-index.json` с 1 471 позицией по 8 неймспейсам).

Эта база используется **строго** для визуального поиска (`src/lib/assets/visualMatcher.ts`) и 3D-рендера при создании элементов пользователем, и **никогда** не создаёт рецепты, машины или производственные цепочки в проектах автоматически.
