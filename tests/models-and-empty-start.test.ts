import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import { matchVisualResources } from '../src/lib/assets/visualMatcher';
import { createEmptyProject } from '../src/lib/data/createCatalog';
import { parseAndMigrateProject, serializeProject } from '../src/lib/io/projectFormat';
import {
  BoundedLruCache,
  McModelResolver,
  modelRefToLocalPath,
  resolveTextureVariable,
  selectBlockstateModels,
  type McJsonFetcher,
  type RawMcModelJson,
} from '../src/lib/models/mcModelResolver';
import type { VisualEntrySummary, VisualIndexData } from '../src/lib/types/atlas';

const SAMPLE_VISUAL_ENTRIES: VisualEntrySummary[] = [
  {
    id: 'create:cogwheel',
    namespace: 'create',
    pathId: 'cogwheel',
    kind: 'block',
    nameEn: 'Cogwheel',
    nameRu: 'Шестерня',
    normalizedKey: 'cogwheel',
    normalizedEn: 'cogwheel',
    normalizedRu: 'шестерня',
    sourceRepo: 'Creators-of-Create/Create',
    sourceBranch: 'mc1.21.1/dev',
    sourceCommit: '37ea4af',
    modVersion: '6.0.11',
    minecraftVersion: '1.21.1',
    license: 'All Rights Reserved',
    blockstatePath: 'create/blockstates/cogwheel.json',
    modelRef: 'create:item/cogwheel',
    modelPath: 'create/models/item/cogwheel.json',
    worldModelRef: 'create:block/cogwheel',
    worldModelPath: 'create/models/block/cogwheel.json',
    texturePath: 'create/textures/block/cogwheel.png',
    modelSupport: '3d-json-elements',
    hasModel3D: true,
    hasTexture2D: true,
  },
  {
    id: 'create:large_cogwheel',
    namespace: 'create',
    pathId: 'large_cogwheel',
    kind: 'block',
    nameEn: 'Large Cogwheel',
    nameRu: 'Большая шестерня',
    normalizedKey: 'largecogwheel',
    normalizedEn: 'largecogwheel',
    normalizedRu: 'большаяшестерня',
    sourceRepo: 'Creators-of-Create/Create',
    sourceBranch: 'mc1.21.1/dev',
    sourceCommit: '37ea4af',
    modVersion: '6.0.11',
    minecraftVersion: '1.21.1',
    license: 'All Rights Reserved',
    blockstatePath: 'create/blockstates/large_cogwheel.json',
    modelRef: 'create:item/large_cogwheel',
    modelPath: 'create/models/item/large_cogwheel.json',
    worldModelRef: 'create:block/large_cogwheel',
    worldModelPath: 'create/models/block/large_cogwheel.json',
    texturePath: 'create/textures/block/large_cogwheel.png',
    modelSupport: '3d-json-elements',
    hasModel3D: true,
    hasTexture2D: true,
  },
  {
    id: 'create:mechanical_press',
    namespace: 'create',
    pathId: 'mechanical_press',
    kind: 'block',
    nameEn: 'Mechanical Press',
    nameRu: 'Механический пресс',
    normalizedKey: 'mechanicalpress',
    normalizedEn: 'mechanicalpress',
    normalizedRu: 'механическийпресс',
    sourceRepo: 'Creators-of-Create/Create',
    sourceBranch: 'mc1.21.1/dev',
    sourceCommit: '37ea4af',
    modVersion: '6.0.11',
    minecraftVersion: '1.21.1',
    license: 'All Rights Reserved',
    blockstatePath: 'create/blockstates/mechanical_press.json',
    modelRef: 'create:block/mechanical_press/item',
    modelPath: 'create/models/block/mechanical_press/item.json',
    worldModelRef: 'create:block/mechanical_press/block',
    worldModelPath: 'create/models/block/mechanical_press/block.json',
    texturePath: 'create/textures/block/mechanical_press_side.png',
    modelSupport: '3d-json-elements',
    hasModel3D: true,
    hasTexture2D: true,
  },
  {
    id: 'create:brass_ingot',
    namespace: 'create',
    pathId: 'brass_ingot',
    kind: 'item',
    nameEn: 'Brass Ingot',
    nameRu: 'Латунный слиток',
    normalizedKey: 'brassingot',
    normalizedEn: 'brassingot',
    normalizedRu: 'латунныйслиток',
    sourceRepo: 'Creators-of-Create/Create',
    sourceBranch: 'mc1.21.1/dev',
    sourceCommit: '37ea4af',
    modVersion: '6.0.11',
    minecraftVersion: '1.21.1',
    license: 'All Rights Reserved',
    blockstatePath: null,
    modelRef: 'create:item/brass_ingot',
    modelPath: 'create/models/item/brass_ingot.json',
    worldModelRef: null,
    worldModelPath: null,
    texturePath: 'create/textures/item/brass_ingot.png',
    modelSupport: '2d-item-generated',
    hasModel3D: false,
    hasTexture2D: true,
  },
];

