import type {
  AssetManifestEntry,
  MachineDef,
  ResourceDef,
  TextureStatus,
  VisualEntrySummary,
} from '../types/atlas';

export interface LocalOriginalMeta {
  id: string;
  modId: string;
  modVersion: string;
  sourceRepo: string;
  sourcePath: string;
  localOriginalPath: string;
  license: string;
  width: number | null;
  height: number | null;
  animatedStrip: boolean;
}

export interface TextureOverrideMap {
  [assetId: string]: {
    dataUrl: string;
    fileName: string;
    updatedAt: string;
  };
}

const STORAGE_KEY_OVERRIDES = 'factory_atlas_texture_overrides_v1';

export function assetIdToOverrideRelPaths(assetId: string): [string, string] {
  const [ns, name] = assetId.includes(':') ? assetId.split(':', 2) : ['custom', assetId];
  const safeNs = ns.replace(/[^a-zA-Z0-9._-]/g, '_');
  const safeName = name.replace(/[^a-zA-Z0-9._/-]/g, '_');
  return [`${safeNs}__${safeName}.png`, `${safeNs}/${safeName}.png`];
}

export function fallbackUrlForKind(kind: AssetManifestEntry['kind']): string {
  switch (kind) {
    case 'fluid':
      return '/assets-fallback/fluid-fallback.svg';
    case 'machine':
    case 'block':
      return '/assets-fallback/machine-fallback.svg';
    default:
      return '/assets-fallback/item-fallback.svg';
  }
}

export function loadBrowserOverrides(): TextureOverrideMap {
  if (typeof localStorage === 'undefined') return {};
  try {
    const raw = localStorage.getItem(STORAGE_KEY_OVERRIDES);
    if (!raw) return {};
    const parsed = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? (parsed as TextureOverrideMap) : {};
  } catch {
    return {};
  }
}

export function saveBrowserOverride(
  assetId: string,
  dataUrl: string,
  fileName: string
): TextureOverrideMap {
  const map = loadBrowserOverrides();
  map[assetId] = {
    dataUrl,
    fileName,
    updatedAt: new Date().toISOString(),
  };
  if (typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem(STORAGE_KEY_OVERRIDES, JSON.stringify(map));
    } catch {
      // Ignore quota errors cleanly
    }
  }
  return map;
}

export function removeBrowserOverride(assetId: string): TextureOverrideMap {
  const map = loadBrowserOverrides();
  delete map[assetId];
  if (typeof localStorage !== 'undefined') {
    try {
      localStorage.setItem(STORAGE_KEY_OVERRIDES, JSON.stringify(map));
    } catch {
      // Ignore storage errors
    }
  }
  return map;
}

const MACHINE_TEXTURE_ALIAS: Record<string, string> = {
  'createdieselgenerators:distillation_tower': 'createdieselgenerators:distillation_controller',
  'create:sequenced_assembly_line': 'create:precision_mechanism',
};

/**
 * Resolves the active texture manifest strictly from:
 * 1. User project resources/machines (never a pre-populated built-in game catalog!)
 * 2. Local visual reference index (`assets-local/visual-index.json`) if present
 * 3. User custom disk or browser overrides (`assets-overrides/` / `localStorage` / project `.visual`)
 * 4. Built-in neutral MIT Factory Atlas fallback SVGs (`public/assets-fallback/`)
 */
