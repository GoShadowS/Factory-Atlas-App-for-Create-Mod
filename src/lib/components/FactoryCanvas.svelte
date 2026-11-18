<script lang="ts">
  import { t } from '../i18n/translations';
  import type {
    AssetManifestEntry,
    FactoryEdge,
    FactoryNode,
    Locale,
    RecipeDef,
    ResourceDef,
    ResourceKind,
    SolverResult,
  } from '../types/atlas';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    nodes: FactoryNode[];
    edges: FactoryEdge[];
    solver: SolverResult;
    locale: Locale;
    selectedNodeIds: string[];
    selectedEdgeId: string | null;
    recipesById: Map<string, RecipeDef>;
    resourcesById: Map<string, ResourceDef>;
    assetsById: Map<string, AssetManifestEntry>;
    focusNodeId?: string | null;
    onSelectNodes: (ids: string[]) => void;
    onSelectEdge: (id: string | null) => void;
    onMoveNodes: (moves: Array<{ id: string; x: number; y: number }>, commitHistory: boolean) => void;
    onCreateEdge: (params: {
      fromNodeId: string;
      toNodeId: string;
      resourceId: string;
      resourceKind: ResourceKind;
      isByproductPort?: boolean;
    }) => void;
    onDeleteSelected: () => void;
    onDuplicateSelected: () => void;
    onOpenCreateResource?: () => void;
    onOpenCreateMachine?: () => void;
    onOpenManualRecipe?: () => void;
    onOpenProjectFile?: () => void;
    onOpenSchematic?: () => void;
  }

  let {
    nodes,
    edges,
    solver,
    locale,
    selectedNodeIds,
    selectedEdgeId,
    recipesById,
    resourcesById,
    assetsById,
    focusNodeId = null,
    onSelectNodes,
    onSelectEdge,
    onMoveNodes,
    onCreateEdge,
    onDeleteSelected,
    onDuplicateSelected,
    onOpenCreateResource,
    onOpenCreateMachine,
    onOpenManualRecipe,
    onOpenProjectFile,
    onOpenSchematic,
  }: Props = $props();

  const tr = $derived(t(locale));
  const NODE_W = 240;
  const NODE_H = 128;

  let containerEl: HTMLDivElement | undefined = $state();
  let panX = $state(40);
  let panY = $state(30);
  let zoom = $state(1);

  let draggingPan = $state(false);
  let panStart = { x: 0, y: 0, origX: 0, origY: 0 };

  let draggingNodes: {
    startClientX: number;
    startClientY: number;
    origPos: Map<string, { x: number; y: number }>;
    moved: boolean;
  } | null = $state(null);

  let wiringFrom: {
    nodeId: string;
    resourceId: string;
    resourceKind: ResourceKind;
    isByproductPort?: boolean;
    startX: number;
    startY: number;
    curX: number;
    curY: number;
  } | null = $state(null);

  // Center canvas when focusNodeId changes
  $effect(() => {
    if (!focusNodeId || !containerEl) return;
    const node = nodes.find((n) => n.id === focusNodeId);
    if (!node) return;
    const rect = containerEl.getBoundingClientRect();
    panX = rect.width / 2 - (node.x + NODE_W / 2) * zoom;
    panY = rect.height / 2 - (node.y + NODE_H / 2) * zoom;
  });

  const nodeMap = $derived(new Map(nodes.map((n) => [n.id, n])));
  const selectedSet = $derived(new Set(selectedNodeIds));

  // Lightweight viewport culling only when graph is huge (>140 nodes)
  const visibleNodes = $derived.by(() => {
    if (nodes.length <= 140 || !containerEl) return nodes;
    const w = containerEl.clientWidth || 1200;
    const h = containerEl.clientHeight || 800;
    const minX = -panX / zoom - 280;
    const minY = -panY / zoom - 200;
    const maxX = (-panX + w) / zoom + 280;
    const maxY = (-panY + h) / zoom + 200;
    return nodes.filter((n) => n.x + NODE_W >= minX && n.x <= maxX && n.y + NODE_H >= minY && n.y <= maxY);
  });

  const visibleNodeSet = $derived(new Set(visibleNodes.map((n) => n.id)));
  const visibleEdges = $derived(
    nodes.length <= 140
      ? edges
      : edges.filter((e) => visibleNodeSet.has(e.fromNodeId) || visibleNodeSet.has(e.toNodeId))
  );

  function screenToWorld(clientX: number, clientY: number) {
    const rect = containerEl!.getBoundingClientRect();
    return {
      x: (clientX - rect.left - panX) / zoom,
      y: (clientY - rect.top - panY) / zoom,
    };
  }

  function handleBgPointerDown(e: PointerEvent) {
    if (e.button !== 0 && e.button !== 1) return;
    const target = e.target as HTMLElement;
    if (target.closest('.node-card') || target.closest('.edge-hit') || target.closest('.canvas-hud')) {
      return;
    }
    wiringFrom = null;
    onSelectEdge(null);
    if (!e.shiftKey) onSelectNodes([]);
    draggingPan = true;
    panStart = { x: e.clientX, y: e.clientY, origX: panX, origY: panY };
  }

  function handleNodePointerDown(e: PointerEvent, node: FactoryNode) {
    if (e.button !== 0) return;
    if ((e.target as HTMLElement).closest('.port-btn')) return;
    e.stopPropagation();

    let nextSel = selectedNodeIds;
    if (e.shiftKey) {
      nextSel = selectedSet.has(node.id)
        ? selectedNodeIds.filter((id) => id !== node.id)
        : [...selectedNodeIds, node.id];
      onSelectNodes(nextSel);
    } else if (!selectedSet.has(node.id)) {
      nextSel = [node.id];
      onSelectNodes(nextSel);
    }
    onSelectEdge(null);

    const origPos = new Map<string, { x: number; y: number }>();
    for (const id of nextSel) {
      const found = nodeMap.get(id);
      if (found) origPos.set(id, { x: found.x, y: found.y });
    }
    draggingNodes = {
      startClientX: e.clientX,
      startClientY: e.clientY,
      origPos,
      moved: false,
    };
  }

  function handleWindowPointerMove(e: PointerEvent) {
    if (draggingPan) {
      panX = panStart.origX + (e.clientX - panStart.x);
      panY = panStart.origY + (e.clientY - panStart.y);
      return;
    }
    if (draggingNodes) {
      const dx = (e.clientX - draggingNodes.startClientX) / zoom;
      const dy = (e.clientY - draggingNodes.startClientY) / zoom;
      if (Math.abs(dx) > 2 || Math.abs(dy) > 2) draggingNodes.moved = true;
      const moves: Array<{ id: string; x: number; y: number }> = [];
      for (const [id, orig] of draggingNodes.origPos) {
        moves.push({
          id,
          x: Math.round((orig.x + dx) / 10) * 10,
          y: Math.round((orig.y + dy) / 10) * 10,
        });
      }
      onMoveNodes(moves, false);
      return;
    }
    if (wiringFrom && containerEl) {
      const w = screenToWorld(e.clientX, e.clientY);
      wiringFrom = { ...wiringFrom, curX: w.x, curY: w.y };
    }
  }

  function handleWindowPointerUp(e: PointerEvent) {
    if (draggingPan) {
      draggingPan = false;
    }
    if (draggingNodes) {
      if (draggingNodes.moved) {
        const dx = (e.clientX - draggingNodes.startClientX) / zoom;
        const dy = (e.clientY - draggingNodes.startClientY) / zoom;
        const moves: Array<{ id: string; x: number; y: number }> = [];
        for (const [id, orig] of draggingNodes.origPos) {
          moves.push({
            id,
            x: Math.round((orig.x + dx) / 10) * 10,
            y: Math.round((orig.y + dy) / 10) * 10,
          });
        }
        onMoveNodes(moves, true);
      }
      draggingNodes = null;
    }
    if (wiringFrom) {
      const el = document.elementFromPoint(e.clientX, e.clientY) as HTMLElement | null;
      const targetPort = el?.closest('[data-input-node-id]') as HTMLElement | null;
      if (targetPort) {
        const toNodeId = targetPort.dataset.inputNodeId!;
        const toRes = targetPort.dataset.inputResourceId || wiringFrom.resourceId;
        if (toNodeId !== wiringFrom.nodeId) {
          onCreateEdge({
            fromNodeId: wiringFrom.nodeId,
            toNodeId,
            resourceId: wiringFrom.resourceId || toRes,
            resourceKind: wiringFrom.resourceKind,
            isByproductPort: wiringFrom.isByproductPort,
          });
          wiringFrom = null;
        }
      }
    }
  }

  function startWiring(
    e: PointerEvent,
    node: FactoryNode,
    resourceId: string,
    resourceKind: ResourceKind,
    isByproductPort = false
  ) {
    e.stopPropagation();
    const startX = node.x + NODE_W;
    const startY = node.y + NODE_H / 2;
    const cur = containerEl ? screenToWorld(e.clientX, e.clientY) : { x: startX + 40, y: startY };
    wiringFrom = {
      nodeId: node.id,
      resourceId,
      resourceKind,
      isByproductPort,
      startX,
      startY,
      curX: cur.x,
      curY: cur.y,
    };
  }

  function finishWiringClick(e: MouseEvent, targetNode: FactoryNode, preferredResId?: string) {
    e.stopPropagation();
    if (!wiringFrom || wiringFrom.nodeId === targetNode.id) return;
    onCreateEdge({
      fromNodeId: wiringFrom.nodeId,
      toNodeId: targetNode.id,
      resourceId: wiringFrom.resourceId || preferredResId || 'create:andesite_alloy',
      resourceKind: wiringFrom.resourceKind,
      isByproductPort: wiringFrom.isByproductPort,
    });
    wiringFrom = null;
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    if (!containerEl) return;
    const rect = containerEl.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const factor = e.deltaY < 0 ? 1.1 : 0.91;
    const nextZoom = Math.min(2.2, Math.max(0.25, zoom * factor));
    panX = mx - ((mx - panX) / zoom) * nextZoom;
    panY = my - ((my - panY) / zoom) * nextZoom;
    zoom = nextZoom;
  }

  export function fitView() {
    if (!containerEl || nodes.length === 0) {
      panX = 40;
      panY = 30;
      zoom = 1;
      return;
    }
    const rect = containerEl.getBoundingClientRect();
    const minX = Math.min(...nodes.map((n) => n.x));
    const minY = Math.min(...nodes.map((n) => n.y));
    const maxX = Math.max(...nodes.map((n) => n.x + NODE_W));
    const maxY = Math.max(...nodes.map((n) => n.y + NODE_H));
    const gw = Math.max(200, maxX - minX + 120);
    const gh = Math.max(200, maxY - minY + 120);
    const nextZoom = Math.min(1.25, Math.max(0.3, Math.min(rect.width / gw, rect.height / gh)));
    zoom = nextZoom;
    panX = (rect.width - (maxX - minX) * nextZoom) / 2 - minX * nextZoom;
    panY = (rect.height - (maxY - minY) * nextZoom) / 2 - minY * nextZoom;
  }

  function getOutputPorts(node: FactoryNode): Array<{ resourceId: string; kind: ResourceKind; isByproduct: boolean }> {
    if (node.kind === 'source') {
      return [
        {
          resourceId: node.resourceId || 'minecraft:andesite',
          kind: node.resourceKind ?? 'item',
          isByproduct: false,
        },
      ];
    }
    if (node.kind === 'recipe' && node.recipeId) {
      const r = recipesById.get(node.recipeId);
      if (r && r.outputs.length > 0) {
        const seen = new Set<string>();
        const list: Array<{ resourceId: string; kind: ResourceKind; isByproduct: boolean }> = [];
        r.outputs.forEach((o, idx) => {
          if (seen.has(o.resourceId)) return;
          seen.add(o.resourceId);
          list.push({ resourceId: o.resourceId, kind: o.kind, isByproduct: idx > 0 });
        });
        return list;
      }
    }
    if (node.kind === 'machine') {
      return [{ resourceId: node.resourceId || 'create:andesite_alloy', kind: node.resourceKind ?? 'item', isByproduct: false }];
    }
    return [];
  }

  function getInputPorts(node: FactoryNode): Array<{ resourceId: string; kind: ResourceKind }> {
    if (node.kind === 'target' || node.kind === 'byproduct') {
      return [{ resourceId: node.resourceId || '', kind: node.resourceKind ?? 'item' }];
    }
    if (node.kind === 'recipe' && node.recipeId) {
      const r = recipesById.get(node.recipeId);
      if (r) return r.inputs.filter((i) => i.amount > 0).map((i) => ({ resourceId: i.resourceId, kind: i.kind }));
    }
    if (node.kind === 'machine') {
      return [{ resourceId: '', kind: node.resourceKind ?? 'item' }];
    }
    return [];
  }

  function primarySpriteId(node: FactoryNode): string {
    if (node.resourceId) return node.resourceId;
    if (node.machineId) return node.machineId;
    if (node.recipeId) {
      const r = recipesById.get(node.recipeId);
      if (r) return r.outputs[0]?.resourceId || r.machineId;
    }
    return '';
  }