describe('Empty project start & persistence of custom resources/visuals', () => {
  it('initializes new projects completely empty with zero pre-filled nodes, recipes, resources, or machines', () => {
    const empty = createEmptyProject('Clean Start');
    expect(empty.schemaVersion).toBe(2);
    expect(empty.nodes).toEqual([]);
    expect(empty.edges).toEqual([]);
    expect(empty.customResources).toEqual([]);
    expect(empty.customMachines).toEqual([]);
    expect(empty.customRecipes).toEqual([]);
    expect(empty.profile.addons).toEqual([]);
  });

  it('round-trips customResources, customMachines, and visual overrides cleanly in .factoryatlas v2', () => {
    const proj = createEmptyProject('Custom Visual Test');
    proj.customResources.push({
      id: 'create:cogwheel',
      kind: 'item',
      nameRu: 'Шестерня',
      nameEn: 'Cogwheel',
      modId: 'create',
      category: 'component',
      color: '#f59e0b',
      userCreated: true,
      visual: {
        visualId: 'create:cogwheel',
        modelRef: 'create:item/cogwheel',
        modelPath: 'create/models/item/cogwheel.json',
        texturePath: 'create/textures/block/cogwheel.png',
        modelSupport: '3d-json-elements',
        matchReason: 'lang-alias',
      },
    });
    const raw = serializeProject(proj);
    const parsed = parseAndMigrateProject(raw);
    expect(parsed.ok).toBe(true);
    if (parsed.ok) {
      expect(parsed.project.customResources).toHaveLength(1);
      expect(parsed.project.customResources[0].visual?.modelRef).toBe('create:item/cogwheel');
      expect(parsed.project.customResources[0].visual?.modelSupport).toBe('3d-json-elements');
    }
  });
});

describe('Visual reference matcher priority & disambiguation', () => {
  it('matches exact ID -> exact path -> normalized -> lang -> ambiguous candidates', () => {
    // 1. Exact ID
    const m1 = matchVisualResources('create:cogwheel', SAMPLE_VISUAL_ENTRIES);
    expect(m1.status).toBe('unique');
    expect(m1.reason).toBe('exact-id');
    expect(m1.best?.id).toBe('create:cogwheel');

    // 2. Exact model/texture path
    const m2 = matchVisualResources('create/models/block/mechanical_press/item.json', SAMPLE_VISUAL_ENTRIES);
    expect(m2.status).toBe('unique');
    expect(m2.reason).toBe('exact-path');
    expect(m2.best?.id).toBe('create:mechanical_press');

    // 3. Normalized ID/filename
    const m3 = matchVisualResources('Brass Ingot', SAMPLE_VISUAL_ENTRIES);
    expect(m3.status).toBe('unique');
    expect(m3.reason).toBe('normalized');
    expect(m3.best?.id).toBe('create:brass_ingot');

    // 4. Exact localized Russian name
    const m4 = matchVisualResources('Механический пресс', SAMPLE_VISUAL_ENTRIES);
    expect(m4.status).toBe('unique');
    expect(m4.reason).toBe('lang-alias');
    expect(m4.best?.id).toBe('create:mechanical_press');

    // 5. Partial/ambiguous Russian query asks user to choose instead of guessing silently
    const m5 = matchVisualResources('шестерн', SAMPLE_VISUAL_ENTRIES);
    expect(m5.status).toBe('ambiguous');
    expect(m5.candidates.length).toBeGreaterThanOrEqual(2);
    expect(m5.best).toBeNull();

    // 6. Unknown query returns not-found
    const m6 = matchVisualResources('nonexistent_quantum_engine', SAMPLE_VISUAL_ENTRIES);
    expect(m6.status).toBe('not-found');
    expect(m6.best).toBeNull();
  });
});

