import { describe, expect, it } from 'vitest';
import {
  BUILTIN_MACHINES,
  BUILTIN_RECIPES,
  BUILTIN_RESOURCES,
  createStarterCogwheelProject,
  createStarterFluidAndChanceProject,
} from '../src/lib/data/testFixtures';
import { computeMachineThroughput } from '../src/lib/engine/machineSpeeds';
import { IncrementalSolverCache, solveFactoryProject } from '../src/lib/engine/solver';
import type { FactoryProject, MachineDef, RecipeDef } from '../src/lib/types/atlas';

const machinesById = new Map(BUILTIN_MACHINES.map((m) => [m.id, m]));
const recipesById = new Map(BUILTIN_RECIPES.map((r) => [r.id, r]));
const resourcesById = new Map(BUILTIN_RESOURCES.map((r) => [r.id, r]));

describe('machineSpeeds (Create 6.0.11 exact formulas & honesty)', () => {
  it('computes exact Millstone speed for 64 RPM on wheat milling (150 ticks)', () => {
    const mill = machinesById.get('create:millstone');
    const wheat = recipesById.get('create:milling/wheat');
    const res = computeMachineThroughput({
      machine: mill,
      recipe: wheat,
      rpm: 64,
      speedMode: 'formula',
    });
    // clamp(floor(64/16), 1, 512) = 4; ceil(150/4)+1 = 39 ticks; 1200/39 = 30.7692 crafts/min
    expect(res.source).toBe('exact-formula');
    expect(res.ticksPerCraft).toBe(39);
    expect(res.craftsPerMin).toBeCloseTo(1200 / 39, 4);
  });

  it('never invents speed for unknown machine formula and emits explicit unknown provenance', () => {
    const unknownMach: MachineDef = {
      id: 'custom:mystery_machine',
      nameRu: 'Неизвестная машина',
      nameEn: 'Mystery Machine',
      modId: 'custom',
      modVersion: '1.0',
      supportsRpm: true,
      maxRpm: 256,
      defaultRpm: 64,
      stressImpactSU: null,
      speedFormulaId: 'unknown',
      speedFormulaNoteRu: 'Формула не проверена',
      speedFormulaNoteEn: 'Formula unverified',
      sourceRef: 'manual',
    };
    const res = computeMachineThroughput({
      machine: unknownMach,
      recipe: undefined,
      rpm: 64,
      speedMode: 'formula',
    });
    expect(res.craftsPerMin).toBeNull();
    expect(res.source).toBe('unknown');
    expect(res.explanationRu.length).toBeGreaterThan(10);
  });

  it('marks Crushing Wheel / Encased Fan batch size as explicit assumption', () => {
    const fan = machinesById.get('create:encased_fan');
    const wash = recipesById.get('create:splashing/crushed_raw_iron');
    const res = computeMachineThroughput({
      machine: fan,
      recipe: wash,
      rpm: 64,
      speedMode: 'assumption',
      stackSizeAssumption: 16,
    });
    expect(res.source).toBe('assumption');
    // 16 items per 151 ticks = ~127.15 ops/min, ~9.4375 ticks per single item amortized
    expect(res.ticksPerCraft).toBeCloseTo(151 / 16, 4);
    expect(res.craftsPerMin).toBeCloseTo((1200 / 151) * 16, 4);
  });
});

