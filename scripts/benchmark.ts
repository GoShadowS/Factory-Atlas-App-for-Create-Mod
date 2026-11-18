import { performance } from 'node:perf_hooks';
import { DEFAULT_PROFILE } from '../src/lib/data/createCatalog';
import {
  BUILTIN_MACHINES,
  BUILTIN_RECIPES,
  BUILTIN_RESOURCES,
} from '../src/lib/data/testFixtures';
import { IncrementalSolverCache, solveFactoryProject } from '../src/lib/engine/solver';
import { exportBalancesToCsv, exportProjectToSvg } from '../src/lib/io/exporters';
import { parseAndMigrateProject, serializeProject } from '../src/lib/io/projectFormat';
import type { FactoryEdge, FactoryNode, FactoryProject } from '../src/lib/types/atlas';

function buildSyntheticLargeGraph(nodeCount = 500, targetEdgeCount = 1000): FactoryProject {
  const nodes: FactoryNode[] = [];
  const edges: FactoryEdge[] = [];

  // Create 50 disconnected 10-node chains (= 500 nodes) with ~20 edges per chain (= 1000 edges)
  const chainsCount = Math.floor(nodeCount / 10);
  for (let c = 0; c < chainsCount; c++) {
    const baseIdx = c * 10;
    const baseY = c * 220;

    nodes.push({
      id: `n-${baseIdx}`,
      kind: 'source',
      title: `Iron Source #${c}`,
      x: 0,
      y: baseY,
      resourceId: 'minecraft:iron_ingot',
      resourceKind: 'item',
      sourceRatePerMin: 240,
    });

    for (let k = 1; k <= 8; k++) {
      const id = `n-${baseIdx + k}`;
      nodes.push({
        id,
        kind: 'recipe',
        title: `Stage ${c}-${k}`,
        x: k * 280,
        y: baseY + ((k % 2) * 90),
        recipeId: k % 3 === 0 ? 'create:pressing/brass_ingot' : 'create:pressing/iron_ingot',
        machineId: 'create:mechanical_press',
        rpm: 64,
        speedMode: 'formula',
      });
    }

    nodes.push({
      id: `n-${baseIdx + 9}`,
      kind: 'target',
      title: `Target #${c}`,
      x: 9 * 280,
      y: baseY,
      resourceId: 'create:iron_sheet',
      resourceKind: 'item',
      targetRatePerMin: 120,
    });

    for (let k = 0; k < 9; k++) {
      edges.push({
        id: `e-${c}-p-${k}`,
        fromNodeId: `n-${baseIdx + k}`,
        toNodeId: `n-${baseIdx + k + 1}`,
        resourceId: k === 0 ? 'minecraft:iron_ingot' : 'create:iron_sheet',
        resourceKind: 'item',
        rateMode: 'auto',
      });
    }
    for (let k = 0; k < 11 && edges.length < targetEdgeCount; k++) {
      const from = `n-${baseIdx + (k % 5)}`;
      const to = `n-${baseIdx + 5 + (k % 5)}`;
      edges.push({
        id: `e-${c}-b-${k}`,
        fromNodeId: from,
        toNodeId: to,
        resourceId: 'create:iron_sheet',
        resourceKind: 'item',
        rateMode: 'auto',
      });
    }
  }

  return {
    schemaVersion: 2,
    id: 'bench-500-1000',
    name: 'Benchmark 500x1000',
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    profile: structuredClone(DEFAULT_PROFILE),
    nodes,
    edges,
    customResources: [],
    customMachines: [],
    customRecipes: [],
  };
}

function main() {
  const machinesById = new Map(BUILTIN_MACHINES.map((m) => [m.id, m]));
  const recipesById = new Map(BUILTIN_RECIPES.map((r) => [r.id, r]));
  const resourcesById = new Map(BUILTIN_RESOURCES.map((r) => [r.id, r]));

  const graph = buildSyntheticLargeGraph(500, 1000);
  const cache = new IncrementalSolverCache();

  const t0 = performance.now();
  const cold = solveFactoryProject(graph, { recipesById, machinesById, resourcesById }, cache);
  const t1 = performance.now();

  // Localized edit in only 1 of the 50 components
  const edited = structuredClone(graph);
  edited.nodes[9].targetRatePerMin = 180;

  const t2 = performance.now();
  const warm = solveFactoryProject(edited, { recipesById, machinesById, resourcesById }, cache);
  const t3 = performance.now();

  const t4 = performance.now();
  const serialized = serializeProject(edited);
  const migrated = parseAndMigrateProject(serialized);
  const t5 = performance.now();

  const t6 = performance.now();
  const svg = exportProjectToSvg(edited, warm, resourcesById, 'ru');
  const csv = exportBalancesToCsv(edited, warm, resourcesById, 'ru');
  const t7 = performance.now();

  const mem = process.memoryUsage();
  const coldMs = +(t1 - t0).toFixed(2);
  const warmMs = +(t3 - t2).toFixed(2);
  const ioMs = +(t5 - t4).toFixed(2);
  const exportMs = +(t7 - t6).toFixed(2);

  console.log('=== Factory Atlas 500-node / 1000-edge Performance Benchmark ===');
  console.log(`Nodes: ${graph.nodes.length}, Edges: ${graph.edges.length}, Components: ${cold.stats.componentCount}`);
  console.log(`Cold full solve:         ${coldMs} ms (cached components: ${cold.stats.cachedComponents}/${cold.stats.componentCount})`);
  console.log(`Incremental edit solve:  ${warmMs} ms (cached components: ${warm.stats.cachedComponents}/${warm.stats.componentCount})`);
  console.log(`.factoryatlas roundtrip: ${ioMs} ms (ok=${migrated.ok}, bytes=${serialized.length})`);
  console.log(`SVG + CSV export:        ${exportMs} ms (svg=${svg.length} chars, csv=${csv.length} chars)`);
  console.log(`Memory RSS / HeapUsed:   ${(mem.rss / 1048576).toFixed(1)} MB / ${(mem.heapUsed / 1048576).toFixed(1)} MB`);

  if (coldMs > 500 || warmMs > 100 || warm.stats.cachedComponents < 49) {
    console.error('Benchmark regression threshold exceeded!');
    process.exit(1);
  }
}

main();