describe('Minecraft JSON 3D model & blockstate resolver', () => {
  it('resolves parent inheritance, #texture variables, blockstates, cycles, and classification honestly', async () => {
    const models = new Map<string, RawMcModelJson>([
      [
        'testmod:block/parent_gear',
        {
          textures: { base: 'testmod:block/gear_tex', alias: '#base' },
          elements: [
            {
              from: [2, 2, 2],
              to: [14, 14, 14],
              rotation: { origin: [8, 8, 8], axis: 'y', angle: 22.5 },
              faces: {
                up: { texture: '#alias', uv: [0, 0, 12, 12] },
                north: { texture: '#base' },
              },
            },
          ],
        },
      ],
      ['testmod:block/child_gear', { parent: 'testmod:block/parent_gear' }],
      ['testmod:block/cube_example', { parent: 'minecraft:block/cube_all', textures: { all: 'testmod:block/steel' } }],
      ['testmod:item/flat_ingot', { parent: 'minecraft:item/generated', textures: { layer0: 'testmod:item/ingot' } }],
      ['testmod:block/obj_turret', { loader: 'neoforge:obj' }],
      ['testmod:block/fluid_proc', { textures: { particle: 'testmod:block/fluid_still' } }],
      ['testmod:block/cycle_a', { parent: 'testmod:block/cycle_b' }],
      ['testmod:block/cycle_b', { parent: 'testmod:block/cycle_a' }],
    ]);

    const blockstates = new Map<string, Record<string, unknown>>([
      [
        'testmod:child_gear',
        {
          variants: {
            'axis=x': { model: 'testmod:block/child_gear', x: 90, y: 90 },
            'axis=y': { model: 'testmod:block/child_gear', x: 0, y: 0 },
          },
        },
      ],
      [
        'testmod:pipe_multi',
        {
          multipart: [
            { apply: { model: 'testmod:block/cube_example' } },
            { when: { OR: [{ north: 'true' }, { south: 'true' }] }, apply: { model: 'testmod:block/child_gear', y: 180 } },
          ],
        },
      ],
    ]);

    const fetcher: McJsonFetcher = {
      fetchModelJson: async (ref) => models.get(ref) ?? null,
      fetchBlockstateJson: async (id) => blockstates.get(id) ?? null,
    };

    const resolver = new McModelResolver(fetcher);

    // 1. Inherited 3D JSON elements + #texture variable + element rotation + blockstate rotation
    const gearRes = await resolver.resolveBlockWithState('testmod:child_gear', { axis: 'x' });
    expect(gearRes.status).toBe('3d-json-elements');
    expect(gearRes.cuboids).toHaveLength(1);
    expect(gearRes.cuboids[0].blockRotationX).toBe(90);
    expect(gearRes.cuboids[0].blockRotationY).toBe(90);
    expect(gearRes.cuboids[0].elementRotation?.angle).toBe(22.5);
    expect(gearRes.cuboids[0].faces.up?.texturePath).toBe('testmod/textures/block/gear_tex.png');
    expect(gearRes.cuboids[0].faces.north?.uv).toEqual([2, 2, 14, 14]);

    // 2. Vanilla primitive parent cube_all -> 6 textured faces
    const cubeRes = await resolver.resolveModel('testmod:block/cube_example');
    expect(cubeRes.status).toBe('3d-json-elements');
    expect(Object.keys(cubeRes.cuboids[0].faces)).toHaveLength(6);
    expect(cubeRes.cuboids[0].faces.up?.texturePath).toBe('testmod/textures/block/steel.png');

    // 3. Multipart blockstate merges base + matching OR condition
    const mpParts = selectBlockstateModels(blockstates.get('testmod:pipe_multi')!, { north: 'true' });
    expect(mpParts).toHaveLength(2);

    // 4. Flat 2D item explicitly NOT reported as 3D
    const item2d = await resolver.resolveModel('testmod:item/flat_ingot');
    expect(item2d.status).toBe('2d-item-generated');
    expect(item2d.cuboids).toHaveLength(0);

    // 5. Wavefront OBJ explicitly reported as unsupported
    const objRes = await resolver.resolveModel('testmod:block/obj_turret');
    expect(objRes.status).toBe('unsupported-obj');

    // 6. Procedural / BlockEntity without elements explicitly reported
    const procRes = await resolver.resolveModel('testmod:block/fluid_proc');
    expect(procRes.status).toBe('procedural-or-blockentity');

    // 7. Cyclic parent & cyclic #texture never hang
    const cycRes = await resolver.resolveModel('testmod:block/cycle_a');
    expect(cycRes.status).toBe('none');
    expect(resolveTextureVariable('#a', { a: '#b', b: '#a' })).toBeNull();
  });

  it('evicts entries cleanly in BoundedLruCache and degrades gracefully when assets-local is absent', async () => {
    const lru = new BoundedLruCache<string, number>(2);
    lru.set('a', 1);
    lru.set('b', 2);
    expect(lru.get('a')).toBe(1); // refresh 'a'
    lru.set('c', 3); // evicts 'b'
    expect(lru.size).toBe(2);
    expect(lru.get('b')).toBeUndefined();
    expect(lru.get('a')).toBe(1);

    const emptyResolver = new McModelResolver({
      fetchModelJson: async () => null,
      fetchBlockstateJson: async () => null,
    });
    const missing = await emptyResolver.resolveBlockWithState('create:cogwheel', { axis: 'y' });
    expect(missing.status).toBe('none');
  });

  it('resolves real extracted upstream models from Create & addons when assets-local exists', async () => {
    const origBase = path.resolve(process.cwd(), 'assets-local/upstream-originals');
    const idxFile = path.resolve(process.cwd(), 'assets-local/visual-index.json');
    if (!fs.existsSync(origBase) || !fs.existsSync(idxFile)) return; // Skipped in CI

    const diskFetcher: McJsonFetcher = {
      async fetchModelJson(modelRef) {
        const rel = modelRefToLocalPath(modelRef);
        if (!rel) return null;
        const p = path.join(origBase, rel);
        if (!fs.existsSync(p)) return null;
        return JSON.parse(fs.readFileSync(p, 'utf8')) as RawMcModelJson;
      },
      async fetchBlockstateJson(blockId) {
        const [ns, sub] = blockId.includes(':') ? blockId.split(':', 2) : ['minecraft', blockId];
        const p = path.join(origBase, `${ns}/blockstates/${sub}.json`);
        if (!fs.existsSync(p)) return null;
        return JSON.parse(fs.readFileSync(p, 'utf8')) as Record<string, unknown>;
      },
    };

    const resolver = new McModelResolver(diskFetcher);
    const cog = await resolver.resolveBlockWithState('create:cogwheel', { axis: 'y' });
    expect(cog.status).toBe('3d-json-elements');
    expect(cog.cuboids.length).toBeGreaterThan(2);
    expect(cog.texturePaths.length).toBeGreaterThanOrEqual(1);

    const visualData = JSON.parse(fs.readFileSync(idxFile, 'utf8')) as VisualIndexData;
    const cogMatch = matchVisualResources('Шестерня', visualData.entries);
    expect(cogMatch.status).toBe('unique');
    expect(cogMatch.best?.id).toBe('create:cogwheel');
  });
});
