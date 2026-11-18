import type { ModelSupportStatus } from '../types/atlas';

export type FaceDir = 'north' | 'south' | 'east' | 'west' | 'up' | 'down';

export interface RawMcFace {
  uv?: [number, number, number, number];
  texture: string;
  rotation?: number;
  tintindex?: number;
}

export interface RawMcElement {
  from: [number, number, number];
  to: [number, number, number];
  rotation?: {
    origin: [number, number, number];
    axis: 'x' | 'y' | 'z';
    angle: number;
    rescale?: boolean;
  };
  faces?: Partial<Record<FaceDir, RawMcFace>>;
}

export interface RawMcModelJson {
  parent?: string;
  loader?: string;
  textures?: Record<string, string>;
  elements?: RawMcElement[];
  display?: Record<string, { rotation?: number[]; translation?: number[]; scale?: number[] }>;
}

export interface ResolvedCuboidFace {
  dir: FaceDir;
  texturePath: string | null; // e.g. 'create/textures/block/cogwheel.png' or '#custom'
  textureRef: string | null; // raw normalized e.g. 'create:block/cogwheel'
  uv: [number, number, number, number]; // 0..16 space
  rotation: number; // 0, 90, 180, 270
}

export interface ResolvedCuboid {
  from: [number, number, number];
  to: [number, number, number];
  elementRotation?: {
    origin: [number, number, number];
    axis: 'x' | 'y' | 'z';
    angle: number;
  };
  blockRotationX: number; // 0, 90, 180, 270 (from blockstate)
  blockRotationY: number; // 0, 90, 180, 270 (from blockstate)
  faces: Partial<Record<FaceDir, ResolvedCuboidFace>>;
}

export interface ResolvedMcModel {
  modelRef: string;
  status: ModelSupportStatus;
  reasonRu: string;
  reasonEn: string;
  cuboids: ResolvedCuboid[];
  texturePaths: string[];
  primaryTexturePath: string | null;
  parentChain: string[];
}

export interface McJsonFetcher {
  fetchModelJson: (modelRef: string) => Promise<RawMcModelJson | null>;
  fetchBlockstateJson?: (blockId: string) => Promise<Record<string, unknown> | null>;
}

// Built-in minimal vanilla primitives so Create/addon blocks extending cube_all/cube_column
// resolve into genuine 6-face 3D geometry without needing vanilla JAR assets.
export const VANILLA_PRIMITIVE_MODELS: Record<string, RawMcModelJson> = {
  'minecraft:block/block': {},
  'minecraft:block/cube': {
    elements: [
      {
        from: [0, 0, 0],
        to: [16, 16, 16],
        faces: {
          down: { texture: '#down', uv: [0, 0, 16, 16] },
          up: { texture: '#up', uv: [0, 0, 16, 16] },
          north: { texture: '#north', uv: [0, 0, 16, 16] },
          south: { texture: '#south', uv: [0, 0, 16, 16] },
          west: { texture: '#west', uv: [0, 0, 16, 16] },
          east: { texture: '#east', uv: [0, 0, 16, 16] },
        },
      },
    ],
  },
  'minecraft:block/cube_all': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#all',
      up: '#all',
      north: '#all',
      south: '#all',
      west: '#all',
      east: '#all',
    },
  },
  'minecraft:block/cube_bottom_top': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#bottom',
      up: '#top',
      north: '#side',
      south: '#side',
      west: '#side',
      east: '#side',
    },
  },
  'minecraft:block/cube_column': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#end',
      up: '#end',
      north: '#side',
      south: '#side',
      west: '#side',
      east: '#side',
    },
  },
  'minecraft:block/cube_column_horizontal': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#side',
      up: '#side',
      north: '#end',
      south: '#end',
      west: '#side',
      east: '#side',
    },
  },
  'minecraft:block/orientable': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#side',
      up: '#top',
      north: '#front',
      south: '#side',
      west: '#side',
      east: '#side',
    },
  },
  'minecraft:block/orientable_with_bottom': {
    parent: 'minecraft:block/cube',
    textures: {
      down: '#bottom',
      up: '#top',
      north: '#front',
      south: '#side',
      west: '#side',
      east: '#side',
    },
  },
  'minecraft:block/slab': {
    elements: [
      {
        from: [0, 0, 0],
        to: [16, 8, 16],
        faces: {
          down: { texture: '#bottom', uv: [0, 0, 16, 16] },
          up: { texture: '#top', uv: [0, 0, 16, 16] },
          north: { texture: '#side', uv: [0, 8, 16, 16] },
          south: { texture: '#side', uv: [0, 8, 16, 16] },
          west: { texture: '#side', uv: [0, 8, 16, 16] },
          east: { texture: '#side', uv: [0, 8, 16, 16] },
        },
      },
    ],
  },
  'minecraft:block/slab_top': {
    elements: [
      {
        from: [0, 8, 0],
        to: [16, 16, 16],
        faces: {
          down: { texture: '#bottom', uv: [0, 0, 16, 16] },
          up: { texture: '#top', uv: [0, 0, 16, 16] },
          north: { texture: '#side', uv: [0, 0, 16, 8] },
          south: { texture: '#side', uv: [0, 0, 16, 8] },
          west: { texture: '#side', uv: [0, 0, 16, 8] },
          east: { texture: '#side', uv: [0, 0, 16, 8] },
        },
      },
    ],
  },
  'minecraft:block/carpet': {
    elements: [
      {
        from: [0, 0, 0],
        to: [16, 1, 16],
        faces: {
          down: { texture: '#wool', uv: [0, 0, 16, 16] },
          up: { texture: '#wool', uv: [0, 0, 16, 16] },
          north: { texture: '#wool', uv: [0, 15, 16, 16] },
          south: { texture: '#wool', uv: [0, 15, 16, 16] },
          west: { texture: '#wool', uv: [0, 15, 16, 16] },
          east: { texture: '#wool', uv: [0, 15, 16, 16] },
        },
      },
    ],
  },
};