</script>

<svelte:window
  onpointermove={handleWindowPointerMove}
  onpointerup={handleWindowPointerUp}
/>

<div
  bind:this={containerEl}
  class="canvas-viewport"
  role="region"
  aria-label={tr.canvasTitle}
  onpointerdown={handleBgPointerDown}
  onwheel={handleWheel}
>
  <div
    class="grid-bg"
    style={`background-position:${panX}px ${panY}px;background-size:${32 * zoom}px ${32 * zoom}px;`}
  ></div>

  <div
    class="world-layer"
    style={`transform:translate(${panX}px, ${panY}px) scale(${zoom});transform-origin:0 0;`}
  >
    <svg class="edges-svg" width="8000" height="8000" viewBox="-2000 -2000 8000 8000">
      {#each visibleEdges as edge (edge.id)}
        {@const a = nodeMap.get(edge.fromNodeId)}
        {@const b = nodeMap.get(edge.toNodeId)}
        {#if a && b}
          {@const x1 = a.x + NODE_W}
          {@const y1 = a.y + NODE_H / 2}
          {@const x2 = b.x}
          {@const y2 = b.y + NODE_H / 2}
          {@const dx = Math.max(48, Math.abs(x2 - x1) * 0.45)}
          {@const pathD = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`}
          {@const ec = solver.edgeMap[edge.id]}
          {@const isSelected = selectedEdgeId === edge.id}
          {@const stroke =
            ec?.isUnitMismatch || ec?.inCycle
              ? '#ef4444'
              : ec?.isBottleneck
                ? '#f97316'
                : edge.resourceKind === 'fluid'
                  ? '#38bdf8'
                  : '#f59e0b'}
          {@const mx = (x1 + x2) / 2}
          {@const my = (y1 + y2) / 2}
          {@const res = resourcesById.get(edge.resourceId)}
          {@const resLabel = res ? (locale === 'ru' ? res.nameRu : res.nameEn) : edge.resourceId}
          {@const unitShort = edge.resourceKind === 'fluid' ? (locale === 'ru' ? 'мБ/м' : 'mB/m') : (locale === 'ru' ? 'шт/м' : '/m')}
          <g
            class="edge-hit"
            role="button"
            tabindex="0"
            onclick={(ev) => {
              ev.stopPropagation();
              onSelectNodes([]);
              onSelectEdge(edge.id);
            }}
            onkeydown={(ev) => {
              if (ev.key === 'Enter' || ev.key === ' ') {
                onSelectNodes([]);
                onSelectEdge(edge.id);
              }
            }}
          >
            <path d={pathD} fill="none" stroke="transparent" stroke-width="14" />
            <path
              d={pathD}
              fill="none"
              stroke={stroke}
              stroke-width={isSelected ? 4 : 2.5}
              stroke-dasharray={edge.isByproductPort ? '6 4' : undefined}
              opacity={isSelected ? 1 : 0.88}
            />
            <rect
              x={mx - 82}
              y={my - 12}
              width="164"
              height="24"
              rx="6"
              fill={isSelected ? 'var(--accent-soft)' : 'var(--bg-panel)'}
              stroke={stroke}
              stroke-width={isSelected ? 2 : 1.2}
            />
            <text
              x={mx}
              y={my + 4}
              text-anchor="middle"
              fill="var(--text-primary)"
              font-size="10.5"
              font-weight="600"
            >
              {resLabel.slice(0, 15)}: {(ec?.flowPerMin ?? 0).toFixed(1)} {unitShort}
            </text>
          </g>
        {/if}
      {/each}

      {#if wiringFrom}
        {@const dx = Math.max(40, Math.abs(wiringFrom.curX - wiringFrom.startX) * 0.4)}
        <path
          d={`M ${wiringFrom.startX} ${wiringFrom.startY} C ${wiringFrom.startX + dx} ${wiringFrom.startY}, ${wiringFrom.curX - dx} ${wiringFrom.curY}, ${wiringFrom.curX} ${wiringFrom.curY}`}
          fill="none"
          stroke={wiringFrom.resourceKind === 'fluid' ? '#38bdf8' : '#f59e0b'}
          stroke-width="2.5"
          stroke-dasharray="5 4"
        />
      {/if}
    </svg>

    {#each visibleNodes as node (node.id)}
      {@const nc = solver.nodeMap[node.id]}
      {@const isSelected = selectedSet.has(node.id)}
      {@const outPorts = getOutputPorts(node)}
      {@const inPorts = getInputPorts(node)}
      {@const recipe = node.recipeId ? recipesById.get(node.recipeId) : undefined}
      <div
        class={`node-card kind-${node.kind}`}
        class:selected={isSelected}
        class:bottleneck={nc?.isBottleneck}
        class:incycle={nc?.inCycle}
        style={`left:${node.x}px;top:${node.y}px;width:${NODE_W}px;min-height:${NODE_H}px;`}
        role="button"
        tabindex="0"
        onpointerdown={(e) => handleNodePointerDown(e, node)}
        onkeydown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            onSelectNodes([node.id]);
          }
        }}
      >
        <div class="node-header">
          <span class="node-kind-badge">{tr.nodeKinds[node.kind]}</span>
          {#if nc?.inCycle}
            <span class="status-tag tag-error">{locale === 'ru' ? 'Цикл' : 'Cycle'}</span>
          {:else if nc?.isBottleneck}
            <span class="status-tag tag-warn">{tr.bottleneckLabel}</span>
          {:else if nc?.speedSource === 'unknown'}
            <span class="status-tag tag-warn">RPM?</span>
          {:else if nc?.speedSource === 'assumption'}
            <span class="status-tag tag-info">{locale === 'ru' ? 'Допущ.' : 'Assump.'}</span>
          {/if}
        </div>

        <div class="node-body">
          <PixelSprite asset={assetsById.get(primarySpriteId(node))} size={28} />
          <div class="node-title-wrap">
            <div class="node-title" title={node.title}>{node.title}</div>
            <div class="node-sub">
              {#if node.kind === 'source'}
                <span class={node.resourceKind === 'fluid' ? 'u-fluid' : 'u-item'}>
                  {node.sourceUnlimited
                    ? '∞'
                    : `${node.sourceRatePerMin ?? 0} ${node.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin}`}
                </span>
              {:else if node.kind === 'target'}
                {@const got = nc?.inputs[0]?.suppliedPerMin ?? 0}
                <span class={node.resourceKind === 'fluid' ? 'u-fluid' : 'u-item'}>
                  {got.toFixed(1)} / {(node.targetRatePerMin ?? 0).toFixed(1)} {node.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin}
                </span>
              {:else if node.kind === 'byproduct'}
                <span class={node.resourceKind === 'fluid' ? 'u-fluid' : 'u-item'}>
                  +{(nc?.inputs[0]?.suppliedPerMin ?? 0).toFixed(1)} {node.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin}
                </span>
              {:else if node.kind === 'note'}
                <span>{(node.noteText ?? '').slice(0, 65)}</span>
              {:else if nc}
                <span>
                  {locale === 'ru' ? 'Машин' : 'Mach'}:
                  <strong>
                    {nc.requiredMachinesExact !== null
                      ? `${nc.requiredMachinesExact.toFixed(2)} (${nc.requiredMachinesCeil})`
                      : '?'}
                  </strong>
                  {#if nc.stressTotalSU !== null}
                    · {nc.stressTotalSU} SU
                  {/if}
                </span>
              {/if}
            </div>
            {#if recipe && recipe.outputs.some((o) => o.probabilityKind !== 'guaranteed')}
              <div class="prob-note">
                ≈ {locale === 'ru' ? 'вероятностный средний расчёт' : 'probabilistic mean'}
              </div>
            {/if}
          </div>
        </div>

        {#if inPorts.length > 0}
          <div class="ports-col ports-left">
            {#each inPorts as p, idx (p.resourceId + idx)}
              <button
                type="button"
                class={`port-btn port-${p.kind}`}
                data-input-node-id={node.id}
                data-input-resource-id={p.resourceId}
                title={locale === 'ru'
                  ? `Вход: ${p.resourceId || 'любой'} (${p.kind === 'fluid' ? 'мБ/мин' : 'предм./мин'})`
                  : `Input: ${p.resourceId || 'any'} (${p.kind})`}
                onclick={(e) => finishWiringClick(e, node, p.resourceId)}
              >
                ◀
              </button>
            {/each}
          </div>
        {/if}

        {#if outPorts.length > 0}
          <div class="ports-col ports-right">
            {#each outPorts as p (p.resourceId)}
              <button
                type="button"
                class={`port-btn port-${p.kind}`}
                class:port-byproduct={p.isByproduct}
                title={locale === 'ru'
                  ? `Выход${p.isByproduct ? ' (побочный)' : ''}: ${p.resourceId} (${p.kind === 'fluid' ? 'мБ/мин' : 'предм./мин'}) — тяните или кликните`
                  : `Output: ${p.resourceId} (${p.kind}) — drag or click`}
                onpointerdown={(e) => startWiring(e, node, p.resourceId, p.kind, p.isByproduct)}
              >
                ▶
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/each}
  </div>

  {#if nodes.length === 0}
    <div class="empty-canvas-overlay">
      <div class="empty-card">
        <img src="/assets-fallback/factory-atlas-icon.svg" alt="" width="48" height="48" />
        <h3>{tr.emptyCanvasTitle}</h3>
        <p>
          {locale === 'ru'
            ? 'Новый проект полностью пуст — создайте свои ресурсы, машины и рецепты или откройте существующий проект / схематику.'
            : 'This project starts empty — create your resources, machines, and recipes manually or open an existing project / schematic.'}
        </p>
        <div class="empty-actions">
          {#if onOpenCreateResource}
            <button type="button" class="btn-primary btn-xs" onclick={onOpenCreateResource}>
              {locale === 'ru' ? '+ Создать ресурс' : '+ Create Resource'}
            </button>
          {/if}
          {#if onOpenCreateMachine}
            <button type="button" class="btn-secondary btn-xs" onclick={onOpenCreateMachine}>
              {locale === 'ru' ? '+ Создать машину' : '+ Create Machine'}
            </button>
          {/if}
          {#if onOpenManualRecipe}
            <button type="button" class="btn-secondary btn-xs" onclick={onOpenManualRecipe}>
              {locale === 'ru' ? '+ Создать рецепт' : '+ Create Recipe'}
            </button>
          {/if}
          {#if onOpenProjectFile}
            <button type="button" class="btn-secondary btn-xs" onclick={onOpenProjectFile}>
              {locale === 'ru' ? 'Открыть .factoryatlas' : 'Open .factoryatlas'}
            </button>
          {/if}
          {#if onOpenSchematic}
            <button type="button" class="btn-secondary btn-xs" onclick={onOpenSchematic}>
              {locale === 'ru' ? 'Просмотр .nbt' : 'Open .nbt'}
            </button>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <div class="canvas-hud">
    <div class="hud-help">{wiringFrom ? tr.selectSourcePrompt : tr.connectHelp}</div>
    <div class="hud-controls">
      {#if selectedNodeIds.length > 0 || selectedEdgeId}
        <button type="button" class="btn-secondary btn-xs" onclick={onDuplicateSelected} title="Ctrl+D">
          {tr.duplicate}
        </button>
        <button type="button" class="btn-danger btn-xs" onclick={onDeleteSelected} title="Delete">
          {tr.deleteSelected}
        </button>
      {/if}
      <button type="button" class="btn-secondary btn-xs" onclick={() => (zoom = Math.max(0.25, zoom * 0.85))}>
        −
      </button>
      <span class="zoom-pill">{Math.round(zoom * 100)}%</span>
      <button type="button" class="btn-secondary btn-xs" onclick={() => (zoom = Math.min(2.2, zoom * 1.15))}>
        +
      </button>
      <button type="button" class="btn-secondary btn-xs" onclick={fitView}>
        {tr.zoomFit}
      </button>
    </div>
  </div>
</div>

<style>
  .canvas-viewport {
    position: relative;
    flex: 1;
    overflow: hidden;
    background: var(--bg-canvas);
    user-select: none;
    cursor: grab;
  }
  .canvas-viewport:active {
    cursor: grabbing;
  }

  .grid-bg {
    position: absolute;
    inset: 0;
    background-image:
      linear-gradient(to right, var(--canvas-grid) 1px, transparent 1px),
      linear-gradient(to bottom, var(--canvas-grid) 1px, transparent 1px);
    pointer-events: none;
  }

  .world-layer {
    position: absolute;
    left: 0;
    top: 0;
    width: 1px;
    height: 1px;
  }

  .edges-svg {
    position: absolute;
    left: -2000px;
    top: -2000px;
    overflow: visible;
    pointer-events: none;
  }

  .edge-hit {
    pointer-events: all;
    cursor: pointer;
  }

  .node-card {
    position: absolute;
    background: var(--bg-panel);
    border: 1.5px solid var(--border-strong);
    border-radius: 10px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.24);
    padding: 8px 12px 10px;
    box-sizing: border-box;
    cursor: move;
    transition: border-color 0.12s, box-shadow 0.12s;
  }
  .node-card:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
  .node-card.selected {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-soft), 0 8px 24px rgba(0, 0, 0, 0.32);
  }
  .node-card.bottleneck {
    border-color: #f97316;
  }
  .node-card.incycle {
    border-color: #ef4444;
  }

  .node-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    margin-bottom: 6px;
    font-size: 10px;
  }

  .node-kind-badge {
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
  }
  .kind-source .node-kind-badge { color: #34d399; }
  .kind-target .node-kind-badge { color: #38bdf8; }
  .kind-recipe .node-kind-badge { color: #f59e0b; }
  .kind-machine .node-kind-badge { color: #fbbf24; }
  .kind-byproduct .node-kind-badge { color: #c084fc; }

  .status-tag {
    padding: 1px 6px;
    border-radius: 999px;
    font-weight: 700;
    font-size: 9.5px;
  }
  .tag-error { background: rgba(239, 68, 68, 0.2); color: #fca5a5; }
  .tag-warn { background: rgba(249, 115, 22, 0.2); color: #fdba74; }
  .tag-info { background: rgba(56, 189, 248, 0.2); color: #7dd3fc; }

  .node-body {
    display: flex;
    align-items: flex-start;
    gap: 9px;
  }

  .node-title-wrap {
    flex: 1;
    min-width: 0;
  }

  .node-title {
    font-size: 12.5px;
    font-weight: 700;
    color: var(--text-primary);
    line-height: 1.25;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .node-sub {
    margin-top: 4px;
    font-size: 11px;
    color: var(--text-secondary);
    line-height: 1.3;
    word-break: break-word;
  }

  .u-fluid { color: #38bdf8; font-weight: 600; }
  .u-item { color: #f59e0b; font-weight: 600; }

  .prob-note {
    margin-top: 3px;
    font-size: 10px;
    color: #c084fc;
  }

  .ports-col {
    position: absolute;
    top: 50%;
    transform: translateY(-50%);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .ports-left { left: -11px; }
  .ports-right { right: -11px; }

  .port-btn {
    width: 20px;
    height: 20px;
    border-radius: 999px;
    border: 2px solid var(--bg-canvas);
    font-size: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: crosshair;
    color: #0f172a;
    font-weight: 700;
    padding: 0;
  }
  .port-item { background: #f59e0b; }
  .port-fluid { background: #38bdf8; }
  .port-byproduct { background: #c084fc; }
  .port-btn:hover { transform: scale(1.18); }

  .empty-canvas-overlay {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    pointer-events: none;
    text-align: center;
    padding: 24px;
    color: var(--text-muted);
  }
  .empty-canvas-overlay h3 {
    margin: 12px 0 6px;
    color: var(--text-primary);
    font-size: 16px;
  }
  .empty-canvas-overlay p {
    max-width: 460px;
    font-size: 13px;
    margin: 0;
    line-height: 1.45;
  }
  .empty-card {
    pointer-events: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 14px;
    padding: 24px 28px;
    box-shadow: 0 18px 40px rgba(0, 0, 0, 0.25);
  }
  .empty-actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 8px;
    margin-top: 14px;
  }

  .canvas-hud {
    position: absolute;
    left: 12px;
    right: 12px;
    bottom: 12px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    pointer-events: none;
  }

  .hud-help {
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 6px 10px;
    font-size: 11.5px;
    color: var(--text-secondary);
    pointer-events: auto;
    opacity: 0.94;
  }

  .hud-controls {
    display: flex;
    align-items: center;
    gap: 5px;
    background: var(--bg-panel);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 4px 6px;
    pointer-events: auto;
  }

  .zoom-pill {
    font-size: 11px;
    font-weight: 600;
    min-width: 40px;
    text-align: center;
    color: var(--text-primary);
  }
</style>
