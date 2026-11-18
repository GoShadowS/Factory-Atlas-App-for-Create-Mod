<script lang="ts">
  import { untrack } from 'svelte';
  import {
    createBrowserAssetFetcher,
    McModelResolver,
    type RawMcModelJson,
    type ResolvedMcModel,
  } from '../models/mcModelResolver';
  import { renderBlocksScene3D, type Camera3D } from '../models/renderer3d';
  import { loadTextureImageCached, preloadTexturesForPaths } from '../models/textureCache';
  import type { Locale, ModelSupportStatus, ResourceVisualAssignment } from '../types/atlas';

  interface Props {
    locale: Locale;
    visual?: ResourceVisualAssignment;
    fallbackUrl?: string;
    size?: number;
    compact?: boolean;
  }

  let {
    locale,
    visual,
    fallbackUrl = '/assets-fallback/item-fallback.svg',
    size = 180,
    compact = false,
  }: Props = $props();

  let canvasEl: HTMLCanvasElement | undefined = $state();
  let resolved = $state<ResolvedMcModel | null>(null);
  let loadedTextures = $state<Map<string, HTMLImageElement>>(new Map());
  let customImg = $state<HTMLImageElement | null>(null);
  let custom2dFallbackImg = $state<HTMLImageElement | null>(null);
  let loading = $state(false);

  let camera = $state<Camera3D>({
    yawDeg: 35,
    pitchDeg: 26,
    zoom: untrack(() => size) * 0.56,
    panX: 0,
    panY: 0,
  });

  let dragging = false;
  let dragStart = { x: 0, y: 0, yaw: 35, pitch: 26 };

  function resetCamera() {
    camera = {
      yawDeg: 35,
      pitchDeg: 26,
      zoom: size * 0.56,
      panX: 0,
      panY: 0,
    };
  }

  // Load resolved 3D model + referenced textures lazily whenever visual assignment changes
  $effect(() => {
    const v = visual;
    let cancelled = false;

    if (!v || v.disabled) {
      resolved = null;
      customImg = null;
      custom2dFallbackImg = null;
      loadedTextures = new Map();
      return;
    }

    loading = true;

    (async () => {
      try {
        let customOverrideJson: RawMcModelJson | undefined;
        let modelRef = v.modelRef || '';

        if (v.customModelJson) {
          try {
            customOverrideJson = JSON.parse(v.customModelJson) as RawMcModelJson;
            modelRef = modelRef || 'custom:inline_model';
          } catch {
            // Invalid custom model JSON handled cleanly
          }
        }

        const overrideMap = new Map<string, RawMcModelJson>();
        if (customOverrideJson && modelRef) {
          overrideMap.set(modelRef.includes(':') ? modelRef : `custom:${modelRef}`, customOverrideJson);
        }

        const resolver = new McModelResolver(createBrowserAssetFetcher(overrideMap));
        let res: ResolvedMcModel | null = null;
        if (modelRef) {
          res = await resolver.resolveModel(modelRef, {
            inlineOverrideJson: customOverrideJson,
          });
        }

        if (cancelled) return;
        resolved = res;

        const pathsToLoad = new Set<string>(res?.texturePaths ?? []);
        if (v.texturePath) pathsToLoad.add(v.texturePath);

        const texMap = await preloadTexturesForPaths(Array.from(pathsToLoad));
        if (cancelled) return;
        loadedTextures = texMap;

        if (v.customTextureOverrideDataUrl) {
          customImg = await loadTextureImageCached(v.customTextureOverrideDataUrl);
        } else {
          customImg = null;
        }

        if (v.texturePath && texMap.has(v.texturePath)) {
          custom2dFallbackImg = texMap.get(v.texturePath) ?? null;
        } else {
          custom2dFallbackImg = null;
        }
      } finally {
        if (!cancelled) loading = false;
      }
    })();

    return () => {
      cancelled = true;
    };
  });

  // Draw 3D cuboids or honest 2D fallback whenever state or camera updates
  $effect(() => {
    const cvs = canvasEl;
    if (!cvs) return;
    const ctx = cvs.getContext('2d');
    if (!ctx) return;

    const w = cvs.width;
    const h = cvs.height;
    ctx.clearRect(0, 0, w, h);

    // Subtle dark/light backdrop gradient
    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    if (!visual || visual.disabled) {
      drawCenteredImageOrHint(ctx, w, h, fallbackUrl);
      return;
    }

    if (resolved && resolved.status === '3d-json-elements' && resolved.cuboids.length > 0) {
      renderBlocksScene3D({
        ctx,
        width: w,
        height: h,
        blocks: [
          {
            bx: 0,
            by: 0,
            bz: 0,
            cuboids: resolved.cuboids,
            placeholderColor: '#64748b',
          },
        ],
        centerWorld: { x: 0.5, y: 0.5, z: 0.5 },
        camera,
        texturesByPath: loadedTextures,
        customDataUrlImg: customImg,
      });
      return;
    }

    // Honest 2D fallback (never fake 3D extrusion from a flat PNG!)
    const flatImg = customImg || custom2dFallbackImg;
    if (flatImg && flatImg.complete && flatImg.naturalWidth > 0) {
      const iw = flatImg.naturalWidth;
      const ih = flatImg.naturalHeight;
      const frameH = ih > iw * 2 && ih % iw === 0 ? iw : ih;
      const pad = Math.round(Math.min(w, h) * 0.18);
      const box = Math.min(w, h) - pad * 2;
      ctx.imageSmoothingEnabled = false;
      ctx.drawImage(flatImg, 0, 0, iw, frameH, (w - box) / 2, (h - box) / 2, box, box);
    } else {
      drawCenteredImageOrHint(ctx, w, h, fallbackUrl);
    }
  });

  function drawCenteredImageOrHint(ctx: CanvasRenderingContext2D, w: number, h: number, svgUrl: string) {
    const img = new Image();
    img.src = svgUrl;
    img.onload = () => {
      const box = Math.min(w, h) * 0.55;
      ctx.drawImage(img, (w - box) / 2, (h - box) / 2, box, box);
    };
  }

  const statusBadge = $derived.by(() => {
    const st: ModelSupportStatus = resolved?.status ?? (visual && !visual.disabled ? visual.modelSupport : 'none');
    switch (st) {
      case '3d-json-elements':
        return {
          cls: 'badge-ok',
          label: locale === 'ru' ? '3D JSON модель' : '3D JSON model',
        };
      case '2d-item-generated':
        return {
          cls: 'badge-info',
          label: locale === 'ru' ? 'Только 2D-иконка (не 3D)' : '2D item sprite only (not 3D)',
        };
      case 'unsupported-obj':
        return {
          cls: 'badge-warn',
          label: locale === 'ru' ? 'OBJ не поддерживается' : 'OBJ unsupported',
        };
      case 'procedural-or-blockentity':
        return {
          cls: 'badge-warn',
          label: locale === 'ru' ? 'Процедурная / BlockEntity (нет 3D JSON)' : 'Procedural / BlockEntity (no static 3D)',
        };
      default:
        return {
          cls: 'badge-muted',
          label: locale === 'ru' ? 'Нет 3D-модели / заглушка' : 'No 3D model / placeholder',
        };
    }
  });