const ITEM_2D_PARENTS = new Set([
  'minecraft:item/generated',
  'minecraft:item/handheld',
  'minecraft:item/handheld_rod',
  'minecraft:builtin/generated',
]);

export function normalizeModelRef(ref: string): string {
  const trimmed = ref.trim();
  if (!trimmed) return '';
  return trimmed.includes(':') ? trimmed : `minecraft:${trimmed}`;
}

export function modelRefToLocalPath(modelRef: string): string | null {
  const norm = normalizeModelRef(modelRef);
  const [ns, sub] = norm.split(':', 2);
  if (!ns || !sub || ns === 'minecraft') return null;
  return `${ns}/models/${sub}.json`;
}

export function blockIdToBlockstatePath(blockId: string): string | null {
  const norm = normalizeModelRef(blockId);
  const [ns, sub] = norm.split(':', 2);
  if (!ns || !sub || ns === 'minecraft') return null;
  return `${ns}/blockstates/${sub}.json`;
}

export function textureRefToLocalPngPath(texRef: string): string | null {
  if (!texRef || texRef.startsWith('#')) return null;
  const norm = normalizeModelRef(texRef);
  const [ns, sub] = norm.split(':', 2);
  if (!ns || !sub || ns === 'minecraft') return null;
  return `${ns}/textures/${sub}.png`;
}

/**
 * Resolves `#variable` chains safely against cyclic `#a -> #b -> #a`.
 */
export function resolveTextureVariable(
  texRef: string,
  texturesMap: Record<string, string>
): string | null {
  let cur: string | undefined = texRef;
  const visited = new Set<string>();
  while (cur && cur.startsWith('#')) {
    const key = cur.slice(1);
    if (visited.has(key)) return null; // Cycle detected
    visited.add(key);
    cur = texturesMap[key];
  }
  return cur ? normalizeModelRef(cur) : null;
}

export function computeDefaultFaceUv(
  from: [number, number, number],
  to: [number, number, number],
  dir: FaceDir
): [number, number, number, number] {
  const [x1, y1, z1] = from;
  const [x2, y2, z2] = to;
  switch (dir) {
    case 'north':
      return [16 - x2, 16 - y2, 16 - x1, 16 - y1];
    case 'south':
      return [x1, 16 - y2, x2, 16 - y1];
    case 'west':
      return [z1, 16 - y2, z2, 16 - y1];
    case 'east':
      return [16 - z2, 16 - y2, 16 - z1, 16 - y1];
    case 'up':
      return [x1, z1, x2, z2];
    case 'down':
      return [x1, 16 - z2, x2, 16 - z1];
  }
}

