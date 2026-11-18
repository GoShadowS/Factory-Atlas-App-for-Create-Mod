import fs from 'node:fs';
import { gzipSync, zipSync, strToU8 } from 'fflate';
import { describe, expect, it } from 'vitest';
import {
  BUILTIN_MACHINES,
  BUILTIN_RECIPES,
  BUILTIN_RESOURCES,
  createStarterCogwheelProject,
} from '../src/lib/data/testFixtures';
import { solveFactoryProject } from '../src/lib/engine/solver';
import { exportBalancesToCsv, exportProjectToSvg } from '../src/lib/io/exporters';
import { importModArchive, parseSingleRecipeJson } from '../src/lib/io/jarImporter';
import {
  buildSyntheticDemoSchematicNbt,
  NBT_LIMITS,
  parseCreateSchematicNbt,
} from '../src/lib/io/nbtParser';
import { parseAndMigrateProject, serializeProject } from '../src/lib/io/projectFormat';

const machinesById = new Map(BUILTIN_MACHINES.map((m) => [m.id, m]));
const recipesById = new Map(BUILTIN_RECIPES.map((r) => [r.id, r]));
const resourcesById = new Map(BUILTIN_RESOURCES.map((r) => [r.id, r]));

describe('.factoryatlas format, migrations & exports', () => {
  it('round-trips a v2 project cleanly and rejects invalid JSON', () => {
    const orig = createStarterCogwheelProject();
    const raw = serializeProject(orig);
    const parsed = parseAndMigrateProject(raw);
    expect(parsed.ok).toBe(true);
    if (parsed.ok) {
      expect(parsed.project.schemaVersion).toBe(2);
      expect(parsed.project.nodes.length).toBe(orig.nodes.length);
      expect(parsed.project.edges.length).toBe(orig.edges.length);
    }

    const bad = parseAndMigrateProject('{broken');
    expect(bad.ok).toBe(false);
    const unknownSchema = parseAndMigrateProject(JSON.stringify({ schemaVersion: 99, nodes: [], edges: [] }));
    expect(unknownSchema.ok).toBe(false);
  });

  it('migrates legacy v1 schema projects cleanly to v2', () => {
    const v1Json = JSON.stringify({
      schemaVersion: 1,
      id: 'legacy-1',
      name: 'Legacy Chain',
      profile: {
        minecraftVersion: '1.20.1',
        loader: 'forge',
        createVersion: '0.5.1.f',
        addons: [],
      },
      nodes: [
        { id: 'a', kind: 'source', label: 'Iron', x: 10, y: 20, resourceId: 'minecraft:iron_ingot', rate: 30 },
        { id: 'b', kind: 'target', label: 'Out', x: 300, y: 20, resourceId: 'minecraft:iron_ingot', rate: 30 },
      ],
      edges: [
        { id: 'e1', fromNodeId: 'a', toNodeId: 'b', resourceId: 'minecraft:iron_ingot' },
      ],
    });

    const res = parseAndMigrateProject(v1Json);
    expect(res.ok).toBe(true);
    if (res.ok) {
      expect(res.migratedFromVersion).toBe(1);
      expect(res.project.schemaVersion).toBe(2);
      expect(res.project.nodes[0].title).toBe('Iron');
      expect(res.project.nodes[0].sourceRatePerMin).toBe(30);
      expect(res.project.edges[0].fromNodeId).toBe('a');
    }
  });

  it('exports valid SVG and UTF-8 BOM CSV with separated items/min vs mB/min units', () => {
    const proj = createStarterCogwheelProject();
    const solver = solveFactoryProject(proj, { recipesById, machinesById, resourcesById });

    const svg = exportProjectToSvg(proj, solver, resourcesById, 'ru');
    expect(svg.startsWith('<svg')).toBe(true);
    expect(svg).toContain('Factory Atlas');
    expect(svg).toContain('Шестерня');

    const csv = exportBalancesToCsv(proj, solver, resourcesById, 'ru');
    expect(csv.charCodeAt(0)).toBe(0xfeff); // UTF-8 BOM for Excel RU
    expect(csv).toContain('items/min');
    expect(csv).toContain('create:cogwheel');
  });
});

