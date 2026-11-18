# Factory Atlas

**Factory Atlas** — локальное офлайн-приложение для проектирования производственных цепочек, ручного создания/расчёта рецептов и 3D-просмотра схем `.nbt` для **Minecraft Create** и аддонов.

> Интерфейс по умолчанию: **русский** (переключение `RU / EN`, тёмная и светлая темы).

---

## Ключевые принципы приложения

### 1. Полностью пустой старт (Empty-Start)
- При **первом запуске** и при нажатии **«Новый проект»** открывается **полностью пустой проект**: `0` узлов, `0` связей, `0` предзагруженных рецептов, `0` предзаполненных предметов или машин.
- Пользователь сам создаёт свои ресурсы (предметы `предм./мин` и жидкости `мБ/мин`), машины и рецепты.
- Импорт рецептов из локальных `.jar` / `.zip` / `.json` выполняется **только по явному действию и подтверждению пользователя** (после предварительного отчёта).

### 2. Локальный визуальный поиск 3D-моделей и текстур
- При создании или редактировании ресурса/машины можно ввести русское/английское название, ID (`create:cogwheel`) или путь модели — `src/lib/assets/visualMatcher.ts` ищет совпадение по локальному справочному индексу (`assets-local/visual-index.json`, если он подготовлен на машине разработчика):
  1. точный namespaced ID,
  2. точный путь модели / текстуры / blockstate,
  3. нормализованный ID / имя файла,
  4. точные локализованные названия из `lang` (`ru_ru.json` / `en_us.json`),
  5. частичное RU/EN совпадение.
- Однозначное совпадение подключает предпросмотр автоматически; при неоднозначности (например, «шестерня») открывается выбор из кандидатов с миниатюрами. Можно в любой момент загрузить свой `.png`, свою 3D `.json`-модель или отключить визуал (заглушка). Назначения сохраняются в файле `.factoryatlas`.
- **Важно:** в публичный репозиторий и установщик сторонние ассеты модов **не входят** (`assets-local/` в `.gitignore`). При отсутствии `assets-local/` приложение и тесты полностью работают с нейтральными SVG-заглушками (`public/assets-fallback/`).

### 3. Реальный 3D-рендер Minecraft JSON моделей и `.nbt` схематик
- Резолвер (`src/lib/models/mcModelResolver.ts`) + Canvas 3D-рендерер (`src/lib/models/renderer3d.ts`):
  - разрешает цепочки `parent` (включая базовые ванильные примитивы `cube_all`, `cube_bottom_top`, `cube_column`, `orientable`, `slab`, `carpet`),
  - разрешает переменные `#texture` и `blockstates/*.json` (`variants` и `multipart` с `OR`/`AND`, поворотами `x`/`y`),
  - учитывает повороты элементов (например, зубцы шестерён под 22.5°/45°), UV-развёртку и анимированные полосы жидкостей,
  - кэширует лениво через ограниченный LRU-кэш (`BoundedLruCache`).
- **Честные ограничения:** плоские 2D-предметы (`item/generated`), модели `neoforge:obj` и процедурные/BlockEntity-блоки помечаются явно и **никогда** не выдаются за статическую 3D JSON геометрию.

### 4. Графовый расчётчик и безопасность
- Строгое разделение `items/min` (`предм./мин`) и `mB/min` (`мБ/мин`) — несовместимые порты блокируются (`UNIT_MISMATCH`).
- Проверенные формулы Create `6.0.11` (`Millstone`, `MechanicalPress`, `MechanicalMixer`, `MechanicalSaw`, `Deployer`, `Spout`, `ItemDrain`, `CrushingWheel`, `EncasedFan`, `MechanicalCrafter`) доступны как опциональные модели скорости при создании машины; также поддерживаются `manual` (ручной темп) и честный статус `unknown`.
- Tarjan SCC (`CYCLE_DETECTED`), инкрементальное кэширование компонент, вероятностные и `weighted`-выходы, экспорт SVG / PNG / CSV (UTF-8 BOM), `.factoryatlas` v2 (+ миграция v1 -> v2), защита от ZIP/GZIP-бомб и path traversal.

---

## Проверенные апстрим-репозитории (аудит визуальных и языковых ресурсов)

Во время разработки выполнен локальный аудит и извлечение в `assets-local/` (8 187 файлов, 1 471 позиция, **не включено** в Git/PR/инсталлятор):
- `Creators-of-Create/Create` (`mc1.21.1/dev` @ `37ea4af`): Create `6.0.11` — 799 позиций (`616` 3D JSON, `129` 2D, `23` OBJ, `28` процедурных)
- `Creators-of-Aeronautics/Simulated-Project` (`main` @ `5cc7aa3`): `1.3.2` (`aeronautics`: 47, `simulated`: 104, `offroad`: 7)
- `george8188625/Create-Electro-Energetics` (`1.21.1` @ `4828c34`): `1.21.1-1.2.0` (129 позиций, `101` 3D JSON)
- `mrh0/createaddition` (`1.21.1` @ `50e21e3`): `1.7.3` (54 позиции, `22` 3D JSON)
- `Cannoneers-of-Create/CreateBigCannons` (`create-v6-1.21.1` @ `46a5d2a`): `5.11.7` (222 позиции, `173` 3D JSON — ассеты `CC BY-NC-SA 4.0`, исключены из репозитория)
- `george8188625/Create-Diesel-Generators` (`1.21.1`) и `Creators-of-Create/wiki` (`main` @ `419f585`).