/**
 * Bounded LRU cache to ensure memory stays capped on large schematics or catalogs.
 */
export class BoundedLruCache<K, V> {
  private map = new Map<K, V>();
  constructor(private readonly maxSize: number) {}

  get(key: K): V | undefined {
    if (!this.map.has(key)) return undefined;
    const val = this.map.get(key)!;
    this.map.delete(key);
    this.map.set(key, val);
    return val;
  }

  set(key: K, val: V): void {
    if (this.map.has(key)) this.map.delete(key);
    else if (this.map.size >= this.maxSize) {
      const oldest = this.map.keys().next().value;
      if (oldest !== undefined) this.map.delete(oldest);
    }
    this.map.set(key, val);
  }

  clear(): void {
    this.map.clear();
  }

  get size(): number {
    return this.map.size;
  }
}

export interface BlockstateAppliedPart {
  model: string;
  x: number;
  y: number;
  uvlock: boolean;
}

function matchConditionMap(
  cond: Record<string, unknown>,
  props: Record<string, string>
): boolean {
  for (const [k, rawVal] of Object.entries(cond)) {
    if (k === 'OR' && Array.isArray(rawVal)) {
      if (!rawVal.some((sub) => sub && typeof sub === 'object' && matchConditionMap(sub as Record<string, unknown>, props))) {
        return false;
      }
      continue;
    }
    if (k === 'AND' && Array.isArray(rawVal)) {
      if (!rawVal.every((sub) => sub && typeof sub === 'object' && matchConditionMap(sub as Record<string, unknown>, props))) {
        return false;
      }
      continue;
    }
    const actual = props[k];
    if (actual === undefined) return false;
    const allowed = String(rawVal).split('|');
    if (!allowed.includes(actual)) return false;
  }
  return true;
}

/**
 * Evaluates both `"variants"` and `"multipart"` Minecraft blockstates JSON.
 */
export function selectBlockstateModels(
  bs: Record<string, unknown>,
  properties: Record<string, string> = {}
): BlockstateAppliedPart[] {
  const out: BlockstateAppliedPart[] = [];

  if (bs.variants && typeof bs.variants === 'object' && !Array.isArray(bs.variants)) {
    const variants = bs.variants as Record<string, unknown>;
    const entries = Object.entries(variants);

    let chosenRaw: unknown = variants[''];
    if (chosenRaw === undefined) {
      // Score each variant key by how many properties match
      let bestScore = -1;
      for (const [k, val] of entries) {
        const pairs = k
          .split(',')
          .map((p) => p.trim())
          .filter(Boolean);
        let allMatched = true;
        let score = 0;
        for (const pair of pairs) {
          const [pk, pv] = pair.split('=', 2);
          if (properties[pk] === pv) {
            score += 2;
          } else if (properties[pk] !== undefined) {
            allMatched = false;
            break;
          } else {
            score += 0; // Unspecified property: keep as fallback candidate
          }
        }
        if (allMatched && score > bestScore) {
          bestScore = score;
          chosenRaw = val;
        }
      }
      if (chosenRaw === undefined && entries.length > 0) {
        chosenRaw = entries[0][1];
      }
    }

    const picked = Array.isArray(chosenRaw) ? chosenRaw[0] : chosenRaw;
    if (picked && typeof picked === 'object') {
      const p = picked as Record<string, unknown>;
      if (typeof p.model === 'string') {
        out.push({
          model: p.model,
          x: typeof p.x === 'number' ? p.x : 0,
          y: typeof p.y === 'number' ? p.y : 0,
          uvlock: Boolean(p.uvlock),
        });
      }
    }
    return out;
  }

  if (Array.isArray(bs.multipart)) {
    for (const part of bs.multipart) {
      if (!part || typeof part !== 'object') continue;
      const p = part as Record<string, unknown>;
      const when = p.when as Record<string, unknown> | undefined;
      if (when && !matchConditionMap(when, properties)) continue;
      const applyRaw = Array.isArray(p.apply) ? p.apply[0] : p.apply;
      if (applyRaw && typeof applyRaw === 'object') {
        const ap = applyRaw as Record<string, unknown>;
        if (typeof ap.model === 'string') {
          out.push({
            model: ap.model,
            x: typeof ap.x === 'number' ? ap.x : 0,
            y: typeof ap.y === 'number' ? ap.y : 0,
            uvlock: Boolean(ap.uvlock),
          });
        }
      }
    }
  }

  return out;
}

