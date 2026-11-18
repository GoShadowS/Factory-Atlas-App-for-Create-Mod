import type {
  EdgeCalculation,
  FactoryEdge,
  FactoryNode,
  FactoryProject,
  FlowUnit,
  MachineDef,
  NodeCalculation,
  NodeFlowLine,
  RecipeDef,
  ResourceBalanceRow,
  ResourceDef,
  ResourceKind,
  SolverResult,
  SolverWarning,
} from '../types/atlas';
import { computeMachineThroughput } from './machineSpeeds';

export interface SolverContext {
  recipesById: Map<string, RecipeDef>;
  machinesById: Map<string, MachineDef>;
  resourcesById: Map<string, ResourceDef>;
}

interface ComponentSolution {
  signature: string;
  nodeCalculations: NodeCalculation[];
  edgeCalculations: EdgeCalculation[];
  warnings: SolverWarning[];
  cycles: string[][];
}

export class IncrementalSolverCache {
  private components = new Map<string, ComponentSolution>();

  clear(): void {
    this.components.clear();
  }

  get(signature: string): ComponentSolution | undefined {
    return this.components.get(signature);
  }

  set(signature: string, sol: ComponentSolution): void {
    // Keep cache bounded so memory stays predictable on massive editing sessions
    if (this.components.size > 512) {
      const oldestKey = this.components.keys().next().value;
      if (oldestKey !== undefined) this.components.delete(oldestKey);
    }
    this.components.set(signature, sol);
  }
}

export function unitForKind(kind: ResourceKind): FlowUnit {
  return kind === 'fluid' ? 'mB/min' : 'items/min';
}

function round6(n: number): number {
  return Math.round(n * 1e6) / 1e6;
}

/**
 * Builds deterministic mathematical signature for a weakly connected component.
 * Intentionally excludes visual coordinates (x, y) and noteText so panning/moving
 * nodes never invalidates calculations.
 */
function buildComponentSignature(
  nodes: FactoryNode[],
  edges: FactoryEdge[],
  ctx: SolverContext
): string {
  const parts: string[] = [];
  const sortedNodes = [...nodes].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  for (const n of sortedNodes) {
    const r = n.recipeId ? ctx.recipesById.get(n.recipeId) : undefined;
    const m = n.machineId ? ctx.machinesById.get(n.machineId) : r ? ctx.machinesById.get(r.machineId) : undefined;
    parts.push(
      `N|${n.id}|${n.kind}|${n.resourceId ?? ''}|${n.resourceKind ?? ''}|${n.machineId ?? ''}|${n.recipeId ?? ''}|${n.sourceRatePerMin ?? ''}|${n.sourceUnlimited ? 1 : 0}|${n.targetRatePerMin ?? ''}|${n.rpm ?? ''}|${n.speedMode ?? ''}|${n.manualCraftsPerMin ?? ''}|${n.stackSizeAssumption ?? ''}|${n.machineCountOverride ?? ''}|R:${r ? r.inputs.map((i) => `${i.resourceId}:${i.amount}`).join(',') + '>' + r.outputs.map((o) => `${o.resourceId}:${o.amount}:${o.chance}`).join(',') + ':' + (r.processingTimeTicks ?? '') : ''}|M:${m ? `${m.speedFormulaId}:${m.stressImpactSU ?? ''}` : ''}`
    );
  }
  const sortedEdges = [...edges].sort((a, b) => (a.id < b.id ? -1 : a.id > b.id ? 1 : 0));
  for (const e of sortedEdges) {
    parts.push(
      `E|${e.id}|${e.fromNodeId}>${e.toNodeId}|${e.resourceId}|${e.resourceKind}|${e.rateMode}|${e.manualRatePerMin ?? ''}|${e.isByproductPort ? 1 : 0}`
    );
  }
  return parts.join(';');
}

function findCyclesTarjan(nodeIds: string[], adj: Map<string, string[]>): string[][] {
  let index = 0;
  const indices = new Map<string, number>();
  const lowlink = new Map<string, number>();
  const onStack = new Set<string>();
  const stack: string[] = [];
  const cycles: string[][] = [];

  function strongConnect(v: string) {
    indices.set(v, index);
    lowlink.set(v, index);
    index++;
    stack.push(v);
    onStack.add(v);

    for (const w of adj.get(v) ?? []) {
      if (!indices.has(w)) {
        strongConnect(w);
        lowlink.set(v, Math.min(lowlink.get(v)!, lowlink.get(w)!));
      } else if (onStack.has(w)) {
        lowlink.set(v, Math.min(lowlink.get(v)!, indices.get(w)!));
      }
    }

    if (lowlink.get(v) === indices.get(v)) {
      const scc: string[] = [];
      while (true) {
        const w = stack.pop()!;
        onStack.delete(w);
        scc.push(w);
        if (w === v) break;
      }
      if (scc.length > 1) {
        cycles.push(scc.reverse());
      } else if (scc.length === 1) {
        const only = scc[0];
        if ((adj.get(only) ?? []).includes(only)) {
          cycles.push([only]);
        }
      }
    }
  }

  for (const id of nodeIds) {
    if (!indices.has(id)) {
      strongConnect(id);
    }
  }
  return cycles;
}

