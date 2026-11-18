<script lang="ts">
  import { buildSyntheticDemoSchematicNbt, parseCreateSchematicNbt } from '../io/nbtParser';
  import {
    createBrowserAssetFetcher,
    McModelResolver,
    type ResolvedMcModel,
  } from '../models/mcModelResolver';
  import {
    renderBlocksScene3D,
    type Camera3D,
    type SceneBlockInstance,
  } from '../models/renderer3d';
  import { preloadTexturesForPaths } from '../models/textureCache';
  import type { AssetManifestEntry, Locale, ParsedSchematic } from '../types/atlas';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    locale: Locale;
    assetsById: Map<string, AssetManifestEntry>;
    onClose: () => void;
  }

  let { locale, assetsById, onClose }: Props = $props();

  let schematic: ParsedSchematic | null = $state(null);
  let errorMsg: string | null = $state(null);
  let loading3D = $state(false);
  let canvasEl: HTMLCanvasElement | undefined = $state();

  let yawDeg = $state(35);
  let pitchDeg = $state(28);
  let zoomFactor = $state(1.0);
  let panX = $state(0);
  let panY = $state(0);
  let maxLayerY = $state(999);
  let paletteSearch = $state('');

  let resolvedByStateKey = $state<Map<string, ResolvedMcModel>>(new Map());
  let loadedTextures = $state<Map<string, HTMLImageElement>>(new Map());
  let lastRenderStats = $state({ renderedFaces: 0, renderedCuboids: 0, missingTextures: 0 });

  let dragMode: 'orbit' | 'pan' | null = null;
  let dragStart = { x: 0, y: 0, yaw: 35, pitch: 28, ox: 0, oy: 0 };

  const modelResolver = new McModelResolver(createBrowserAssetFetcher());

  function blockStateKey(blockId: string, props: Record<string, string>): string {
    const sorted = Object.keys(props)
      .sort()
      .map((k) => `${k}=${props[k]}`)
      .join(',');
    return `${blockId}[${sorted}]`;
  }

  function colorForBlock(blockId: string): string {
    let h = 0;
    for (let i = 0; i < blockId.length; i++) h = (h * 31 + blockId.charCodeAt(i)) | 0;
    if (blockId.includes('brass')) return '#d97706';
    if (blockId.includes('copper')) return '#ea580c';
    if (blockId.includes('andesite')) return '#94a3b8';
    if (blockId.includes('belt')) return '#334155';
    if (blockId.includes('press') || blockId.includes('mixer')) return '#64748b';
    if (blockId.includes('cogwheel') || blockId.includes('shaft')) return '#b45309';
    const hue = Math.abs(h) % 360;
    return `hsl(${hue}, 42%, 52%)`;
  }

  async function resolveSchematicModels(sch: ParsedSchematic) {
    loading3D = true;
    try {
      const uniqueStates = new Map<string, { blockId: string; props: Record<string, string> }>();
      for (const blk of sch.blocks) {
        const key = blockStateKey(blk.blockId, blk.properties);
        if (!uniqueStates.has(key)) {
          uniqueStates.set(key, { blockId: blk.blockId, props: blk.properties });
        }
      }

      const entries = Array.from(uniqueStates.entries());
      const resolvedMap = new Map<string, ResolvedMcModel>();
      const texPaths = new Set<string>();

      // Resolve in controlled batches so UI stays smooth on large schematics
      const results = await Promise.all(
        entries.map(async ([key, st]) => [key, await modelResolver.resolveBlockWithState(st.blockId, st.props)] as const)
      );
      for (const [key, res] of results) {
        resolvedMap.set(key, res);
        for (const tp of res.texturePaths) texPaths.add(tp);
      }

      const texMap = await preloadTexturesForPaths(Array.from(texPaths));
      resolvedByStateKey = resolvedMap;
      loadedTextures = texMap;
    } finally {
      loading3D = false;
    }
  }

  async function handleFileInput(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    errorMsg = null;
    try {
      const buf = new Uint8Array(await file.arrayBuffer());
      const parsed = parseCreateSchematicNbt(buf, file.name, { isSyntheticDemo: false });
      schematic = parsed;
      maxLayerY = Math.max(0, parsed.size[1] - 1);
      resetCamera();
      await resolveSchematicModels(parsed);
    } catch (err) {
      schematic = null;
      errorMsg = (err as Error).message;
    }
  }

  async function loadSyntheticDemo() {
    errorMsg = null;
    const demoBytes = buildSyntheticDemoSchematicNbt(true);
    const parsed = parseCreateSchematicNbt(demoBytes, 'synthetic-press-line-demo.nbt', {
      isSyntheticDemo: true,
    });
    schematic = parsed;
    maxLayerY = Math.max(0, parsed.size[1] - 1);
    resetCamera();
    await resolveSchematicModels(parsed);
  }

  function resetCamera() {
    yawDeg = 35;
    pitchDeg = 28;
    zoomFactor = 1.0;
    panX = 0;
    panY = 0;
  }

  $effect(() => {
    const cvs = canvasEl;
    const sch = schematic;
    if (!cvs || !sch) return;
    const ctx = cvs.getContext('2d');
    if (!ctx) return;

    const w = cvs.width;
    const h = cvs.height;
    const [sx, sy, sz] = sch.size;
    const maxDim = Math.max(sx, sy, sz, 1);
    const baseScale = Math.min(w, h) * 0.56 / maxDim;

    const cam: Camera3D = {
      yawDeg,
      pitchDeg,
      zoom: baseScale * zoomFactor,
      panX,
      panY,
    };

    const visibleBlocks: SceneBlockInstance[] = [];
    // Cap rendered blocks on huge schematics to keep 60fps interaction cleanly
    const filtered = sch.blocks.filter((b) => b.pos[1] <= maxLayerY);
    const step = filtered.length > 3500 ? Math.ceil(filtered.length / 3500) : 1;

    for (let i = 0; i < filtered.length; i += step) {
      const b = filtered[i];
      const key = blockStateKey(b.blockId, b.properties);
      const model = resolvedByStateKey.get(key);
      visibleBlocks.push({
        bx: b.pos[0],
        by: b.pos[1],
        bz: b.pos[2],
        cuboids: model?.status === '3d-json-elements' ? model.cuboids : [],
        placeholderColor: colorForBlock(b.blockId),
      });
    }

    // Background
    ctx.fillStyle = '#0b1120';
    ctx.fillRect(0, 0, w, h);

    lastRenderStats = renderBlocksScene3D({
      ctx,
      width: w,
      height: h,
      blocks: visibleBlocks,
      centerWorld: { x: sx / 2, y: sy / 2, z: sz / 2 },
      camera: cam,
      texturesByPath: loadedTextures,
    });
  });

  function exportSchematicBomCsv() {
    if (!schematic) return;
    const rows = ['BlockID,Count'];
    for (const p of schematic.palette) {
      rows.push(`"${p.name.replace(/"/g, '""')}",${p.count}`);
    }
    const blob = new Blob(['\uFEFF' + rows.join('\r\n')], { type: 'text/csv;charset=utf-8' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `${schematic.fileName.replace(/\.nbt$/i, '')}-bom.csv`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function statusForPaletteItem(name: string, props: Record<string, string>): string {
    const model = resolvedByStateKey.get(blockStateKey(name, props));
    if (!model) return '—';
    switch (model.status) {
      case '3d-json-elements':
        return '3D JSON';
      case 'unsupported-obj':
        return locale === 'ru' ? 'OBJ (нет 3D)' : 'OBJ (no 3D)';
      case 'procedural-or-blockentity':
        return locale === 'ru' ? 'Процедурная' : 'Procedural';
      case '2d-item-generated':
        return '2D';
      default:
        return name.startsWith('minecraft:')
          ? (locale === 'ru' ? 'Ванильный (бокс)' : 'Vanilla (box)')
          : (locale === 'ru' ? 'Нет файла' : 'Missing');
    }
  }

  const filteredPalette = $derived.by(() => {
    const s = schematic;
    if (!s) return [];
    const q = paletteSearch.trim().toLowerCase();
    return q ? s.palette.filter((p) => p.name.toLowerCase().includes(q)) : s.palette;
  });

  const modelStatsSummary = $derived.by(() => {
    let real3d = 0;
    let fallback = 0;
    for (const m of resolvedByStateKey.values()) {
      if (m.status === '3d-json-elements') real3d++;
      else fallback++;
    }
    return { real3d, fallback };
  });
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="NBT Schematic Viewer">
  <div class="modal-card modal-xl">
    <div class="modal-header">
      <div>
        <h3>{locale === 'ru' ? '3D-просмотрщик Create-схематик (.nbt)' : '3D Create Schematic Viewer (.nbt)'}</h3>
        <div class="modal-sub">
          {locale === 'ru'
            ? 'Читает StructureTemplate .nbt (size, palette, blocks, DataVersion) и рендерит 3D-модели блоков с текстурами. .litematic и .schem отклоняются с пояснением.'
            : 'Reads StructureTemplate .nbt and renders 3D textured block models from blockstates & JSON elements.'}
        </div>
      </div>
      <button type="button" class="btn-ghost" onclick={onClose}>✕</button>
    </div>

    <div class="modal-toolbar">
      <label class="btn-primary file-btn">
        {locale === 'ru' ? 'Открыть .nbt файл' : 'Open .nbt file'}
        <input type="file" accept=".nbt,.litematic,.schem" onchange={handleFileInput} hidden />
      </label>
      <button type="button" class="btn-secondary" onclick={loadSyntheticDemo}>
        {locale === 'ru' ? 'Синтетический тест (.nbt)' : 'Synthetic test (.nbt)'}
      </button>

      {#if schematic}
        <div class="viewer-controls">
          <button type="button" class="btn-secondary btn-xs" onclick={() => (yawDeg = (yawDeg + 90) % 360)}>
            ↻ 90° ({Math.round(yawDeg)}°)
          </button>
          <button type="button" class="btn-secondary btn-xs" onclick={resetCamera}>
            ↺
          </button>
          <button type="button" class="btn-secondary btn-xs" onclick={() => (zoomFactor = Math.max(0.35, zoomFactor * 0.85))}>
            −
          </button>
          <span class="zoom-lbl">{Math.round(zoomFactor * 100)}%</span>
          <button type="button" class="btn-secondary btn-xs" onclick={() => (zoomFactor = Math.min(4, zoomFactor * 1.15))}>
            +
          </button>
          <label class="slice-ctrl">
            <span>Y ≤ {maxLayerY}</span>
            <input
              type="range"
              min="0"
              max={Math.max(0, schematic.size[1] - 1)}
              bind:value={maxLayerY}
            />
          </label>
        </div>
      {/if}
    </div>

    {#if errorMsg}
      <div class="error-banner">{errorMsg}</div>
    {/if}

    <div class="viewer-split">
      <div class="canvas-pane">
        {#if schematic}
          <div class="canvas-holder">
            <canvas
              bind:this={canvasEl}
              width="680"
              height="430"
              class="iso-canvas"
              oncontextmenu={(e) => e.preventDefault()}
              onpointerdown={(e) => {
                dragMode = e.shiftKey || e.button === 2 || e.button === 1 ? 'pan' : 'orbit';
                dragStart = {
                  x: e.clientX,
                  y: e.clientY,
                  yaw: yawDeg,
                  pitch: pitchDeg,
                  ox: panX,
                  oy: panY,
                };
                (e.currentTarget as HTMLCanvasElement).setPointerCapture(e.pointerId);
              }}
              onpointermove={(e) => {
                if (!dragMode) return;
                if (dragMode === 'orbit') {
                  yawDeg = dragStart.yaw + (e.clientX - dragStart.x) * 0.65;
                  pitchDeg = Math.max(-85, Math.min(85, dragStart.pitch + (e.clientY - dragStart.y) * 0.55));
                } else {
                  panX = dragStart.ox + (e.clientX - dragStart.x);
                  panY = dragStart.oy + (e.clientY - dragStart.y);
                }
              }}
              onpointerup={() => (dragMode = null)}
              onwheel={(e) => {
                e.preventDefault();
                zoomFactor = Math.min(4, Math.max(0.35, zoomFactor * (e.deltaY < 0 ? 1.12 : 0.9)));
              }}
            ></canvas>
            {#if loading3D}
              <div class="loading-pill">
                {locale === 'ru' ? 'Загрузка 3D-моделей...' : 'Loading 3D models...'}
              </div>
            {/if}
          </div>
          <div class="controls-hint">
            {locale === 'ru'
              ? `ЛКМ — вращение 3D | Shift+ЛКМ / ПКМ — сдвиг | Колесо — зум | Вариантов 3D JSON: ${modelStatsSummary.real3d}, заглушек: ${modelStatsSummary.fallback}, граней: ${lastRenderStats.renderedFaces}`
              : `Drag: 3D orbit | Shift+Drag: pan | Wheel: zoom | 3D JSON states: ${modelStatsSummary.real3d}, fallback boxes: ${modelStatsSummary.fallback}, faces: ${lastRenderStats.renderedFaces}`}
          </div>
          {#if schematic.warnings.length > 0}
            <div class="synth-note">{schematic.warnings.join(' ')}</div>
          {/if}
        {:else}
          <div class="empty-viewer">
            {locale === 'ru'
              ? 'Откройте локальный .nbt файл из папки schematics/ для 3D-просмотра по blockstates и моделям блоков.'
              : 'Open a local .nbt schematic file to view 3D textured block models and inspect its BOM.'}
          </div>
        {/if}
      </div>

      <div class="palette-pane">
        {#if schematic}
          <div class="meta-grid">
            <div><span>{locale === 'ru' ? 'Габариты' : 'Size'}:</span> <strong>{schematic.size.join(' × ')}</strong></div>
            <div><span>DataVersion:</span> <strong>{schematic.dataVersion ?? '—'}</strong></div>
            <div><span>{locale === 'ru' ? 'Блоков' : 'Blocks'}:</span> <strong>{schematic.nonAirBlocks}</strong></div>
            <div><span>{locale === 'ru' ? 'Сущностей' : 'Entities'}:</span> <strong>{schematic.entityCount}</strong></div>
          </div>

          <div class="pal-head">
            <input
              type="search"
              class="pal-search"
              bind:value={paletteSearch}
              placeholder={locale === 'ru' ? 'Фильтр палитры...' : 'Filter palette...'}
            />
            <button type="button" class="btn-secondary btn-xs" onclick={exportSchematicBomCsv}>
              CSV
            </button>
          </div>

          <div class="pal-list">
            {#each filteredPalette as item (item.index + item.name)}
              {@const renderTag = statusForPaletteItem(item.name, item.properties)}
              <div class="pal-row">
                <PixelSprite asset={assetsById.get(item.name)} size={20} />
                <div class="pal-name" title={item.name}>{item.name}</div>
                <span class={`pal-tag ${renderTag === '3D JSON' ? 'ok' : ''}`}>{renderTag}</span>
                <div class="pal-count">×{item.count}</div>
              </div>
            {/each}
          </div>
        {:else}
          <div class="empty-viewer small">
            {locale === 'ru' ? 'После открытия файла здесь появится палитра и статус 3D-моделей.' : 'Palette and 3D model statuses will appear here.'}
          </div>
        {/if}
      </div>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 23, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 20px;
  }
  .modal-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    width: min(1080px, 96vw);
    max-height: 92vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    box-shadow: 0 24px 60px rgba(0, 0, 0, 0.5);
  }
  .modal-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .modal-header h3 { margin: 0 0 4px; font-size: 16px; }
  .modal-sub { font-size: 12px; color: var(--text-muted); }

  .modal-toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 18px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    flex-wrap: wrap;
  }
  .file-btn { cursor: pointer; display: inline-flex; align-items: center; }

  .viewer-controls {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
  }
  .zoom-lbl { font-size: 12px; font-weight: 600; min-width: 42px; text-align: center; }
  .slice-ctrl {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
  }

  .error-banner {
    margin: 12px 18px 0;
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(239, 68, 68, 0.16);
    border: 1px solid #ef4444;
    color: #fecaca;
    font-size: 13px;
  }

  .viewer-split {
    display: grid;
    grid-template-columns: 1.5fr 1fr;
    gap: 0;
    flex: 1;
    min-height: 450px;
    overflow: hidden;
  }
  .canvas-pane {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    border-right: 1px solid var(--border-subtle);
  }
  .canvas-holder {
    position: relative;
  }
  .iso-canvas {
    width: 100%;
    height: 410px;
    border-radius: 8px;
    border: 1px solid var(--border-subtle);
    cursor: grab;
    display: block;
  }
  .loading-pill {
    position: absolute;
    left: 10px;
    top: 10px;
    background: rgba(15, 23, 42, 0.85);
    color: #f8fafc;
    font-size: 11px;
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid var(--border);
  }
  .controls-hint {
    font-size: 11px;
    color: var(--text-muted);
  }
  .synth-note {
    font-size: 11.5px;
    color: #7dd3fc;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.35);
    padding: 6px 10px;
    border-radius: 7px;
  }

  .palette-pane {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    overflow: hidden;
  }
  .meta-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    font-size: 12px;
    background: var(--bg-elevated);
    padding: 8px 10px;
    border-radius: 8px;
  }
  .meta-grid span { color: var(--text-muted); }

  .pal-head {
    display: flex;
    gap: 6px;
  }
  .pal-search {
    flex: 1;
    padding: 6px 9px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12px;
  }
  .pal-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .pal-row {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border-radius: 6px;
    background: var(--bg-elevated);
    font-size: 12px;
  }
  .pal-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pal-tag {
    font-size: 9.5px;
    padding: 1px 5px;
    border-radius: 4px;
    background: rgba(148, 163, 184, 0.16);
    color: var(--text-muted);
    white-space: nowrap;
  }
  .pal-tag.ok {
    background: rgba(16, 185, 129, 0.18);
    color: #34d399;
    font-weight: 700;
  }
  .pal-count { font-weight: 700; color: var(--accent-text); }

  .empty-viewer {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    text-align: center;
    color: var(--text-muted);
    padding: 24px;
    font-size: 13px;
  }
</style>