/**
 * Resolves a full Minecraft JSON model (including parent inheritance, #texture variables,
 * cycle protection, and blockstate x/y rotations) using bounded LRU caching.
 */
export class McModelResolver {
  private modelCache = new BoundedLruCache<string, ResolvedMcModel>(160);
  private rawModelCache = new BoundedLruCache<string, RawMcModelJson | null>(240);
  private blockstateCache = new BoundedLruCache<string, Record<string, unknown> | null>(160);

  constructor(private readonly fetcher: McJsonFetcher) {}

  clearCaches(): void {
    this.modelCache.clear();
    this.rawModelCache.clear();
    this.blockstateCache.clear();
  }

  private async getRawModel(modelRef: string): Promise<RawMcModelJson | null> {
    const norm = normalizeModelRef(modelRef);
    if (norm in VANILLA_PRIMITIVE_MODELS) {
      return VANILLA_PRIMITIVE_MODELS[norm];
    }
    const cached = this.rawModelCache.get(norm);
    if (cached !== undefined) return cached;
    const loaded = await this.fetcher.fetchModelJson(norm);
    this.rawModelCache.set(norm, loaded);
    return loaded;
  }

  private async getBlockstate(blockId: string): Promise<Record<string, unknown> | null> {
    const norm = normalizeModelRef(blockId);
    const cached = this.blockstateCache.get(norm);
    if (cached !== undefined) return cached;
    const loaded = this.fetcher.fetchBlockstateJson
      ? await this.fetcher.fetchBlockstateJson(norm)
      : null;
    this.blockstateCache.set(norm, loaded);
    return loaded;
  }

