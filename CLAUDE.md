# CLAUDE.md

Краткое руководство для ИИ-агентов (полный перечень правил см. в [`AGENTS.md`](AGENTS.md)):

- **Стек:** Svelte 5 + TypeScript + Vite 6 + `fflate` + Vitest 3 (`src/`), Tauri v2 (`src-tauri/`).
- **Язык интерфейса по умолчанию:** русский (`RU / EN`), темы: тёмная и светлая.
- **Ключевые инварианты (не нарушать):**
  1. Первый запуск и новый проект (`createEmptyProject()`) **всегда пусты** (`nodes: []`, `edges: []`, `customResources: []`, `customMachines: []`, `customRecipes: []`). Демо-данные допускаются только в `src/lib/data/testFixtures.ts` для тестов и бенчмарка.
  2. `assets-local/` (включая `visual-index.json` и `upstream-originals/`) и `user-projects/` **никогда не коммитятся в Git** и не входят в `dist/` или установщик; `npm run check`, `npm test`, `npm run build` обязаны проходить без `assets-local/`.
  3. Не выдавайте 2D PNG или процедурные/OBJ-модели за статические 3D JSON модели — классифицируйте честно (`3d-json-elements`, `2d-item-generated`, `unsupported-obj`, `procedural-or-blockentity`, `none`).
  4. Движок расчёта (`src/lib/engine/solver.ts`) строго разделяет `items/min` и `mB/min`, не выдумывает неизвестные скорости и выявляет циклы через Tarjan SCC.
  5. Не заявляйте проверку Windows `.exe` / `.zip`, если не собирали и не тестировали на реальной ОС Windows.
- **Проверки:** `npm run check && npm test && npm run benchmark && npm run build`.