function solveComponent(
  componentId: number,
  nodes: FactoryNode[],
  edges: FactoryEdge[],
  ctx: SolverContext
): Omit<ComponentSolution, 'signature'> {
  const nodeById = new Map<string, FactoryNode>(nodes.map((n) => [n.id, n]));
  const inEdges = new Map<string, FactoryEdge[]>();
  const outEdges = new Map<string, FactoryEdge[]>();
  const dirAdj = new Map<string, string[]>();

  for (const n of nodes) {
    inEdges.set(n.id, []);
    outEdges.set(n.id, []);
    dirAdj.set(n.id, []);
  }
  for (const e of edges) {
    if (!nodeById.has(e.fromNodeId) || !nodeById.has(e.toNodeId)) continue;
    outEdges.get(e.fromNodeId)!.push(e);
    inEdges.get(e.toNodeId)!.push(e);
    dirAdj.get(e.fromNodeId)!.push(e.toNodeId);
  }

  const cycles = findCyclesTarjan(
    nodes.map((n) => n.id),
    dirAdj
  );
  const cyclicNodeIds = new Set<string>();
  for (const c of cycles) {
    for (const id of c) cyclicNodeIds.add(id);
  }

  const warnings: SolverWarning[] = [];
  let warnCounter = 0;
  const pushWarn = (w: Omit<SolverWarning, 'id'>): string => {
    const id = `w-${componentId}-${++warnCounter}`;
    warnings.push({ id, ...w });
    return id;
  };

  for (const c of cycles) {
    const titles = c.map((id) => nodeById.get(id)?.title || id).join(' -> ');
    pushWarn({
      severity: 'error',
      code: 'CYCLE_DETECTED',
      nodeId: c[0],
      messageRu: `Обнаружена циклическая цепочка (${titles}). Автоматический расчёт внутри цикла остановлен, чтобы избежать бесконечного зацикливания.`,
      messageEn: `Cycle detected (${titles}). Automatic flow calculation inside the cycle was stopped to avoid infinite recursion.`,
      hintRu: 'Разорвите цикл или задайте внешнюю точку подпитки / ручную скорость на контуре.',
      hintEn: 'Break the feedback loop or add a dedicated external seed source / manual rate.',
    });
  }

  // Kahn topological ordering over acyclic nodes
  const acyclicIds = nodes.filter((n) => !cyclicNodeIds.has(n.id)).map((n) => n.id);
  const inDeg = new Map<string, number>();
  for (const id of acyclicIds) inDeg.set(id, 0);
  for (const e of edges) {
    if (!cyclicNodeIds.has(e.fromNodeId) && !cyclicNodeIds.has(e.toNodeId)) {
      inDeg.set(e.toNodeId, (inDeg.get(e.toNodeId) ?? 0) + 1);
    }
  }
  const queue: string[] = [];
  for (const [id, deg] of inDeg) {
    if (deg === 0) queue.push(id);
  }
  const topoOrder: string[] = [];
  while (queue.length > 0) {
    const u = queue.shift()!;
    topoOrder.push(u);
    for (const e of outEdges.get(u) ?? []) {
      if (cyclicNodeIds.has(e.toNodeId)) continue;
      const nd = (inDeg.get(e.toNodeId) ?? 1) - 1;
      inDeg.set(e.toNodeId, nd);
      if (nd === 0) queue.push(e.toNodeId);
    }
  }

  // Validate edge resource & unit compatibility
  const edgeUnitMismatch = new Set<string>();
  const edgeResourceMismatch = new Set<string>();

  for (const e of edges) {
    const fromNode = nodeById.get(e.fromNodeId);
    const toNode = nodeById.get(e.toNodeId);
    if (!fromNode || !toNode) continue;

    // Check expected resource kind on source/target/recipe ports
    const expectedFromKind = getNodePrimaryResourceKind(fromNode, e.resourceId, 'output', ctx);
    const expectedToKind = getNodePrimaryResourceKind(toNode, e.resourceId, 'input', ctx);

    if (
      (expectedFromKind && expectedFromKind !== e.resourceKind) ||
      (expectedToKind && expectedToKind !== e.resourceKind)
    ) {
      edgeUnitMismatch.add(e.id);
      pushWarn({
        severity: 'error',
        code: 'UNIT_MISMATCH',
        edgeId: e.id,
        nodeId: toNode.id,
        resourceId: e.resourceId,
        messageRu: `Несовместимые единицы на линии «${fromNode.title} -> ${toNode.title}»: нельзя смешивать предметы (предм./мин) и жидкости (мБ/мин).`,
        messageEn: `Unit mismatch on "${fromNode.title} -> ${toNode.title}": cannot mix items (items/min) and fluids (mB/min).`,
      });
    } else {
      // Check resource ID match against source/target/recipe
      const fromProduces = nodeProducesResource(fromNode, e.resourceId, ctx);
      const toConsumes = nodeConsumesResource(toNode, e.resourceId, ctx);
      if (!fromProduces || !toConsumes) {
        edgeResourceMismatch.add(e.id);
        pushWarn({
          severity: 'warning',
          code: 'RESOURCE_MISMATCH',
          edgeId: e.id,
          nodeId: !toConsumes ? toNode.id : fromNode.id,
          resourceId: e.resourceId,
          messageRu: `Несоответствие ресурса ${e.resourceId} на соединении «${fromNode.title} -> ${toNode.title}».`,
          messageEn: `Resource mismatch for ${e.resourceId} on "${fromNode.title} -> ${toNode.title}".`,
        });
      }
    }
  }

  // Track edge demand (backward pass) and supply (forward pass)
  const edgeDemand = new Map<string, number>();
  const edgeSupply = new Map<string, number>();
  const nodeRequestedCrafts = new Map<string, number>();
  const nodeEffectiveCrafts = new Map<string, number>();
  const nodeWarnings = new Map<string, string[]>();
  const addNodeWarn = (nodeId: string, wid: string) => {
    const arr = nodeWarnings.get(nodeId) ?? [];
    arr.push(wid);
    nodeWarnings.set(nodeId, arr);
  };

  // Helper: recipe output expected yield per craft for resourceId
  function recipeExpectedYield(recipe: RecipeDef, resourceId: string): number {
    let total = 0;
    for (const out of recipe.outputs) {
      if (out.resourceId === resourceId) {
        total += out.amount * out.chance;
      }
    }
    return total;
  }

  function recipeGuaranteedYield(recipe: RecipeDef, resourceId: string): number {
    let total = 0;
    for (const out of recipe.outputs) {
      if (out.resourceId === resourceId && out.probabilityKind === 'guaranteed' && out.chance >= 1) {
        total += out.amount;
      }
    }
    return total;
  }

  // Precompute throughput per machine for each node
  const nodeSpeedInfo = new Map<string, ReturnType<typeof computeMachineThroughput>>();
  for (const n of nodes) {
    if (n.kind === 'recipe' || n.kind === 'machine') {
      const recipe = n.recipeId ? ctx.recipesById.get(n.recipeId) : undefined;
      const machineId = n.machineId ?? recipe?.machineId;
      const machine = machineId ? ctx.machinesById.get(machineId) : undefined;
      const spd = computeMachineThroughput({
        machine,
        recipe,
        rpm: n.rpm,
        speedMode: n.speedMode,
        manualCraftsPerMin: n.manualCraftsPerMin,
        stackSizeAssumption: n.stackSizeAssumption,
      });
      nodeSpeedInfo.set(n.id, spd);
      if (n.kind === 'recipe' && !recipe) {
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'warning',
            code: 'MISSING_RECIPE',
            nodeId: n.id,
            messageRu: `В узле «${n.title}» не выбран рецепт — расчёт входов и выходов невозможен.`,
            messageEn: `Node "${n.title}" has no recipe selected.`,
          })
        );
      } else if (spd.source === 'unknown') {
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'warning',
            code: 'UNKNOWN_SPEED',
            nodeId: n.id,
            messageRu: `«${n.title}»: ${spd.explanationRu}`,
            messageEn: `"${n.title}": ${spd.explanationEn}`,
          })
        );
      } else if (spd.source === 'assumption') {
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'info',
            code: 'ASSUMPTION_ACTIVE',
            nodeId: n.id,
            messageRu: `«${n.title}»: ${spd.explanationRu}`,
            messageEn: `"${n.title}": ${spd.explanationEn}`,
          })
        );
      }

      if (recipe && recipe.outputs.some((o) => o.probabilityKind !== 'guaranteed')) {
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'info',
            code: 'PROBABILISTIC_OUTPUT',
            nodeId: n.id,
            messageRu: `«${n.title}» использует вероятностный рецепт — выпуск показан как математическое ожидание (среднее), а не гарантия каждой операции.`,
            messageEn: `"${n.title}" uses probabilistic outputs — rates reflect expected average yield, not a guaranteed drop per single craft.`,
          })
        );
      }
    }
  }

  // Step 1: Backward pass (Demand propagation in reverse topological order)
  for (let i = topoOrder.length - 1; i >= 0; i--) {
    const id = topoOrder[i];
    const node = nodeById.get(id)!;
    const outs = outEdges.get(id) ?? [];
    const ins = inEdges.get(id) ?? [];

    if (node.kind === 'target') {
      const targetRate = Math.max(0, node.targetRatePerMin ?? 0);
      if (ins.length === 0 && targetRate > 0) {
        addNodeWarn(
          id,
          pushWarn({
            severity: 'warning',
            code: 'DISCONNECTED_TARGET',
            nodeId: id,
            resourceId: node.resourceId,
            messageRu: `Цель «${node.title}» требует ${round6(targetRate)} ${unitForKind(node.resourceKind ?? 'item')}, но не имеет входящих подключений.`,
            messageEn: `Target "${node.title}" requests ${round6(targetRate)} ${unitForKind(node.resourceKind ?? 'item')} but has no incoming wires.`,
          })
        );
      }
      // Distribute target demand evenly across valid incoming edges
      const matchingIns = ins.filter(
        (e) => !edgeUnitMismatch.has(e.id) && (!node.resourceId || e.resourceId === node.resourceId)
      );
      const divisor = matchingIns.length || 1;
      for (const e of ins) {
        if (e.rateMode === 'manual' && e.manualRatePerMin !== undefined) {
          edgeDemand.set(e.id, Math.max(0, e.manualRatePerMin));
        } else if (matchingIns.includes(e)) {
          edgeDemand.set(e.id, targetRate / divisor);
        } else {
          edgeDemand.set(e.id, 0);
        }
      }
      continue;
    }

    if (node.kind === 'byproduct' || node.kind === 'note' || node.kind === 'source') {
      // Sources don't demand upstream inputs; byproducts absorb excess
      continue;
    }

    // Recipe or Machine node
    const recipe = node.recipeId ? ctx.recipesById.get(node.recipeId) : undefined;
    let requiredCrafts = 0;

    if (recipe) {
      // For each resource produced, check downstream edgeDemand
      const demandByRes = new Map<string, number>();
      for (const e of outs) {
        if (edgeUnitMismatch.has(e.id) || edgeResourceMismatch.has(e.id)) continue;
        const d =
          e.rateMode === 'manual' && e.manualRatePerMin !== undefined
            ? Math.max(0, e.manualRatePerMin)
            : (edgeDemand.get(e.id) ?? 0);
        demandByRes.set(e.resourceId, (demandByRes.get(e.resourceId) ?? 0) + d);
      }
      for (const [resId, dem] of demandByRes) {
        const y = recipeExpectedYield(recipe, resId);
        if (y > 0 && dem > 0) {
          requiredCrafts = Math.max(requiredCrafts, dem / y);
        }
      }
    } else if (node.kind === 'machine') {
      // Generic pass-through machine without recipe: use manual/downstream demand
      const totalOutDemand = outs.reduce((acc, e) => acc + (edgeDemand.get(e.id) ?? 0), 0);
      requiredCrafts = totalOutDemand;
    }

    nodeRequestedCrafts.set(id, requiredCrafts);

    // Propagate demand to incoming edges
    if (recipe) {
      for (const ing of recipe.inputs) {
        if (ing.amount <= 0) continue;
        const matchingIns = ins.filter((e) => e.resourceId === ing.resourceId && !edgeUnitMismatch.has(e.id));
        const totalReq = requiredCrafts * ing.amount;
        if (matchingIns.length > 0) {
          const share = totalReq / matchingIns.length;
          for (const e of matchingIns) {
            const val =
              e.rateMode === 'manual' && e.manualRatePerMin !== undefined
                ? Math.max(0, e.manualRatePerMin)
                : share;
            edgeDemand.set(e.id, (edgeDemand.get(e.id) ?? 0) + val);
          }
        }
      }
    }
  }

  // Step 2: Forward pass (Supply & Bottleneck propagation in topological order)
  for (const id of topoOrder) {
    const node = nodeById.get(id)!;
    const ins = inEdges.get(id) ?? [];
    const outs = outEdges.get(id) ?? [];

    if (node.kind === 'source') {
      const cap = node.sourceUnlimited ? Number.POSITIVE_INFINITY : Math.max(0, node.sourceRatePerMin ?? 0);
      const validOuts = outs.filter((e) => !edgeUnitMismatch.has(e.id) && !edgeResourceMismatch.has(e.id));
      const totalOutDemand = validOuts.reduce((acc, e) => acc + (edgeDemand.get(e.id) ?? 0), 0);

      if (validOuts.length === 0) continue;

      if (!Number.isFinite(cap)) {
        for (const e of validOuts) {
          const d = e.rateMode === 'manual' && e.manualRatePerMin !== undefined ? e.manualRatePerMin : (edgeDemand.get(e.id) ?? 0);
          edgeSupply.set(e.id, d);
        }
      } else if (totalOutDemand <= 0) {
        // No downstream demand specified: push source capacity evenly so forward-only chains still show rates
        const share = cap / validOuts.length;
        for (const e of validOuts) edgeSupply.set(e.id, share);
      } else if (cap >= totalOutDemand) {
        for (const e of validOuts) {
          edgeSupply.set(e.id, edgeDemand.get(e.id) ?? 0);
        }
      } else {
        // Source constrained: proportional allocation -> bottleneck!
        const ratio = cap / totalOutDemand;
        for (const e of validOuts) {
          edgeSupply.set(e.id, (edgeDemand.get(e.id) ?? 0) * ratio);
        }
      }
      continue;
    }

    if (node.kind === 'target' || node.kind === 'byproduct' || node.kind === 'note') {
      continue;
    }

    const recipe = node.recipeId ? ctx.recipesById.get(node.recipeId) : undefined;
    const spd = nodeSpeedInfo.get(id);
    const requestedCrafts = nodeRequestedCrafts.get(id) ?? 0;

    // Determine incoming supply per resource
    const suppliedByRes = new Map<string, number>();
    for (const e of ins) {
      if (edgeUnitMismatch.has(e.id) || edgeResourceMismatch.has(e.id)) continue;
      const sup =
        e.rateMode === 'manual' && e.manualRatePerMin !== undefined
          ? Math.min(e.manualRatePerMin, edgeSupply.get(e.id) ?? e.manualRatePerMin)
          : (edgeSupply.get(e.id) ?? 0);
      suppliedByRes.set(e.resourceId, (suppliedByRes.get(e.resourceId) ?? 0) + sup);
    }

    let maxCraftsByInputs = Number.POSITIVE_INFINITY;
    if (recipe && recipe.inputs.some((i) => i.amount > 0)) {
      for (const ing of recipe.inputs) {
        if (ing.amount <= 0) continue;
        const sup = suppliedByRes.get(ing.resourceId) ?? 0;
        const hasWiredIng = ins.some((e) => e.resourceId === ing.resourceId && !edgeUnitMismatch.has(e.id));
        if (hasWiredIng) {
          maxCraftsByInputs = Math.min(maxCraftsByInputs, sup / ing.amount);
        } else if (requestedCrafts > 0) {
          // Unwired input means 0 supply for that ingredient -> deficit!
          maxCraftsByInputs = 0;
        }
      }
    }

    // Machine count cap if user explicitly set machineCountOverride
    let maxCraftsByMachines = Number.POSITIVE_INFINITY;
    if (
      node.machineCountOverride !== undefined &&
      node.machineCountOverride > 0 &&
      spd?.craftsPerMin !== null &&
      spd?.craftsPerMin !== undefined
    ) {
      maxCraftsByMachines = node.machineCountOverride * spd.craftsPerMin;
    }

    let effectiveCrafts = 0;
    if (requestedCrafts > 0) {
      effectiveCrafts = Math.min(requestedCrafts, maxCraftsByInputs, maxCraftsByMachines);
    } else if (Number.isFinite(maxCraftsByInputs) && maxCraftsByInputs > 0) {
      // Forward-driven chain (source connected, no target at end yet)
      effectiveCrafts = Math.min(maxCraftsByInputs, maxCraftsByMachines);
    } else if (Number.isFinite(maxCraftsByMachines) && maxCraftsByMachines < Number.POSITIVE_INFINITY) {
      effectiveCrafts = maxCraftsByMachines;
    }

    nodeEffectiveCrafts.set(id, effectiveCrafts);

    // Push supply to outgoing edges
    if (recipe) {
      for (const resOut of new Set(recipe.outputs.map((o) => o.resourceId))) {
        const expectedTotal = effectiveCrafts * recipeExpectedYield(recipe, resOut);
        const matchingOuts = outs.filter((e) => e.resourceId === resOut && !edgeUnitMismatch.has(e.id));
        if (matchingOuts.length === 0) continue;
        const totalDem = matchingOuts.reduce((acc, e) => acc + (edgeDemand.get(e.id) ?? 0), 0);
        if (totalDem > 0) {
          for (const e of matchingOuts) {
            const share = ((edgeDemand.get(e.id) ?? 0) / totalDem) * expectedTotal;
            edgeSupply.set(e.id, share);
          }
        } else {
          const share = expectedTotal / matchingOuts.length;
          for (const e of matchingOuts) edgeSupply.set(e.id, share);
        }
      }
    }
  }

  // Step 3: Assemble NodeCalculations, EdgeCalculations, and Deficit / Bottleneck warnings
  const nodeCalculations: NodeCalculation[] = [];
  const edgeCalculations: EdgeCalculation[] = [];

  for (const e of edges) {
    if (!nodeById.has(e.fromNodeId) || !nodeById.has(e.toNodeId)) continue;
    const inCycle = cyclicNodeIds.has(e.fromNodeId) && cyclicNodeIds.has(e.toNodeId);
    const dem = round6(edgeDemand.get(e.id) ?? 0);
    const sup = round6(edgeSupply.get(e.id) ?? 0);
    const deficit = dem > sup + 1e-6 ? round6(dem - sup) : 0;
    const isBottleneck = deficit > 1e-5;
    const flow = dem > 0 ? Math.min(dem, sup) : sup;

    if (isBottleneck && !inCycle) {
      const fromTitle = nodeById.get(e.fromNodeId)?.title ?? e.fromNodeId;
      const toTitle = nodeById.get(e.toNodeId)?.title ?? e.toNodeId;
      const u = unitForKind(e.resourceKind);
      pushWarn({
        severity: 'warning',
        code: 'BOTTLENECK',
        edgeId: e.id,
        nodeId: e.toNodeId,
        resourceId: e.resourceId,
        messageRu: `Узкое место (${e.resourceId}): линия «${fromTitle} -> ${toTitle}» отдаёт ${sup.toFixed(2)} ${u}, но требуется ${dem.toFixed(2)} ${u} (дефицит ${deficit.toFixed(2)}).`,
        messageEn: `Bottleneck (${e.resourceId}): "${fromTitle} -> ${toTitle}" supplies ${sup.toFixed(2)} ${u}, but ${dem.toFixed(2)} ${u} required (deficit ${deficit.toFixed(2)}).`,
      });
    }

    edgeCalculations.push({
      edgeId: e.id,
      resourceId: e.resourceId,
      resourceKind: e.resourceKind,
      unit: unitForKind(e.resourceKind),
      flowPerMin: round6(flow),
      demandPerMin: dem,
      supplyPerMin: sup,
      deficitPerMin: deficit,
      isUnitMismatch: edgeUnitMismatch.has(e.id),
      isResourceMismatch: edgeResourceMismatch.has(e.id),
      isBottleneck,
      inCycle,
    });
  }

  for (const n of nodes) {
    const inCycle = cyclicNodeIds.has(n.id);
    const recipe = n.recipeId ? ctx.recipesById.get(n.recipeId) : undefined;
    const machineId = n.machineId ?? recipe?.machineId;
    const machine = machineId ? ctx.machinesById.get(machineId) : undefined;
    const spd = nodeSpeedInfo.get(n.id);
    const ins = inEdges.get(n.id) ?? [];
    const outs = outEdges.get(n.id) ?? [];

    const requestedCrafts = round6(nodeRequestedCrafts.get(n.id) ?? 0);
    const effectiveCrafts = round6(nodeEffectiveCrafts.get(n.id) ?? 0);

    const craftsPerMachine = spd?.craftsPerMin ?? null;
    const reqExact =
      craftsPerMachine !== null && craftsPerMachine > 0 ? round6(Math.max(requestedCrafts, effectiveCrafts) / craftsPerMachine) : null;
    const reqCeil = reqExact !== null ? Math.ceil(reqExact - 1e-9) : null;
    const assignedMachines =
      n.machineCountOverride !== undefined && n.machineCountOverride > 0 ? n.machineCountOverride : reqCeil;

    const absRpm = Math.abs(n.rpm ?? machine?.defaultRpm ?? 0);
    const stressTotalSU =
      machine && machine.stressImpactSU !== null && assignedMachines !== null
        ? round6(machine.stressImpactSU * (machine.supportsRpm ? absRpm : 1) * assignedMachines)
        : null;

    const utilizationRatio =
      reqExact !== null && assignedMachines !== null && assignedMachines > 0
        ? round6(reqExact / assignedMachines)
        : null;

    const inputLines: NodeFlowLine[] = [];
    const outputLines: NodeFlowLine[] = [];
    let deficitDetected = false;

    if (n.kind === 'source') {
      const k = n.resourceKind ?? 'item';
      const cap = n.sourceUnlimited ? Number.POSITIVE_INFINITY : Math.max(0, n.sourceRatePerMin ?? 0);
      const totalDemand = outs.reduce((acc, e) => acc + (edgeDemand.get(e.id) ?? 0), 0);
      if (Number.isFinite(cap) && totalDemand > cap + 1e-5) {
        deficitDetected = true;
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'warning',
            code: 'DEFICIT',
            nodeId: n.id,
            resourceId: n.resourceId,
            messageRu: `Источник «${n.title}» даёт ${cap.toFixed(2)} ${unitForKind(k)}, но цепочка запрашивает ${totalDemand.toFixed(2)} ${unitForKind(k)}.`,
            messageEn: `Source "${n.title}" provides ${cap.toFixed(2)} ${unitForKind(k)}, while downstream requests ${totalDemand.toFixed(2)} ${unitForKind(k)}.`,
          })
        );
      }
      if (n.resourceId) {
        outputLines.push({
          resourceId: n.resourceId,
          kind: k,
          unit: unitForKind(k),
          requiredPerMin: round6(totalDemand),
          suppliedPerMin: Number.isFinite(cap) ? round6(cap) : round6(totalDemand),
          producedExpectedPerMin: Number.isFinite(cap) ? round6(cap) : round6(totalDemand),
          producedGuaranteedPerMin: Number.isFinite(cap) ? round6(cap) : round6(totalDemand),
          probabilistic: false,
          isByproduct: false,
        });
      }
    } else if (n.kind === 'target') {
      const k = n.resourceKind ?? 'item';
      const targetRate = Math.max(0, n.targetRatePerMin ?? 0);
      const supplied = ins
        .filter((e) => !edgeUnitMismatch.has(e.id) && (!n.resourceId || e.resourceId === n.resourceId))
        .reduce((acc, e) => acc + (edgeSupply.get(e.id) ?? 0), 0);
      if (targetRate > supplied + 1e-5 && !inCycle) {
        deficitDetected = true;
        addNodeWarn(
          n.id,
          pushWarn({
            severity: 'warning',
            code: 'DEFICIT',
            nodeId: n.id,
            resourceId: n.resourceId,
            messageRu: `Цель «${n.title}» недовыполнена: поступает ${supplied.toFixed(2)} из ${targetRate.toFixed(2)} ${unitForKind(k)}.`,
            messageEn: `Target "${n.title}" under-supplied: receiving ${supplied.toFixed(2)} of ${targetRate.toFixed(2)} ${unitForKind(k)}.`,
          })
        );
      }
      if (n.resourceId) {
        inputLines.push({
          resourceId: n.resourceId,
          kind: k,
          unit: unitForKind(k),
          requiredPerMin: round6(targetRate),
          suppliedPerMin: round6(supplied),
          producedExpectedPerMin: 0,
          producedGuaranteedPerMin: 0,
          probabilistic: false,
          isByproduct: false,
        });
      }
    } else if (n.kind === 'byproduct') {
      const k = n.resourceKind ?? 'item';
      const supplied = ins.reduce((acc, e) => acc + (edgeSupply.get(e.id) ?? 0), 0);
      if (n.resourceId) {
        inputLines.push({
          resourceId: n.resourceId,
          kind: k,
          unit: unitForKind(k),
          requiredPerMin: 0,
          suppliedPerMin: round6(supplied),
          producedExpectedPerMin: 0,
          producedGuaranteedPerMin: 0,
          probabilistic: false,
          isByproduct: true,
        });
      }
    } else if (recipe) {
      const craftRef = Math.max(requestedCrafts, effectiveCrafts);
      for (const ing of recipe.inputs) {
        if (ing.amount <= 0) continue;
        const reqRate = round6(craftRef * ing.amount);
        const supRate = round6(
          ins
            .filter((e) => e.resourceId === ing.resourceId && !edgeUnitMismatch.has(e.id))
            .reduce((acc, e) => acc + (edgeSupply.get(e.id) ?? 0), 0)
        );
        if (reqRate > supRate + 1e-5 && !inCycle) {
          deficitDetected = true;
          const wired = ins.some((e) => e.resourceId === ing.resourceId && !edgeUnitMismatch.has(e.id));
          if (!wired) {
            addNodeWarn(
              n.id,
              pushWarn({
                severity: 'warning',
                code: 'DEFICIT',
                nodeId: n.id,
                resourceId: ing.resourceId,
                messageRu: `Для «${n.title}» не подключён вход ${ing.resourceId} (требуется ${reqRate.toFixed(2)} ${unitForKind(ing.kind)}).`,
                messageEn: `Node "${n.title}" is missing input wire for ${ing.resourceId} (${reqRate.toFixed(2)} ${unitForKind(ing.kind)} required).`,
              })
            );
          }
        }
        inputLines.push({
          resourceId: ing.resourceId,
          kind: ing.kind,
          unit: unitForKind(ing.kind),
          requiredPerMin: reqRate,
          suppliedPerMin: supRate,
          producedExpectedPerMin: 0,
          producedGuaranteedPerMin: 0,
          probabilistic: false,
          isByproduct: false,
        });
      }

      const seenOutRes = new Set<string>();
      for (const out of recipe.outputs) {
        if (seenOutRes.has(out.resourceId)) continue;
        seenOutRes.add(out.resourceId);
        const expectedPerCraft = recipeExpectedYield(recipe, out.resourceId);
        const guaranteedPerCraft = recipeGuaranteedYield(recipe, out.resourceId);
        const isProb = recipe.outputs.some(
          (o) => o.resourceId === out.resourceId && o.probabilityKind !== 'guaranteed'
        );
        const isPrimary = recipe.outputs[0]?.resourceId === out.resourceId;
        outputLines.push({
          resourceId: out.resourceId,
          kind: out.kind,
          unit: unitForKind(out.kind),
          requiredPerMin: 0,
          suppliedPerMin: 0,
          producedExpectedPerMin: round6(effectiveCrafts * expectedPerCraft),
          producedGuaranteedPerMin: round6(effectiveCrafts * guaranteedPerCraft),
          probabilistic: isProb,
          isByproduct: !isPrimary,
        });
      }
    }

    const isMachCapBottleneck =
      utilizationRatio !== null &&
      n.machineCountOverride !== undefined &&
      n.machineCountOverride > 0 &&
      utilizationRatio > 1 + 1e-5;
    if (isMachCapBottleneck) {
      deficitDetected = true;
      addNodeWarn(
        n.id,
        pushWarn({
          severity: 'warning',
          code: 'BOTTLENECK',
          nodeId: n.id,
          messageRu: `Узел «${n.title}» перегружен: задано ${n.machineCountOverride} машин, требуется ${reqExact?.toFixed(2)}.`,
          messageEn: `Node "${n.title}" overloaded: capped at ${n.machineCountOverride} machines, needs ${reqExact?.toFixed(2)}.`,
        })
      );
    }

    nodeCalculations.push({
      nodeId: n.id,
      componentId,
      inCycle,
      requestedCraftsPerMin: requestedCrafts,
      effectiveCraftsPerMin: effectiveCrafts,
      craftsPerMachinePerMin: craftsPerMachine !== null ? round6(craftsPerMachine) : null,
      speedSource:
        n.kind === 'recipe' || n.kind === 'machine' ? (spd?.source ?? 'unknown') : 'not-applicable',
      requiredMachinesExact: reqExact,
      requiredMachinesCeil: reqCeil,
      assignedMachines,
      utilizationRatio,
      stressTotalSU,
      isBottleneck: deficitDetected || isMachCapBottleneck,
      deficitDetected,
      inputs: inputLines,
      outputs: outputLines,
      warnings: nodeWarnings.get(n.id) ?? [],
    });
  }

  return {
    nodeCalculations,
    edgeCalculations,
    warnings,
    cycles,
  };
}

