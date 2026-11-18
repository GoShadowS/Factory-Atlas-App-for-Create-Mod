import { DEFAULT_PROFILE } from '../data/createCatalog';
import type { FactoryProject, ModpackProfile } from '../types/atlas';

export const CURRENT_PROJECT_SCHEMA_VERSION = 2;
export const PROJECT_FILE_EXTENSION = '.factoryatlas';

export interface ProjectParseSuccess {
  ok: true;
  project: FactoryProject;
  migratedFromVersion: number | null;
}

export interface ProjectParseError {
  ok: false;
  errorRu: string;
  errorEn: string;
}

export type ProjectParseResult = ProjectParseSuccess | ProjectParseError;

/**
 * Serializes a Factory Atlas project into pretty JSON with explicit schemaVersion.
 */
export function serializeProject(project: FactoryProject): string {
  const normalized: FactoryProject = {
    ...project,
    schemaVersion: CURRENT_PROJECT_SCHEMA_VERSION,
    updatedAt: new Date().toISOString(),
  };
  return JSON.stringify(normalized, null, 2);
}

/**
 * Parses and migrates a `.factoryatlas` JSON payload safely.
 * Never mutates or corrupts the caller's source string/file on failure.
 */
export function parseAndMigrateProject(rawText: string): ProjectParseResult {
  if (!rawText || typeof rawText !== 'string') {
    return {
      ok: false,
      errorRu: 'Файл проекта пуст или не является текстом.',
      errorEn: 'Project file is empty or not readable text.',
    };
  }
  if (rawText.length > 32 * 1024 * 1024) {
    return {
      ok: false,
      errorRu: 'Файл проекта превышает безопасный лимит 32 МБ.',
      errorEn: 'Project file exceeds safe 32 MB size limit.',
    };
  }

  let raw: unknown;
  try {
    raw = JSON.parse(rawText);
  } catch (err) {
    return {
      ok: false,
      errorRu: `Повреждённый JSON в файле проекта: ${(err as Error).message}`,
      errorEn: `Invalid JSON in project file: ${(err as Error).message}`,
    };
  }

  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    return {
      ok: false,
      errorRu: 'Некорректная структура проекта: ожидался JSON-объект.',
      errorEn: 'Invalid project structure: expected a JSON object.',
    };
  }

  const obj = raw as Record<string, unknown>;
  const versionRaw = obj.schemaVersion ?? obj.version ?? 1;
  const version = typeof versionRaw === 'number' ? versionRaw : Number(versionRaw);

  if (!Number.isInteger(version) || version < 1) {
    return {
      ok: false,
      errorRu: `Неподдерживаемая версия схемы проекта: ${String(versionRaw)}.`,
      errorEn: `Unsupported project schema version: ${String(versionRaw)}.`,
    };
  }

  if (version > CURRENT_PROJECT_SCHEMA_VERSION) {
    return {
      ok: false,
      errorRu: `Проект создан в более новой версии Factory Atlas (схема v${version}). Поддерживаются схемы до v${CURRENT_PROJECT_SCHEMA_VERSION}.`,
      errorEn: `Project requires newer Factory Atlas schema v${version} (max supported v${CURRENT_PROJECT_SCHEMA_VERSION}).`,
    };
  }

  let migratedFromVersion: number | null = null;
  let working: Record<string, unknown> = structuredClone(obj);

  // Migration v1 -> v2
  if (version === 1) {
    migratedFromVersion = 1;
    working = migrateV1ToV2(working);
  }

  if (!Array.isArray(working.nodes) || !Array.isArray(working.edges)) {
    return {
      ok: false,
      errorRu: 'В файле проекта отсутствуют обязательные списки узлов (nodes) или соединений (edges).',
      errorEn: 'Project file is missing required "nodes" or "edges" arrays.',
    };
  }

  const now = new Date().toISOString();
  const profile = sanitizeProfile(working.profile);

  const project: FactoryProject = {
    schemaVersion: 2,
    id: typeof working.id === 'string' && working.id ? working.id : `proj-${Date.now()}`,
    name: typeof working.name === 'string' && working.name.trim() ? working.name.trim() : 'Без названия',
    createdAt: typeof working.createdAt === 'string' ? working.createdAt : now,
    updatedAt: typeof working.updatedAt === 'string' ? working.updatedAt : now,
    profile,
    nodes: working.nodes.filter(isValidNodeLike).map((n) => ({
      ...n,
      x: Number.isFinite(n.x) ? n.x : 100,
      y: Number.isFinite(n.y) ? n.y : 100,
    })),
    edges: working.edges.filter(isValidEdgeLike).map((e) => ({
      ...e,
      resourceKind: e.resourceKind === 'fluid' ? 'fluid' : 'item',
      rateMode: e.rateMode === 'manual' ? 'manual' : 'auto',
    })),
    customResources: Array.isArray(working.customResources) ? working.customResources : [],
    customMachines: Array.isArray(working.customMachines) ? working.customMachines : [],
    customRecipes: Array.isArray(working.customRecipes) ? working.customRecipes : [],
  };

  return {
    ok: true,
    project,
    migratedFromVersion,
  };
}