  async resolveModel(
    modelRef: string,
    opts?: { blockRotationX?: number; blockRotationY?: number; inlineOverrideJson?: RawMcModelJson }
  ): Promise<ResolvedMcModel> {
    const normRef = normalizeModelRef(modelRef);
    const rotX = opts?.blockRotationX ?? 0;
    const rotY = opts?.blockRotationY ?? 0;
    const cacheKey = `${normRef}@${rotX}:${rotY}`;

    if (!opts?.inlineOverrideJson) {
      const hit = this.modelCache.get(cacheKey);
      if (hit) return hit;
    }

    const chain: RawMcModelJson[] = [];
    const parentChain: string[] = [];
    const visited = new Set<string>();
    let cycleDetected = false;
    let is2dItem = false;
    let isObj = false;
    let isBuiltinEntity = false;
    let missingFile = false;

    let curRef: string | undefined = normRef;
    while (curRef && parentChain.length < 16) {
      const nRef = normalizeModelRef(curRef);
      if (visited.has(nRef)) {
        cycleDetected = true;
        break;
      }
      visited.add(nRef);
      parentChain.push(nRef);

      if (ITEM_2D_PARENTS.has(nRef)) {
        is2dItem = true;
        break;
      }
      if (nRef === 'minecraft:builtin/entity') {
        isBuiltinEntity = true;
        break;
      }

      const raw =
        parentChain.length === 1 && opts?.inlineOverrideJson
          ? opts.inlineOverrideJson
          : await this.getRawModel(nRef);

      if (!raw) {
        if (!(nRef in VANILLA_PRIMITIVE_MODELS)) {
          missingFile = true;
        }
        break;
      }
      if (raw.loader === 'neoforge:obj') {
        isObj = true;
      }
      chain.push(raw);
      curRef = raw.parent;
    }

    // Merge textures root-to-child (child overrides parent)
    const texturesMap: Record<string, string> = {};
    let chosenElements: RawMcElement[] | undefined;

    for (let i = chain.length - 1; i >= 0; i--) {
      const layer = chain[i];
      if (layer.textures) {
        for (const [k, v] of Object.entries(layer.textures)) {
          if (typeof v === 'string') texturesMap[k] = v;
        }
      }
      if (Array.isArray(layer.elements) && layer.elements.length > 0) {
        chosenElements = layer.elements;
      }
    }

    // Resolve all texture paths referenced
    const texSet = new Set<string>();
    const cuboids: ResolvedCuboid[] = [];

    if (chosenElements && !isObj) {
      for (const el of chosenElements) {
        if (!Array.isArray(el.from) || !Array.isArray(el.to)) continue;
        const faces: Partial<Record<FaceDir, ResolvedCuboidFace>> = {};
        if (el.faces) {
          for (const dir of ['north', 'south', 'east', 'west', 'up', 'down'] as FaceDir[]) {
            const rf = el.faces[dir];
            if (!rf) continue;
            const resolvedRef = resolveTextureVariable(rf.texture, texturesMap);
            const pngPath = resolvedRef ? textureRefToLocalPngPath(resolvedRef) : null;
            if (pngPath) texSet.add(pngPath);
            faces[dir] = {
              dir,
              texturePath: pngPath,
              textureRef: resolvedRef,
              uv: rf.uv ?? computeDefaultFaceUv(el.from, el.to, dir),
              rotation: rf.rotation ?? 0,
            };
          }
        }
        cuboids.push({
          from: el.from,
          to: el.to,
          elementRotation: el.rotation
            ? {
                origin: el.rotation.origin,
                axis: el.rotation.axis,
                angle: el.rotation.angle,
              }
            : undefined,
          blockRotationX: rotX,
          blockRotationY: rotY,
          faces,
        });
      }
    }

    // Also include layer0 / particle for 2D items
    for (const k of ['layer0', 'all', 'particle', 'side', 'front']) {
      if (texturesMap[k]) {
        const r = resolveTextureVariable(texturesMap[k], texturesMap);
        const p = r ? textureRefToLocalPngPath(r) : null;
        if (p) texSet.add(p);
      }
    }

    const texturePaths = Array.from(texSet);
    const primaryTexturePath = texturePaths[0] ?? null;

    let status: ModelSupportStatus = 'none';
    let reasonRu = '';
    let reasonEn = '';

    if (cycleDetected) {
      status = cuboids.length > 0 ? '3d-json-elements' : 'none';
      reasonRu = 'Обнаружена циклическая ссылка parent в цепочке модели (защищено от зависания).';
      reasonEn = 'Cyclic parent inheritance detected (safely aborted).';
    } else if (isObj) {
      status = 'unsupported-obj';
      reasonRu = 'Модель использует формат Wavefront OBJ (neoforge:obj), не поддерживаемый в 3D-JSON просмотрщике.';
      reasonEn = 'Model uses Wavefront OBJ loader (neoforge:obj), unsupported in static JSON 3D viewer.';
    } else if (cuboids.length > 0) {
      status = '3d-json-elements';
      reasonRu = `Статическая 3D-модель Minecraft JSON (${cuboids.length} геом. элементов).`;
      reasonEn = `Static Minecraft JSON 3D model (${cuboids.length} cuboid elements).`;
    } else if (is2dItem) {
      status = '2d-item-generated';
      reasonRu = 'Плоская 2D-иконка предмета (item/generated) — не является 3D-моделью блока.';
      reasonEn = 'Flat 2D item sprite (item/generated) — not a 3D block model.';
    } else if (isBuiltinEntity || chain.length > 0) {
      status = 'procedural-or-blockentity';
      reasonRu = 'Процедурная или BlockEntity-модель без статических JSON-элементов.';
      reasonEn = 'Procedural or BlockEntity-rendered block with no static JSON elements.';
    } else if (missingFile) {
      status = 'none';
      reasonRu = 'Файл модели отсутствует локально (проверьте assets-local/ или загрузите JSON вручную).';
      reasonEn = 'Model file missing locally (assets-local/ absent or unbundled vanilla resource).';
    } else {
      status = 'none';
      reasonRu = 'Модель не найдена.';
      reasonEn = 'Model not found.';
    }

    const res: ResolvedMcModel = {
      modelRef: normRef,
      status,
      reasonRu,
      reasonEn,
      cuboids,
      texturePaths,
      primaryTexturePath,
      parentChain,
    };

    if (!opts?.inlineOverrideJson) {
      this.modelCache.set(cacheKey, res);
    }
    return res;
  }

