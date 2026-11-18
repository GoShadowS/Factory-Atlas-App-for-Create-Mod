# Формат данных проекта и рецептов (`Factory Atlas`)

## 1. Файл проекта `.factoryatlas` (схема `v2`)

Файл представляет собой UTF-8 JSON. Новые проекты стартуют полностью пустыми (`nodes: []`, `edges: []`, `customResources: []`, `customMachines: []`, `customRecipes: []`):

```json
{
  "schemaVersion": 2,
  "id": "proj-1763320000",
  "name": "Новый проект",
  "createdAt": "2026-11-16T21:00:00.000Z",
  "updatedAt": "2026-11-16T21:00:00.000Z",
  "profile": {
    "id": "custom-profile-1-21-1",
    "name": "Create 6.0.11 (MC 1.21.1 · NeoForge)",
    "minecraftVersion": "1.21.1",
    "loader": "neoforge",
    "createVersion": "6.0.11",
    "addons": []
  },
  "nodes": [],
  "edges": [],
  "customResources": [],
  "customMachines": [],
  "customRecipes": []
}
```

### Пользовательские ресурсы (`customResources[]`), машины (`customMachines[]`) и визуальные назначения (`.visual`)
Каждый созданный пользователем ресурс или машина хранит привязку к визуальному оформлению:
```json
"visual": {
  "visualId": "create:cogwheel",
  "modelRef": "create:item/cogwheel",
  "modelPath": "create/models/item/cogwheel.json",
  "texturePath": "create/textures/block/cogwheel.png",
  "modelSupport": "3d-json-elements",
  "customTextureOverrideDataUrl": "data:image/png;base64,...",
  "customModelJson": "{...}",
  "disabled": false,
  "matchReason": "lang-alias"
}
```

### Типы узлов (`nodes[].kind`)
- `source` — источник / склад (`resourceId`, `resourceKind`, `sourceRatePerMin` или `sourceUnlimited`).
- `recipe` — рецептурная операция (`recipeId`, `machineId`, `rpm`, `speedMode`, `manualCraftsPerMin`, `stackSizeAssumption`, `machineCountOverride`).
- `machine` — свободный машинный узел (`machineId`, `rpm`, `speedMode`).
- `byproduct` — приёмник побочного продукта (`resourceId`, `resourceKind`).
- `target` — производственная цель (`resourceId`, `resourceKind`, `targetRatePerMin`).
- `note` — текстовая заметка на холсте (`noteText`).

### Миграция со схемы `v1` -> `v2`
Парсер `parseAndMigrateProject` (`src/lib/io/projectFormat.ts`) автоматически мигрирует файлы `schemaVersion: 1` и ранние `v2`-файлы без `customResources`/`customMachines`:
- `node.label` -> `node.title`
- `node.rate` -> `node.sourceRatePerMin` / `node.targetRatePerMin`
- `node.unit` (`mb/min` vs `items/min`) -> `node.resourceKind` (`fluid` | `item`)
- `edge.from` / `edge.to` -> `edge.fromNodeId` / `edge.toNodeId`
- Автоматически заполняет отсутствующие `profile`, `customResources`, `customMachines`, `customRecipes`.

---

## 2. Внутренняя модель рецепта (`RecipeDef`)

- **Единицы количества (`inputs[]`, `outputs[]`):**
  - `kind: "item"` — количество в штуках за 1 цикл крафта.
  - `kind: "fluid"` — количество строго в милливедрах (`мБ` / `mB`).
- **Вероятности выходов:**
  - `probabilityKind: "guaranteed"` — гарантированный выход (`chance = 1`).
  - `probabilityKind: "bernoulli"` — независимый шанс `[0..1]`.
  - `probabilityKind: "weighted"` — взвешенный выбор из пула (`weight` из `create:sequenced_assembly`), при расчёте переводится в `chance = weight / totalWeight` и помечается как *ожидаемое среднее*.
- **Происхождение (`source`):** `'builtin' | 'imported-jar' | 'imported-json' | 'manual'`.

---

## 3. Поддерживаемые форматы импорта рецептов (`.jar` / `.zip` / `.json`)

Импорт запускается **только по явному действию и подтверждению пользователя**:

| Формат | Расположение | Единицы жидкостей | Примечания |
| --- | --- | --- | --- |
| NeoForge 1.21.1 | `data/<ns>/recipe/*.json` | `mB` (`neoforge:single`, `neoforge:tag`, `fluid_stack`, `fluid_tag`) | Поле длительности `processing_time`, результаты `results[].id` |
| Fabric / Forge 1.20.1 | `data/<ns>/recipes/*.json` | В Fabric — `droplets` (`81000 = 1000 mB`) | Поле `processingTime`, результаты `results[].item` |
| Защита | Игнорируются `/advancement/`, `/advancements/`, `/loot_table/` | — | Лимиты на размер/сжатие, защита от path traversal |
