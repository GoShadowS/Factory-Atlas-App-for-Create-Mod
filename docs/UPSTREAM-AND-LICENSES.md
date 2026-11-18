# Аудит апстрим-репозиториев и лицензий (`UPSTREAM-AND-LICENSES`)

Ниже приведены **фактически проверенные** данные о версиях, коммитах, структуре ресурсов и лицензиях апстрим-репозиториев, полученные напрямую через GitHub API и файлы репозиториев (`gradle.properties`, `LICENSE*`, `fabric.mod.json`, `neoforge.mods.toml`, `assets/`).

---

## 1. Обязательный аудит 5 модовых репозиториев (+ доп. контекст)

| Репозиторий | Ветка | Проверенный коммит | Версии | Лицензия кода | Лицензия ассетов | Что извлечено в локальный `assets-local/` (исключено из Git) |
| --- | --- | --- | --- | --- | --- | --- |
| `Creators-of-Create/Create` | `mc1.21.1/dev` | `37ea4af` | Create `6.0.11`, MC `1.21.1`, NeoForge `21.1.219`, Flywheel `1.0.7` | **MIT** (`LICENSE.md`) | **All Rights Reserved** (`./src/main/resources/assets/`) | 799 позиций (`616` 3D JSON, `129` 2D, `23` OBJ, `28` процедурных, `740` RU `lang`) |
| `Creators-of-Aeronautics/Simulated-Project` (`aeronautics`, `simulated`, `offroad`) | `main` | `5cc7aa3` | `1.3.2`, MC `1.21.1`, Create `6.0.10` | **MIT** | **All Rights Reserved** (`*/common/src/main/resources/assets/`) | 158 позиций (`42+97+3` 3D JSON, `4+7+0` 2D, `0+0+4` OBJ, `1` проц., 100% RU `lang`) |
| `george8188625/Create-Electro-Energetics` | `1.21.1` | `4828c34` | `1.21.1-1.2.0`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** | 129 позиций (`101` 3D JSON, `26` 2D, `2` проц., `122` RU `lang`) |
| `mrh0/createaddition` (Crafts & Additions) | `1.21.1` | `50e21e3` | `1.7.3`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** (звуки CC0) | 54 позиции (`22` 3D JSON, `29` 2D, `1` OBJ, `2` проц., `50` RU `lang`) |
| `Cannoneers-of-Create/CreateBigCannons` | `create-v6-1.21.1` | `46a5d2a` | `5.11.7`, MC `1.21.1`, Create `6.0.10-280` | **MIT** (код и `lang/`) | **CC BY-NC-SA 4.0 International (NonCommercial)** (`src/main/resources/assets/createbigcannons`) | 222 позиции (`173` 3D JSON, `41` 2D, `1` OBJ, `7` проц., `203` RU `lang`) |
| `george8188625/Create-Diesel-Generators` | `1.21.1` | проверено | `1.21.1-1.3.15`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** | 109 позиций (`29` 3D JSON, `42` 2D, `38` проц., `101` RU `lang`) |
| `Creators-of-Create/wiki` | `main` | `419f585` | VitePress wiki | — | — | Справочная документация механик Create (`src/`) |

---

## 2. Ключевые технические находки по ресурсам, моделям и переводам

1. **Двухкорневая структура ресурсов (NeoForge datagen + main):**
   - `src/generated/resources/assets/<ns>/` хранит сгенерированные `blockstates/*.json`, авто-модели и `lang/en_us.json`.
   - `src/main/resources/assets/<ns>/` хранит ручные 3D-модели с `elements`, PNG-текстуры, `.mcmeta`, `neoforge:obj` (например, рулевые/приборные модели `create` и `offroad`), `.nbt`-схематики и перевод `lang/ru_ru.json`.
   - Для мультиплатформенных проектов (`Simulated-Project`) корни разнесены по подпроектам `aeronautics/`, `simulated/`, `offroad/` (`common` + `fabric`/`neoforge`).
2. **Витринная модель vs модель в мире:**
   - У ряда механизмов (например, `create:mechanical_press`, `create:mechanical_mixer`, `create:cogwheel`) `models/block/<id>/item.json` содержит полную статическую 3D-геометрию сборки (для UI/инвентаря), тогда как `models/block/<id>/block.json` описывает корпус блока для `blockstate`, а вращающиеся/подвижные части рендерятся Flywheel / BlockEntity.
3. **Разделение форматов в рендерере:**
   - Из **1 471** проиндексированной позиции **1 083** имеют статические 3D JSON `elements` (+ наследование `parent`, включая стандартные `minecraft:block/cube_all` и т.п.), **278** являются плоскими 2D-иконками (`item/generated`), **29** используют `neoforge:obj`, и **78** — процедурные/BlockEntity без статических элементов.
   - Приложение строго классифицирует каждый случай и **не** подменяет отсутствующую 3D-модель плоской картинкой.

---

## 3. Прочие ранее проверенные аддоны (справка по профилям и лицензиям)

| Аддон (репозиторий) | Ветка | Версии | Лицензия | Примечания |
| --- | --- | --- | --- | --- |
| `Fabricators-of-Create/Create` | `mc1.20.1/fabric/dev` | MC `1.20.1` | Код **MIT**, ассеты **ARR** | Жидкости в рецептах хранятся в `droplets` (`81000 = 1000 mB`) |
| `Layers-of-Railways/Railway` | `1.20/dev` | `1.7.3-beta`, MC `1.20.1` | **LGPL-3.0** | Для Create 0.5.1 |
| `copycats-plus/copycats` | `multiloader` | MC `1.21.1` / `1.20.1` | **All Rights Reserved** | Перераспространение запрещено |
| `DragonsPlusMinecraft/CreateEnchantmentIndustry` | `1.21.1/6.0.0-dev` | `2.5.4b`, MC `1.21.1` | **LGPL-3.0-or-later** | Импортируется пользователем из локального `.jar` |
| `DragonsPlusMinecraft/CreateCentralKitchen` | `1.21.1/6.0.0-dev` | `2.6.2b`, MC `1.21.1` | **LGPL-3.0-or-later** | Импортируется пользователем из локального `.jar` |
| `PssbleTrngle/SliceAndDice` | `main` | `0.0.0-dev`, MC `1.21.1` | Код **MIT**, ассеты **ARR** | Разделение лицензий кода и текстур |
| `hlysine/create_connected` | `main` | `1.3.3-mc1.21.1` | **AGPL-3.0** | Код AGPL не заимствован |
| `talrey/CreateDeco` | `1.21-neo` | `2.1.2`, MC `1.21.1` | Расхождение (`ARR` vs `CC0`) | Консервативно трактуется как ARR |