function getNodePrimaryResourceKind(
  node: FactoryNode,
  resourceId: string,
  side: 'input' | 'output',
  ctx: SolverContext
): ResourceKind | null {
  if (node.kind === 'source' || node.kind === 'target' || node.kind === 'byproduct') {
    if (node.resourceKind) return node.resourceKind;
    if (node.resourceId) return ctx.resourcesById.get(node.resourceId)?.kind ?? null;
    return null;
  }
  if (node.recipeId) {
    const r = ctx.recipesById.get(node.recipeId);
    if (!r) return null;
    const list = side === 'input' ? r.inputs : r.outputs;
    const found = list.find((x) => x.resourceId === resourceId);
    if (found) return found.kind;
  }
  return ctx.resourcesById.get(resourceId)?.kind ?? null;
}

function nodeProducesResource(node: FactoryNode, resourceId: string, ctx: SolverContext): boolean {
  if (node.kind === 'source') return !node.resourceId || node.resourceId === resourceId;
  if (node.kind === 'machine') return true;
  if (node.kind === 'recipe' && node.recipeId) {
    const r = ctx.recipesById.get(node.recipeId);
    if (!r) return false;
    return r.outputs.some((o) => o.resourceId === resourceId);
  }
  return false;
}

function nodeConsumesResource(node: FactoryNode, resourceId: string, ctx: SolverContext): boolean {
  if (node.kind === 'target' || node.kind === 'byproduct') {
    return !node.resourceId || node.resourceId === resourceId;
  }
  if (node.kind === 'machine') return true;
  if (node.kind === 'recipe' && node.recipeId) {
    const r = ctx.recipesById.get(node.recipeId);
    if (!r) return false;
    return r.inputs.some((i) => i.resourceId === resourceId);
  }
  return false;
}