export function buildAssetManifest(params: {
  localOriginals: LocalOriginalMeta[];
  visualEntries?: VisualEntrySummary[];
  userResources?: ResourceDef[];
  userMachines?: MachineDef[];
  diskOverrides: string[];
  browserOverrides: TextureOverrideMap;
  extraIds?: Array<{ id: string; nameRu: string; nameEn: string; kind: AssetManifestEntry['kind']; modId: string }>;
}): AssetManifestEntry[] {
  const origById = new Map(params.localOriginals.map((o) => [o.id, o]));
  const visualById = new Map((params.visualEntries ?? []).map((v) => [v.id, v]));
  const diskSet = new Set(params.diskOverrides);

  const entries: AssetManifestEntry[] = [];
  const seen = new Set<string>();

  const register = (
    id: string,
    nameRu: string,
    nameEn: string,
    kind: AssetManifestEntry['kind'],
    modId: string,
    modVersion: string,
    customVisual?: ResourceDef['visual']
  ) => {
    if (seen.has(id)) return;
    seen.add(id);

    const aliasId = MACHINE_TEXTURE_ALIAS[id];
    const visualId = customVisual?.visualId || id;
    const vis =
      visualById.get(visualId) ??
      visualById.get(id) ??
      (aliasId ? visualById.get(aliasId) : undefined);
    const orig =
      origById.get(visualId) ??
      origById.get(id) ??
      (aliasId ? origById.get(aliasId) : undefined);

    const [flatRel, nestedRel] = assetIdToOverrideRelPaths(id);
    const browserOvr = params.browserOverrides[id];
    const diskRel = diskSet.has(flatRel) ? flatRel : diskSet.has(nestedRel) ? nestedRel : null;

    let status: TextureStatus = 'fallback';
    let url = fallbackUrlForKind(kind);

    if (customVisual?.disabled) {
      status = 'fallback';
      url = fallbackUrlForKind(kind);
    } else if (customVisual?.customTextureOverrideDataUrl) {
      status = 'user-override';
      url = customVisual.customTextureOverrideDataUrl;
    } else if (browserOvr) {
      status = 'user-override';
      url = browserOvr.dataUrl;
    } else if (diskRel) {
      status = 'user-override';
      url = `/assets-overrides/${encodeURIComponent(diskRel).replace(/%2F/g, '/')}`;
    } else if (customVisual?.texturePath) {
      status = 'temporary-original';
      url = `/local-originals/${customVisual.texturePath}`;
    } else if (vis?.texturePath) {
      status = 'temporary-original';
      url = `/local-originals/${vis.texturePath}`;
    } else if (orig) {
      status = 'temporary-original';
      const rel = orig.localOriginalPath
        .replace(/^assets-local\/upstream-originals\//, '')
        .replace(/^assets-local\//, '');
      url = `/local-originals/${rel}`;
    }

    const isMinecraftVanilla = modId === 'minecraft';

    entries.push({
      id,
      nameRu,
      nameEn,
      kind,
      modId,
      modVersion,
      sourceRepo: vis?.sourceRepo ?? orig?.sourceRepo ?? (isMinecraftVanilla ? 'Minecraft Client Assets (не входит в репозиторий)' : 'Локальный проект / пользовательский ресурс'),
      sourcePath: vis?.texturePath ?? orig?.sourcePath ?? '—',
      recommendedSize: orig?.width && orig?.height ? `${orig.width}x${orig.height}` : '16x16 (или 32x32) PNG',
      license: vis?.license ?? orig?.license ?? (isMinecraftVanilla ? 'Mojang Proprietary (не распространяется)' : 'Custom / User'),
      status,
      url,
      animatedStrip: orig?.animatedStrip ?? false,
      frameWidth: orig?.width ?? 16,
      frameHeight: orig?.animatedStrip ? (orig.width ?? 16) : (orig?.height ?? 16),
      missingUpstreamReason:
        vis?.texturePath || orig
          ? undefined
          : isMinecraftVanilla
            ? 'Текстуры ванильного Minecraft не копировались из-за прав Mojang; вы можете поместить свою перерисовку в assets-overrides/'
            : 'Локальная текстура не привязана или assets-local/ отсутствует; используется нейтральная заглушка.',
    });
  };

  for (const r of params.userResources ?? []) {
    register(
      r.id,
      r.nameRu,
      r.nameEn,
      r.kind === 'fluid' ? 'fluid' : 'item',
      r.modId,
      'user',
      r.visual
    );
  }
  for (const m of params.userMachines ?? []) {
    register(m.id, m.nameRu, m.nameEn, 'machine', m.modId, m.modVersion, m.visual);
  }
  for (const ex of params.extraIds ?? []) {
    register(ex.id, ex.nameRu, ex.nameEn, ex.kind, ex.modId, 'custom');
  }
  for (const orig of params.localOriginals) {
    if (!seen.has(orig.id)) {
      const shortName = orig.id.split(':')[1]?.replace(/_/g, ' ') ?? orig.id;
      register(orig.id, shortName, shortName, 'block', orig.modId, orig.modVersion);
    }
  }

  return entries;
}