function migrateV1ToV2(v1: Record<string, unknown>): Record<string, unknown> {
  const nodesRaw = Array.isArray(v1.nodes) ? v1.nodes : [];
  const edgesRaw = Array.isArray(v1.edges) ? v1.edges : [];

  // v1 used `label`, `rate`, `unit` ('mb/min' | 'items/min') and lacked `profile` / `customRecipes`
  const nodes = nodesRaw.map((item) => {
    if (!item || typeof item !== 'object') return item;
    const n = { ...(item as Record<string, unknown>) };
    if (typeof n.title !== 'string' && typeof n.label === 'string') {
      n.title = n.label;
    }
    if (n.kind === 'source' && n.sourceRatePerMin === undefined && typeof n.rate === 'number') {
      n.sourceRatePerMin = n.rate;
    }
    if (n.kind === 'target' && n.targetRatePerMin === undefined && typeof n.rate === 'number') {
      n.targetRatePerMin = n.rate;
    }
    if (n.resourceKind === undefined && typeof n.unit === 'string') {
      n.resourceKind = n.unit.toLowerCase().includes('mb') ? 'fluid' : 'item';
    }
    return n;
  });

  const edges = edgesRaw.map((item) => {
    if (!item || typeof item !== 'object') return item;
    const e = { ...(item as Record<string, unknown>) };
    if (e.fromNodeId === undefined && typeof e.from === 'string') e.fromNodeId = e.from;
    if (e.toNodeId === undefined && typeof e.to === 'string') e.toNodeId = e.to;
    if (e.resourceKind === undefined && typeof e.unit === 'string') {
      e.resourceKind = e.unit.toLowerCase().includes('mb') ? 'fluid' : 'item';
    }
    if (e.rateMode === undefined) e.rateMode = 'auto';
    return e;
  });

  return {
    ...v1,
    schemaVersion: 2,
    profile: v1.profile ?? structuredClone(DEFAULT_PROFILE),
    nodes,
    edges,
    customResources: Array.isArray(v1.customResources) ? v1.customResources : [],
    customMachines: Array.isArray(v1.customMachines) ? v1.customMachines : [],
    customRecipes: Array.isArray(v1.customRecipes) ? v1.customRecipes : [],
  };
}

function sanitizeProfile(raw: unknown): ModpackProfile {
  if (!raw || typeof raw !== 'object') return structuredClone(DEFAULT_PROFILE);
  const p = raw as Record<string, unknown>;
  return {
    id: typeof p.id === 'string' ? p.id : DEFAULT_PROFILE.id,
    name: typeof p.name === 'string' ? p.name : DEFAULT_PROFILE.name,
    minecraftVersion: typeof p.minecraftVersion === 'string' ? p.minecraftVersion : DEFAULT_PROFILE.minecraftVersion,
    loader: p.loader === 'fabric' || p.loader === 'forge' || p.loader === 'neoforge' ? p.loader : 'neoforge',
    createVersion: typeof p.createVersion === 'string' ? p.createVersion : DEFAULT_PROFILE.createVersion,
    addons: Array.isArray(p.addons) ? p.addons : structuredClone(DEFAULT_PROFILE.addons),
  };
}

function isValidNodeLike(v: unknown): v is FactoryProject['nodes'][number] {
  if (!v || typeof v !== 'object') return false;
  const n = v as Record<string, unknown>;
  const validKinds = new Set(['source', 'machine', 'recipe', 'byproduct', 'target', 'note']);
  return typeof n.id === 'string' && typeof n.kind === 'string' && validKinds.has(n.kind) && typeof n.title === 'string';
}

function isValidEdgeLike(v: unknown): v is FactoryProject['edges'][number] {
  if (!v || typeof v !== 'object') return false;
  const e = v as Record<string, unknown>;
  return (
    typeof e.id === 'string' &&
    typeof e.fromNodeId === 'string' &&
    typeof e.toNodeId === 'string' &&
    typeof e.resourceId === 'string'
  );
}