export function solveFactoryProject(
  project: FactoryProject,
  ctx: SolverContext,
  cache?: IncrementalSolverCache
): SolverResult {
  const t0 = performance.now();

  // Partition into weakly connected components using Union-Find
  const parent = new Map<string, string>();
  const find = (x: string): string => {
    let cur = x;
    while (parent.get(cur) !== cur) {
      const p = parent.get(cur)!;
      parent.set(cur, parent.get(p)!);
      cur = p;
    }
    return cur;
  };
  const union = (a: string, b: string) => {
    const ra = find(a);
    const rb = find(b);
    if (ra !== rb) parent.set(rb, ra);
  };

  for (const n of project.nodes) parent.set(n.id, n.id);
  for (const e of project.edges) {
    if (parent.has(e.fromNodeId) && parent.has(e.toNodeId)) {
      union(e.fromNodeId, e.toNodeId);
    }
  }

  const compNodes = new Map<string, FactoryNode[]>();
  const compEdges = new Map<string, FactoryEdge[]>();
  for (const n of project.nodes) {
    const root = find(n.id);
    if (!compNodes.has(root)) {
      compNodes.set(root, []);
      compEdges.set(root, []);
    }
    compNodes.get(root)!.push(n);
  }
  for (const e of project.edges) {
    if (parent.has(e.fromNodeId) && parent.has(e.toNodeId)) {
      compEdges.get(find(e.fromNodeId))!.push(e);
    }
  }

  const nodeMap: Record<string, NodeCalculation> = {};
  const edgeMap: Record<string, EdgeCalculation> = {};
  const warnings: SolverWarning[] = [];
  const cycles: string[][] = [];

  let componentIndex = 0;
  let recomputedComponents = 0;
  let cachedComponents = 0;

  for (const [root, cNodes] of compNodes) {
    const cid = ++componentIndex;
    const cEdges = compEdges.get(root) ?? [];
    const sig = buildComponentSignature(cNodes, cEdges, ctx);

    let sol = cache?.get(sig);
    if (sol) {
      cachedComponents++;
    } else {
      recomputedComponents++;
      const computed = solveComponent(cid, cNodes, cEdges, ctx);
      sol = { signature: sig, ...computed };
      cache?.set(sig, sol);
    }

    for (const nc of sol.nodeCalculations) nodeMap[nc.nodeId] = nc;
    for (const ec of sol.edgeCalculations) edgeMap[ec.edgeId] = ec;
    for (const w of sol.warnings) warnings.push(w);
    for (const c of sol.cycles) cycles.push(c);
  }

  // Aggregate global resource balance (kept strictly separated by resourceId + kind)
  const balMap = new Map<string, ResourceBalanceRow>();
  const ensureBal = (resourceId: string, kind: ResourceKind): ResourceBalanceRow => {
    const key = `${kind}:${resourceId}`;
    let row = balMap.get(key);
    if (!row) {
      row = {
        resourceId,
        kind,
        unit: unitForKind(kind),
        totalSourceSupplyPerMin: 0,
        totalConsumedPerMin: 0,
        totalProducedMainPerMin: 0,
        totalProducedByproductPerMin: 0,
        totalTargetDemandPerMin: 0,
        netBalancePerMin: 0,
        hasProbabilisticSupply: false,
      };
      balMap.set(key, row);
    }
    return row;
  };

  for (const n of project.nodes) {
    const nc = nodeMap[n.id];
    if (!nc) continue;
    if (n.kind === 'source' && n.resourceId) {
      const row = ensureBal(n.resourceId, n.resourceKind ?? 'item');
      const supplied = n.sourceUnlimited
        ? nc.outputs[0]?.requiredPerMin ?? 0
        : Math.max(0, n.sourceRatePerMin ?? 0);
      row.totalSourceSupplyPerMin = round6(row.totalSourceSupplyPerMin + supplied);
    } else if (n.kind === 'target' && n.resourceId) {
      const row = ensureBal(n.resourceId, n.resourceKind ?? 'item');
      row.totalTargetDemandPerMin = round6(row.totalTargetDemandPerMin + (n.targetRatePerMin ?? 0));
    } else {
      for (const inp of nc.inputs) {
        const row = ensureBal(inp.resourceId, inp.kind);
        row.totalConsumedPerMin = round6(row.totalConsumedPerMin + inp.requiredPerMin);
      }
      for (const out of nc.outputs) {
        const row = ensureBal(out.resourceId, out.kind);
        if (out.isByproduct) {
          row.totalProducedByproductPerMin = round6(
            row.totalProducedByproductPerMin + out.producedExpectedPerMin
          );
        } else {
          row.totalProducedMainPerMin = round6(
            row.totalProducedMainPerMin + out.producedExpectedPerMin
          );
        }
        if (out.probabilistic) row.hasProbabilisticSupply = true;
      }
    }
  }

  for (const row of balMap.values()) {
    row.netBalancePerMin = round6(
      row.totalSourceSupplyPerMin +
        row.totalProducedMainPerMin +
        row.totalProducedByproductPerMin -
        row.totalConsumedPerMin -
        row.totalTargetDemandPerMin
    );
  }

  let totalStressSU = 0;
  let totalMachinesRounded = 0;
  for (const nc of Object.values(nodeMap)) {
    if (nc.stressTotalSU !== null) totalStressSU += nc.stressTotalSU;
    if (nc.assignedMachines !== null) totalMachinesRounded += nc.assignedMachines;
  }

  const solveTimeMs = round6(performance.now() - t0);

  return {
    nodeMap,
    edgeMap,
    balances: Array.from(balMap.values()).sort((a, b) => a.resourceId.localeCompare(b.resourceId)),
    warnings,
    cycles,
    stats: {
      totalNodes: project.nodes.length,
      totalEdges: project.edges.length,
      componentCount: compNodes.size,
      recomputedComponents,
      cachedComponents,
      solveTimeMs,
      totalStressSU: round6(totalStressSU),
      totalMachinesRounded,
      cycleCount: cycles.length,
    },
  };
}