</script>

<div class="mv-wrap" class:compact>
  <div class="mv-stage" style={`width:${size}px;height:${size}px;`}>
    <canvas
      bind:this={canvasEl}
      width={size}
      height={size}
      class="mv-canvas"
      aria-label="3D model preview"
      onpointerdown={(e) => {
        dragging = true;
        dragStart = { x: e.clientX, y: e.clientY, yaw: camera.yawDeg, pitch: camera.pitchDeg };
        (e.currentTarget as HTMLCanvasElement).setPointerCapture(e.pointerId);
      }}
      onpointermove={(e) => {
        if (!dragging) return;
        camera = {
          ...camera,
          yawDeg: dragStart.yaw + (e.clientX - dragStart.x) * 0.85,
          pitchDeg: Math.max(-80, Math.min(80, dragStart.pitch + (e.clientY - dragStart.y) * 0.7)),
        };
      }}
      onpointerup={() => {
        dragging = false;
      }}
      onwheel={(e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 1.12 : 0.9;
        camera = {
          ...camera,
          zoom: Math.max(size * 0.22, Math.min(size * 1.4, camera.zoom * factor)),
        };
      }}
    ></canvas>

    {#if loading}
      <div class="mv-loading">{locale === 'ru' ? 'Загрузка...' : 'Loading...'}</div>
    {/if}

    {#if (resolved?.status ?? visual?.modelSupport) === '3d-json-elements'}
      <button
        type="button"
        class="mv-reset"
        title={locale === 'ru' ? 'Сбросить ракурс' : 'Reset camera'}
        onclick={resetCamera}
      >
        ↺
      </button>
    {/if}
  </div>

  <div class="mv-meta">
    <span class={`mv-badge ${statusBadge.cls}`}>{statusBadge.label}</span>
    {#if !compact && resolved?.reasonRu}
      <div class="mv-reason">
        {locale === 'ru' ? resolved.reasonRu : resolved.reasonEn}
      </div>
    {:else if !compact && visual?.disabled}
      <div class="mv-reason">
        {locale === 'ru' ? 'Визуал отключён пользователем (заглушка).' : 'Visual disabled by user (placeholder).'}
      </div>
    {/if}
  </div>
</div>

<style>
  .mv-wrap {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .mv-stage {
    position: relative;
    border-radius: 10px;
    overflow: hidden;
    border: 1px solid var(--border);
    background: #0f172a;
    user-select: none;
  }

  .mv-canvas {
    display: block;
    width: 100%;
    height: 100%;
    cursor: grab;
  }
  .mv-canvas:active {
    cursor: grabbing;
  }

  .mv-loading {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgba(15, 23, 42, 0.72);
    color: #e2e8f0;
    font-size: 11px;
  }

  .mv-reset {
    position: absolute;
    right: 6px;
    top: 6px;
    width: 24px;
    height: 24px;
    border-radius: 6px;
    border: 1px solid rgba(148, 163, 184, 0.35);
    background: rgba(15, 23, 42, 0.8);
    color: #e2e8f0;
    font-size: 13px;
    cursor: pointer;
    display: grid;
    place-items: center;
  }

  .mv-meta {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    text-align: center;
    max-width: 240px;
  }

  .mv-badge {
    font-size: 10.5px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: 5px;
  }
  .badge-ok {
    background: rgba(16, 185, 129, 0.16);
    color: #34d399;
    border: 1px solid rgba(16, 185, 129, 0.35);
  }
  .badge-info {
    background: rgba(56, 189, 248, 0.16);
    color: #7dd3fc;
    border: 1px solid rgba(56, 189, 248, 0.35);
  }
  .badge-warn {
    background: rgba(245, 158, 11, 0.18);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.35);
  }
  .badge-muted {
    background: rgba(148, 163, 184, 0.14);
    color: var(--text-muted);
    border: 1px solid var(--border);
  }

  .mv-reason {
    font-size: 11px;
    color: var(--text-muted);
    line-height: 1.3;
  }
</style>
