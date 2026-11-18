# Сторонние уведомления и лицензии (THIRD_PARTY_NOTICES)

Собственный исходный код, дизайн, векторная иконка приложения, нейтральные SVG-заглушки и документация **Factory Atlas** распространяются под лицензией **MIT** (см. файл [`LICENSE`](LICENSE)).

> **Важно:** оригинальные текстуры, модели, blockstates и языковые файлы сторонних модов Minecraft **не входят** в этот репозиторий, **не включаются** в публичный пакет, `dist/` и Windows-установщик. Они могут присутствовать исключительно локально в gitignored-папке `assets-local/` при проверке разработчиком или подменяться пользователем локально в `assets-overrides/`.

---

## 1. NPM-зависимости (MIT / Apache-2.0 / BSD / ISC)

- **Svelte 5** (`svelte`) — MIT License — UI-фреймворк
- **fflate** (`fflate`) — MIT License — быстрая синхронная и потоковая декомпрессия ZIP/GZIP в чистом JS/TS
- **Vite** (`vite`, `@sveltejs/vite-plugin-svelte`) — MIT License — сборщик фронтенда
- **TypeScript** (`typescript`, `svelte-check`) — Apache-2.0 / MIT — статическая типизация
- **Vitest** (`vitest`, `vite-node`) — MIT License — тестирование и запуск TypeScript-скриптов

---

## 2. Десктопный каркас (опционально при сборке Tauri v2)

- **Tauri v2** (`tauri`, `tauri-build`, `tauri-plugin-dialog`, `tauri-plugin-fs`) — Apache-2.0 OR MIT

---

## 3. Проверенные апстрим-источники модов и формул (аудит по GitHub)

Ниже перечислены фактически проверенные репозитории, ветки, коммиты и лицензии (ассеты в репозиторий **не скопированы**):

| Репозиторий | Ветка | Проверенный коммит | Версии | Лицензия кода | Лицензия ассетов / примечания |
| --- | --- | --- | --- | --- | --- |
| `Creators-of-Create/Create` | `mc1.21.1/dev` | `37ea4af` | Create `6.0.11`, MC `1.21.1`, NeoForge `21.1.219` | **MIT** | `./src/main/resources/assets/`: **All Rights Reserved** |
| `Creators-of-Aeronautics/Simulated-Project` (`aeronautics`, `simulated`, `offroad`) | `main` | `5cc7aa3` | `1.3.2`, MC `1.21.1`, Create `6.0.10` | **MIT** | `*/common/src/main/resources/assets/`: **All Rights Reserved** |
| `george8188625/Create-Electro-Energetics` | `1.21.1` | `4828c34` | `1.21.1-1.2.0`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** (весь репозиторий по `LICENSE`) |
| `mrh0/createaddition` | `1.21.1` | `50e21e3` | `1.7.3`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** (кроме звуков Pixabay/Freesound CC0) |
| `Cannoneers-of-Create/CreateBigCannons` | `create-v6-1.21.1` | `46a5d2a` | `5.11.7`, MC `1.21.1`, Create `6.0.10-280` | **MIT** (код и `lang/`) | `./src/main/resources/assets/createbigcannons`: **CC BY-NC-SA 4.0 International (NonCommercial)** — коммерческое перераспространение запрещено |
| `george8188625/Create-Diesel-Generators` | `1.21.1` | проверено по репо | `1.21.1-1.3.15`, MC `1.21.1`, Create `6.0.10-280` | **MIT** | **MIT** (текстуры исключены из Git для унификации) |
| `Creators-of-Create/wiki` | `main` | `419f585` | VitePress wiki | — | Справочная документация Create |

Полная таблица совместимости и лицензионных ограничений других аддонов приведена в [`docs/UPSTREAM-AND-LICENSES.md`](docs/UPSTREAM-AND-LICENSES.md).
