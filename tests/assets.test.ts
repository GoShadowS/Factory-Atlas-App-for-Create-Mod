import fs from 'node:fs';
import path from 'node:path';
import { describe, expect, it } from 'vitest';
import {
  assetIdToOverrideRelPaths,
  buildAssetManifest,
  fallbackUrlForKind,
} from '../src/lib/assets/assetManager';
import type { MachineDef, ResourceDef, VisualIndexData } from '../src/lib/types/atlas';

describe('4-tier texture manifest & override resolution (without pre-populated catalog)', () => {
  it('resolves priority strictly: browser override > disk override > local original > SVG fallback', () => {
    const userResources: ResourceDef[] = [
      {
        id: 'create:cogwheel',
        kind: 'item',
        nameRu: 'Шестерня',
        nameEn: 'Cogwheel',
        modId: 'create',
        category: 'component',
        color: '#b45309',
      },
      {
        id: 'minecraft:iron_ingot',
        kind: 'item',
        nameRu: 'Железный слиток',
        nameEn: 'Iron Ingot',
        modId: 'minecraft',
        category: 'raw',
        color: '#cbd5e1',
      },
      {
        id: 'create:chocolate',
        kind: 'fluid',
        nameRu: 'Шоколад',
        nameEn: 'Chocolate',
        modId: 'create',
        category: 'fluid',
        color: '#78350f',
      },
    ];
    const userMachines: MachineDef[] = [
      {
        id: 'create:mechanical_mixer',
        nameRu: 'Механический смешиватель',
        nameEn: 'Mechanical Mixer',
        modId: 'create',
        modVersion: '6.0.11',
        stressImpactSU: 4,
        defaultRpm: 64,
        maxRpm: 256,
        supportsRpm: true,
        speedFormulaId: 'mechanical_mixer',
        speedFormulaNoteRu: '',
        speedFormulaNoteEn: '',
        sourceRef: 'user',
      },
    ];

    const manifest = buildAssetManifest({
      userResources,
      userMachines,
      localOriginals: [
        {
          id: 'create:cogwheel',
          modId: 'create',
          modVersion: '6.0.11',
          sourceRepo: 'Creators-of-Create/Create',
          sourcePath: 'src/main/resources/assets/create/textures/block/cogwheel.png',
          localOriginalPath: 'assets-local/upstream-originals/create/textures/block/cogwheel.png',
          license: 'All Rights Reserved',
          width: 16,
          height: 16,
          animatedStrip: false,
        },
        {
          id: 'create:chocolate',
          modId: 'create',
          modVersion: '6.0.11',
          sourceRepo: 'Creators-of-Create/Create',
          sourcePath: 'src/main/resources/assets/create/textures/fluid/chocolate_still.png',
          localOriginalPath: 'assets-local/upstream-originals/create/textures/fluid/chocolate_still.png',
          license: 'All Rights Reserved',
          width: 16,
          height: 512,
          animatedStrip: true,
        },
      ],
      diskOverrides: ['create__cogwheel.png', 'minecraft__iron_ingot.png'],
      browserOverrides: {
        'create:cogwheel': {
          dataUrl: 'data:image/png;base64,iVBORw0KGgo=',
          fileName: 'custom.png',
          updatedAt: '2026-01-01T00:00:00Z',
        },
      },
    });

    const byId = new Map(manifest.map((m) => [m.id, m]));

    expect(byId.get('create:cogwheel')?.status).toBe('user-override');
    expect(byId.get('create:cogwheel')?.url.startsWith('data:image/png')).toBe(true);

    expect(byId.get('minecraft:iron_ingot')?.status).toBe('user-override');
    expect(byId.get('minecraft:iron_ingot')?.url).toBe('/assets-overrides/minecraft__iron_ingot.png');

    expect(byId.get('create:chocolate')?.status).toBe('temporary-original');
    expect(byId.get('create:chocolate')?.animatedStrip).toBe(true);
    expect(byId.get('create:chocolate')?.frameHeight).toBe(16);

    expect(byId.get('create:mechanical_mixer')?.status).toBe('fallback');
    expect(byId.get('create:mechanical_mixer')?.url).toBe('/assets-fallback/machine-fallback.svg');
  });

  it('produces valid flat & nested override relative paths and safe fallback URLs', () => {
    expect(assetIdToOverrideRelPaths('create:brass_ingot')).toEqual([
      'create__brass_ingot.png',
      'create/brass_ingot.png',
    ]);
    expect(fallbackUrlForKind('machine')).toBe('/assets-fallback/machine-fallback.svg');
    expect(fallbackUrlForKind('fluid')).toBe('/assets-fallback/fluid-fallback.svg');
    expect(fallbackUrlForKind('item')).toBe('/assets-fallback/item-fallback.svg');
  });

  it('starts completely empty when userResources and localOriginals are empty (no hidden preloaded catalog)', () => {
    const manifest = buildAssetManifest({
      localOriginals: [],
      diskOverrides: [],
      browserOverrides: {},
    });
    expect(manifest).toHaveLength(0);
  });

  it('resolves multi-repo visual index cleanly when gitignored assets-local exists', () => {
    const idxPath = path.resolve(process.cwd(), 'assets-local/visual-index.json');
    if (!fs.existsSync(idxPath)) return; // Skipped cleanly in CI where gitignored assets-local is absent
    const parsed = JSON.parse(fs.readFileSync(idxPath, 'utf8')) as VisualIndexData;
    expect(parsed.entries.length).toBeGreaterThanOrEqual(1000);
    expect(parsed.sources.length).toBeGreaterThanOrEqual(5);

    const namespaces = new Set(parsed.entries.map((e) => e.namespace));
    expect(namespaces.has('create')).toBe(true);
    expect(namespaces.has('electroenergetics')).toBe(true);
    expect(namespaces.has('createaddition')).toBe(true);
    expect(namespaces.has('createbigcannons')).toBe(true);
    expect(namespaces.has('aeronautics') || namespaces.has('simulated')).toBe(true);
  });
});