Подробнее о лицензиях: [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) и [`docs/UPSTREAM-AND-LICENSES.md`](docs/UPSTREAM-AND-LICENSES.md).

---

## Формы распространения для Windows x64 (две чётко разделённые сборки)

Оба варианта используют единое нативное настольное приложение Tauri v2 (`factory-atlas.exe` со встроенным `dist/`), не требуют Node.js, терминала или локального HTTP-сервера.

> **Важно (статус компиляции на Windows):** в текущей Linux-песочнице подготовлены исходники, конфигурация `src-tauri/` и сценарий `.github/workflows/windows-installer.yml`, однако реальная компиляция и запуск `.exe` / `.zip` на Windows **не выполнялись** (в среде нет Rust/MSVC, NSIS и Windows).

### 1. Обычный установщик Windows x64 — основной и рекомендуемый вариант
- **Режим:** NSIS `installMode: "currentUser"` (`%LOCALAPPDATA%\Factory Atlas`), без обязательных прав администратора.
- **Ярлыки и система:** ярлык в меню **«Пуск»**, опция ярлыка на Рабочем столе в конце мастера, регистрация в «Приложения и возможности», ассоциация файлов `.factoryatlas`.
- **Изолированное хранение данных:** `%APPDATA%\dev.goshadows.factoryatlas\` (`autosave.json`, `projects/`, `assets-overrides/`), кэш WebView2 в `%LOCALAPPDATA%\dev.goshadows.factoryatlas\webview\`. Деинсталлятор по умолчанию **не удаляет** проекты пользователя.
- **WebView2:** встроен в Windows 11; на Windows 10 NSIS-мастер запускает `embedBootstrapper` (при отсутствии интернета можно предварительно установить Evergreen WebView2 Standalone x64).

### 2. Портативная версия (ZIP-архив) — дополнительный вариант по желанию пользователя
- **Запуск:** распаковка в любую папку, `factory-atlas.exe` рядом с `portable.flag` (или флагом `--portable`).
- **Полная изоляция:** все проекты, `autosave.json`, `assets-overrides/` и `webview/` (`data_directory`) хранятся строго внутри `./data/` рядом с `.exe`, не затрагивая установленную версию и `%APPDATA%`.

---

## Команды разработки, проверки и сборки Windows x64

### Проверка и разработка (без запуска Windows-сборки)
```bash
npm ci
npm run check      # svelte-check + tsc --noEmit (0 errors / 0 warnings)
npm test           # 31 тест в 5 наборах (Vitest)
npm run benchmark  # 500 узлов / 1000 рёбер
npm run build      # сборка dist/

# Режим разработки / браузерного предпросмотра (ТОЛЬКО для разработки, по умолчанию 127.0.0.1)
npm run dev
npm run serve:local
```

### Сборка Windows-установщика и портативного ZIP (выполнять на Windows 10/11 x64 с Rust MSVC + Node 22)
```powershell
npm ci
npm run check
npm test
npm run build

# 1) Основной NSIS-установщик
npx tauri build --target x86_64-pc-windows-msvc --bundles nsis

# 2) Портативный ZIP
New-Item -ItemType Directory -Force -Path "dist-portable\data\projects","dist-portable\data\assets-overrides","dist-portable\assets-overrides"
Copy-Item "src-tauri\target\x86_64-pc-windows-msvc\release\factory-atlas.exe" "dist-portable\factory-atlas.exe"
Copy-Item "LICENSE","THIRD_PARTY_NOTICES.md" "dist-portable\"
Copy-Item "assets-overrides\README.txt" "dist-portable\assets-overrides\README.txt"
"portable-mode" | Out-File -Encoding ascii "dist-portable\portable.flag"
Compress-Archive -Path "dist-portable\*" -DestinationPath "Factory-Atlas-0.1.0-win64-portable.zip" -Force

Get-FileHash -Algorithm SHA256 "src-tauri\target\x86_64-pc-windows-msvc\release\bundle\nsis\*.exe","Factory-Atlas-0.1.0-win64-portable.zip"
```

---

## Документация проекта

- [`AGENTS.md`](AGENTS.md) и [`CLAUDE.md`](CLAUDE.md) — правила разработки и инварианты проекта
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — архитектура, 3D-резолвер, расчётчик и изоляция desktop-режимов
- [`docs/RECIPE-DATA-FORMAT.md`](docs/RECIPE-DATA-FORMAT.md) — схема `.factoryatlas` v2 (включая `customResources`, `customMachines`, `.visual`)
- [`docs/ASSET-POLICY.md`](docs/ASSET-POLICY.md) — политика ассетов и локальной базы `assets-local/`
- [`docs/ASSET-REPLACEMENT.md`](docs/ASSET-REPLACEMENT.md) — ручная замена 2D PNG и 3D JSON моделей
- [`docs/UPSTREAM-AND-LICENSES.md`](docs/UPSTREAM-AND-LICENSES.md) — полный аудит апстрим-репозиториев и лицензий
- [`docs/ROADMAP.md`](docs/ROADMAP.md) — дальнейшее развитие
- [`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) — сторонние лицензии