  /**
   * Resolves a blockstate + properties from an `.nbt` schematic palette entry into combined 3D cuboids.
   */
  async resolveBlockWithState(
    blockId: string,
    properties: Record<string, string> = {}
  ): Promise<ResolvedMcModel> {
    const bs = await this.getBlockstate(blockId);
    if (!bs) {
      // Fallback: try direct block model if no blockstate JSON exists locally
      const norm = normalizeModelRef(blockId);
      const [ns, sub] = norm.split(':', 2);
      if (ns === 'minecraft') {
        return {
          modelRef: norm,
          status: 'none',
          reasonRu: 'Ванильный блок (текстуры/модели Minecraft не распространяются в проекте).',
          reasonEn: 'Vanilla Minecraft block (vanilla textures/models are not bundled).',
          cuboids: [],
          texturePaths: [],
          primaryTexturePath: null,
          parentChain: [],
        };
      }
      return this.resolveModel(`${ns}:block/${sub}`);
    }

    const parts = selectBlockstateModels(bs, properties);
    if (parts.length === 0) {
      return {
        modelRef: blockId,
        status: 'procedural-or-blockentity',
        reasonRu: 'Вариант blockstate не содержит статических моделей.',
        reasonEn: 'Blockstate matched no static model parts.',
        cuboids: [],
        texturePaths: [],
        primaryTexturePath: null,
        parentChain: [],
      };
    }

    const resolvedParts = await Promise.all(
      parts.map((p) =>
        this.resolveModel(p.model, {
          blockRotationX: p.x,
          blockRotationY: p.y,
        })
      )
    );

    const combinedCuboids: ResolvedCuboid[] = [];
    const combinedTex = new Set<string>();
    for (const rp of resolvedParts) {
      combinedCuboids.push(...rp.cuboids);
      for (const t of rp.texturePaths) combinedTex.add(t);
    }

    const primary = resolvedParts[0];
    return {
      modelRef: primary.modelRef,
      status: combinedCuboids.length > 0 ? '3d-json-elements' : primary.status,
      reasonRu:
        combinedCuboids.length > 0
          ? `Blockstate -> 3D JSON (${resolvedParts.length} частей, ${combinedCuboids.length} элементов).`
          : primary.reasonRu,
      reasonEn:
        combinedCuboids.length > 0
          ? `Blockstate -> 3D JSON (${resolvedParts.length} parts, ${combinedCuboids.length} elements).`
          : primary.reasonEn,
      cuboids: combinedCuboids,
      texturePaths: Array.from(combinedTex),
      primaryTexturePath: primary.primaryTexturePath,
      parentChain: primary.parentChain,
    };
  }
}

export function createBrowserAssetFetcher(
  customOverrideJsonByRef?: Map<string, RawMcModelJson>
): McJsonFetcher {
  return {
    async fetchModelJson(modelRef: string): Promise<RawMcModelJson | null> {
      const norm = normalizeModelRef(modelRef);
      if (customOverrideJsonByRef?.has(norm)) {
        return customOverrideJsonByRef.get(norm)!;
      }
      const rel = modelRefToLocalPath(norm);
      if (!rel || typeof fetch === 'undefined') return null;
      try {
        // Check user disk overrides first, then local upstream originals
        const ovr = await fetch(`/assets-overrides/models/${rel}`);
        if (ovr.ok) return (await ovr.json()) as RawMcModelJson;
      } catch {
        // Ignore network errors cleanly
      }
      try {
        const res = await fetch(`/local-originals/${rel}`);
        if (!res.ok) return null;
        return (await res.json()) as RawMcModelJson;
      } catch {
        return null;
      }
    },

    async fetchBlockstateJson(blockId: string): Promise<Record<string, unknown> | null> {
      const rel = blockIdToBlockstatePath(blockId);
      if (!rel || typeof fetch === 'undefined') return null;
      try {
        const res = await fetch(`/local-originals/${rel}`);
        if (!res.ok) return null;
        return (await res.json()) as Record<string, unknown>;
      } catch {
        return null;
      }
    },
  };
}