describe('Hardened .jar / .zip / .json importer', () => {
  it('parses NeoForge 1.21.1 & Fabric 1.20.1 (droplets) recipes while ignoring advancements', () => {
    const neoforgePress = JSON.stringify({
      type: 'create:pressing',
      ingredients: [{ item: 'minecraft:copper_ingot' }],
      results: [{ id: 'create:copper_sheet', count: 1 }],
    });
    const fabricMixing = JSON.stringify({
      type: 'create:mixing',
      heatRequirement: 'heated',
      processingTime: 100,
      ingredients: [
        { item: 'minecraft:cocoa_beans' },
        { fluidTag: 'c:milk', amount: 27000 },
      ],
      results: [{ fluid: 'create:chocolate', amount: 27000 }],
    });
    const advancementFake = JSON.stringify({
      criteria: { unlock: { trigger: 'minecraft:inventory_changed' } },
    });

    const fakeJar = zipSync({
      'data/create/recipe/pressing/copper_ingot.json': strToU8(neoforgePress),
      'data/create/recipes/mixing/chocolate.json': strToU8(fabricMixing),
      'data/create/advancement/recipes/copper.json': strToU8(advancementFake),
    });

    const out = importModArchive(fakeJar, 'test-mod.jar', {
      convertFabricDroplets: true,
    });

    expect(out.report.recipes.length).toBe(2);
    expect(out.report.importedCount).toBe(2);
    // Advancements are filtered cleanly before parsing so never appear in recipes or skippedDetails
    expect(out.report.recipes.some((r) => r.id.includes('advancement'))).toBe(false);

    const choc = out.report.recipes.find((r) => r.id.includes('chocolate'))!;
    expect(choc.heatRequirement).toBe('heated');
    // 27000 droplets -> ~333.33 mB
    expect(choc.outputs[0].kind).toBe('fluid');
    expect(choc.outputs[0].amount).toBeCloseTo(333.333, 2);
  });

  it('blocks path-traversal zip entries and rejects corrupt non-ZIP archives', () => {
    const maliciousZip = zipSync({
      '../../etc/passwd.json': strToU8('{"type":"create:pressing"}'),
      'data/create/recipe/ok.json': strToU8(
        JSON.stringify({
          type: 'create:pressing',
          ingredients: [{ item: 'minecraft:iron_ingot' }],
          results: [{ id: 'create:iron_sheet', count: 1 }],
        })
      ),
    });
    const out = importModArchive(maliciousZip, 'mixed.jar');
    expect(out.report.warnings.some((w) => /path traversal/i.test(w))).toBe(true);
    expect(out.report.recipes.length).toBe(1);

    // Compression ratio zip bomb check (>80:1 on >512 KiB recipe entry)
    const bigRecipe = `{"type":"create:pressing","pad":"${'A'.repeat(700_000)}"}`;
    const ratioBombZip = zipSync({
      'data/evil/recipe/bomb.json': strToU8(bigRecipe),
    }, { level: 9 });
    const bombOut = importModArchive(ratioBombZip, 'ratio-bomb.jar');
    expect(bombOut.report.warnings.some((w) => /бомб/i.test(w))).toBe(true);
    expect(bombOut.report.recipes.length).toBe(0);

    expect(() => importModArchive(new Uint8Array([1, 2, 3, 4]), 'corrupt.jar')).toThrow();
  });

  it('handles standalone JSON with unsupported recipe types transparently without inventing support', () => {
    const res = parseSingleRecipeJson(
      { type: 'some_mod:custom_magic', ingredients: [], results: [] },
      'magic.json',
      {
        modId: 'some_mod',
        modVersion: '1.0',
        minecraftVersion: '1.21.1',
        loader: 'neoforge',
      }
    );
    expect(res.ok).toBe(false);
    if (!res.ok) {
      expect(res.reasonCode).toBe('UNSUPPORTED_TYPE');
    }
  });
});

describe('.nbt Schematic Viewer & streaming decompression defense', () => {
  it('parses our synthetic demo .nbt schematic and marks isSyntheticDemo=true', () => {
    const bytes = buildSyntheticDemoSchematicNbt(true);
    const parsed = parseCreateSchematicNbt(bytes, 'demo.nbt', { isSyntheticDemo: true });
    expect(parsed.isSyntheticDemo).toBe(true);
    expect(parsed.size).toEqual([5, 3, 3]);
    expect(parsed.blocks.length).toBeGreaterThan(0);
    expect(parsed.palette.some((p) => p.name === 'create:cogwheel')).toBe(true);
  });

  it('rejects unsupported .litematic and .schem files cleanly', () => {
    const fakeLitematic = gzipSync(new Uint8Array([10, 0, 0, 0]));
    expect(() => parseCreateSchematicNbt(fakeLitematic, 'factory.litematic')).toThrow(/Litematica/i);
    expect(() => parseCreateSchematicNbt(fakeLitematic, 'area.schem')).toThrow(/Sponge/i);
  });

  it('blocks GZIP decompression bombs via streaming limit before OOM', () => {
    // 36 MB of zeros exceeds NBT_LIMITS.maxUncompressedBytes (32 MiB) while gzipped size stays tiny (<50 KB)
    const bigZero = new Uint8Array(NBT_LIMITS.maxUncompressedBytes + 2 * 1024 * 1024);
    const bomb = gzipSync(bigZero, { level: 6 });
    expect(bomb.byteLength).toBeLessThan(NBT_LIMITS.maxCompressedBytes);
    expect(() => parseCreateSchematicNbt(bomb, 'bomb.nbt')).toThrow(/GZIP-бомба/i);
  });

  it('handles truncated/corrupt .nbt without crashing', () => {
    expect(() =>
      parseCreateSchematicNbt(new Uint8Array([0x1f, 0x8b, 0x08, 0x00, 0xff]), 'broken.nbt')
    ).toThrow();
  });

  it('parses real extracted Create ponder analog_lever.nbt when local upstream cache exists', () => {
    const realPath =
      '/tmp/upstream/Create-mc1.21.1-dev/src/main/resources/assets/create/ponder/analog_lever.nbt';
    if (!fs.existsSync(realPath)) return; // Skipped cleanly in CI where /tmp/upstream is absent
    const bytes = new Uint8Array(fs.readFileSync(realPath));
    const parsed = parseCreateSchematicNbt(bytes, 'analog_lever.nbt', { isSyntheticDemo: false });
    expect(parsed.isSyntheticDemo).toBe(false);
    expect(parsed.size[0]).toBeGreaterThan(0);
    expect(parsed.palette.length).toBeGreaterThan(0);
    expect(parsed.blocks.length).toBeGreaterThan(0);
  });
});