describe('Incremental Graph Solver', () => {
  it('solves Cogwheel starter chain accurately with machine count & SU', () => {
    const proj = createStarterCogwheelProject();
    const result = solveFactoryProject(proj, { recipesById, machinesById, resourcesById });

    expect(result.warnings.filter((w) => w.severity === 'error')).toHaveLength(0);
    expect(result.stats.totalStressSU).toBeGreaterThan(0);

    const deployResult = result.nodeMap['n-deployer'];
    expect(deployResult).toBeDefined();
    expect(deployResult.requiredMachinesExact).toBeGreaterThan(0);
    expect(deployResult.requiredMachinesCeil).toBe(Math.ceil(deployResult.requiredMachinesExact!));
    expect(deployResult.outputs.some((o) => o.resourceId === 'create:cogwheel' && o.kind === 'item')).toBe(true);
  });

  it('rejects mixing items and fluids across incompatible ports', () => {
    const proj = createStarterCogwheelProject();
    proj.nodes.push({
      id: 'n-fluid-target',
      kind: 'target',
      title: 'Water target',
      x: 500,
      y: 500,
      resourceId: 'minecraft:water',
      resourceKind: 'fluid',
      targetRatePerMin: 1000,
    });
    proj.edges.push({
      id: 'e-bad-unit',
      fromNodeId: 'n-mixer',
      toNodeId: 'n-fluid-target',
      resourceId: 'create:andesite_alloy',
      resourceKind: 'item',
      rateMode: 'auto',
    });

    const result = solveFactoryProject(proj, { recipesById, machinesById, resourcesById });
    expect(result.warnings.some((w) => w.code === 'UNIT_MISMATCH' && w.severity === 'error')).toBe(true);
    expect(result.edgeMap['e-bad-unit'].isUnitMismatch).toBe(true);
  });

  it('computes fluid separation, probabilistic byproducts (expectedAverage) and deficits cleanly', () => {
    const proj = createStarterFluidAndChanceProject();
    const srcOre = proj.nodes.find((n) => n.id === 's-ore')!;
    srcOre.sourceRatePerMin = 5;

    const result = solveFactoryProject(proj, { recipesById, machinesById, resourcesById });
    const wash = result.nodeMap['op-wash'];
    expect(wash).toBeDefined();

    // Washing crushed iron has probabilistic redstone 75% byproduct
    const redstoneOut = wash.outputs.find((o) => o.resourceId === 'minecraft:redstone');
    expect(redstoneOut?.probabilistic).toBe(true);
    expect(redstoneOut?.producedGuaranteedPerMin).toBe(0);
    expect(redstoneOut?.producedExpectedPerMin).toBeGreaterThan(0);

    // Deficit warning emitted because 5 ore/min cannot fulfill 180 nuggets/min
    expect(result.warnings.some((w) => w.code === 'DEFICIT')).toBe(true);
    // Resource balances keep items and fluids strictly separated
    const fluidBalance = result.balances.find((b) => b.resourceId === 'createdieselgenerators:crude_oil');
    expect(fluidBalance?.kind).toBe('fluid');
    expect(fluidBalance?.unit).toBe('mB/min');
  });

  it('detects cycles via Tarjan SCC without hanging and warns clearly', () => {
    const proj: FactoryProject = {
      schemaVersion: 2,
      id: 'cycle-test',
      name: 'Cycle Test',
      createdAt: '2026-01-01T00:00:00Z',
      updatedAt: '2026-01-01T00:00:00Z',
      profile: createStarterCogwheelProject().profile,
      customResources: [],
      customMachines: [],
      customRecipes: [
        {
          id: 'test:loop_a',
          type: 'create:pressing',
          nameRu: 'Loop A',
          nameEn: 'Loop A',
          machineId: 'create:mechanical_press',
          inputs: [{ resourceId: 'create:iron_sheet', kind: 'item', amount: 1 }],
          outputs: [
            {
              resourceId: 'minecraft:iron_ingot',
              kind: 'item',
              amount: 1,
              chance: 1,
              probabilityKind: 'guaranteed',
            },
          ],
          processingTimeTicks: 100,
          heatRequirement: 'none',
          source: {
            origin: 'manual',
            modId: 'test',
            modVersion: '1.0',
            minecraftVersion: '1.21.1',
            loader: 'neoforge',
          },
        },
      ],
      nodes: [
        {
          id: 'n1',
          kind: 'recipe',
          title: 'Press',
          x: 0,
          y: 0,
          recipeId: 'create:pressing/iron_ingot',
          machineId: 'create:mechanical_press',
          rpm: 64,
          speedMode: 'formula',
        },
        {
          id: 'n2',
          kind: 'recipe',
          title: 'Reverse',
          x: 250,
          y: 0,
          recipeId: 'test:loop_a',
          machineId: 'create:mechanical_press',
          rpm: 64,
          speedMode: 'formula',
        },
      ],
      edges: [
        {
          id: 'e1',
          fromNodeId: 'n1',
          toNodeId: 'n2',
          resourceId: 'create:iron_sheet',
          resourceKind: 'item',
          rateMode: 'auto',
        },
        {
          id: 'e2',
          fromNodeId: 'n2',
          toNodeId: 'n1',
          resourceId: 'minecraft:iron_ingot',
          resourceKind: 'item',
          rateMode: 'auto',
        },
      ],
    };

    const customRecipesMap = new Map<string, RecipeDef>(recipesById);
    for (const r of proj.customRecipes) customRecipesMap.set(r.id, r);
    const customMachinesMap = new Map<string, MachineDef>(machinesById);

    const result = solveFactoryProject(proj, {
      recipesById: customRecipesMap,
      machinesById: customMachinesMap,
      resourcesById,
    });
    expect(result.cycles.length).toBeGreaterThan(0);
    expect(result.warnings.some((w) => w.code === 'CYCLE_DETECTED')).toBe(true);
  });

  it('uses incremental component caching on partial graph edits', () => {
    const base = createStarterCogwheelProject();
    // Add an independent second disconnected chain
    base.nodes.push(
      {
        id: 'n-wheat',
        kind: 'source',
        title: 'Wheat',
        x: 0,
        y: 600,
        resourceId: 'minecraft:wheat',
        resourceKind: 'item',
        sourceRatePerMin: 60,
      },
      {
        id: 'n-mill',
        kind: 'recipe',
        title: 'Mill Wheat',
        x: 280,
        y: 600,
        recipeId: 'create:milling/wheat',
        machineId: 'create:millstone',
        rpm: 64,
        speedMode: 'formula',
      }
    );
    base.edges.push({
      id: 'e-wheat',
      fromNodeId: 'n-wheat',
      toNodeId: 'n-mill',
      resourceId: 'minecraft:wheat',
      resourceKind: 'item',
      rateMode: 'auto',
    });

    const cache = new IncrementalSolverCache();
    const first = solveFactoryProject(base, { recipesById, machinesById, resourcesById }, cache);
    expect(first.stats.cachedComponents).toBe(0);
    expect(first.stats.componentCount).toBe(3); // Cogwheel graph + starter note + wheat graph

    // Edit ONLY the cogwheel chain, leaving wheat chain and note untouched
    const modified = structuredClone(base);
    modified.nodes.find((n) => n.id === 'n-target')!.targetRatePerMin = 45;

    const second = solveFactoryProject(modified, { recipesById, machinesById, resourcesById }, cache);
    expect(second.stats.cachedComponents).toBe(2);
    expect(second.stats.recomputedComponents).toBe(1);
    expect(second.nodeMap['n-mill']).toEqual(first.nodeMap['n-mill']);
  });
});
