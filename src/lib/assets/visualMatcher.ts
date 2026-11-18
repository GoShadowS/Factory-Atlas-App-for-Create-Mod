import type { MatchReason, VisualEntrySummary } from '../types/atlas';

export interface VisualMatchResult {
  status: 'unique' | 'ambiguous' | 'not-found';
  reason: MatchReason;
  best: VisualEntrySummary | null;
  candidates: VisualEntrySummary[];
}

export function normalizeVisualQuery(input: string): string {
  return input.toLowerCase().replace(/[^a-z0-9а-яё]+/gi, '');
}

function tokenizeQuery(input: string): string[] {
  return input
    .toLowerCase()
    .split(/[^a-z0-9а-яё]+/gi)
    .filter((t) => t.length >= 2);
}

/**
 * Searches the local visual reference index strictly for matching 3D models & textures.
 * Follows the exact 5-tier lookup order:
 * 1. Exact namespaced ID (`create:cogwheel`)
 * 2. Exact model/texture path (`create/models/block/cogwheel.json`, etc.)
 * 3. Normalized filename & ID (ignoring case, spaces, underscores, hyphens)
 * 4. Exact localized names (`lang` RU/EN)
 * 5. Partial/token RU/EN match (returns ambiguous candidates if >1 match)
 */
export function matchVisualResources(
  query: string,
  entries: VisualEntrySummary[],
  maxCandidates = 12
): VisualMatchResult {
  const raw = query.trim();
  if (!raw || entries.length === 0) {
    return { status: 'not-found', reason: 'none', best: null, candidates: [] };
  }

  const lower = raw.toLowerCase();

  // 1. Exact namespaced ID
  const exactId = entries.filter((e) => e.id.toLowerCase() === lower);
  if (exactId.length === 1) {
    return { status: 'unique', reason: 'exact-id', best: exactId[0], candidates: exactId };
  }
  if (exactId.length > 1) {
    return { status: 'ambiguous', reason: 'exact-id', best: null, candidates: exactId.slice(0, maxCandidates) };
  }

  // 2. Exact model, blockstate, or texture path (full or relative)
  const cleanPath = lower.replace(/^\/?(local-originals|assets)\//, '').replace(/\.json$|\.png$/, '');
  const exactPath = entries.filter((e) => {
    const paths = [e.modelPath, e.worldModelPath, e.texturePath, e.blockstatePath, e.modelRef, e.pathId]
      .filter((p): p is string => Boolean(p))
      .map((p) => p.toLowerCase().replace(/\.json$|\.png$/, ''));
    return paths.some((p) => p === cleanPath || p.endsWith(`/${cleanPath}`));
  });
  if (exactPath.length === 1) {
    return { status: 'unique', reason: 'exact-path', best: exactPath[0], candidates: exactPath };
  }
  if (exactPath.length > 1) {
    return { status: 'ambiguous', reason: 'exact-path', best: null, candidates: exactPath.slice(0, maxCandidates) };
  }

  // 3. Normalized filename & ID (ignore case, spaces, _, -)
  const norm = normalizeVisualQuery(raw);
  if (norm.length >= 2) {
    const byNormId = entries.filter(
      (e) =>
        e.normalizedKey === norm ||
        normalizeVisualQuery(e.id) === norm
    );
    if (byNormId.length === 1) {
      return { status: 'unique', reason: 'normalized', best: byNormId[0], candidates: byNormId };
    }
    if (byNormId.length > 1) {
      return { status: 'ambiguous', reason: 'normalized', best: null, candidates: byNormId.slice(0, maxCandidates) };
    }
  }

  // 4. Exact localized names from lang files (RU / EN)
  if (norm.length >= 2) {
    const byLangExact = entries.filter(
      (e) =>
        (e.normalizedRu && e.normalizedRu === norm) ||
        (e.normalizedEn && e.normalizedEn === norm)
    );
    if (byLangExact.length === 1) {
      return { status: 'unique', reason: 'lang-alias', best: byLangExact[0], candidates: byLangExact };
    }
    if (byLangExact.length > 1) {
      return { status: 'ambiguous', reason: 'lang-alias', best: null, candidates: byLangExact.slice(0, maxCandidates) };
    }
  }

  // 5. Partial / token match on RU/EN names & IDs
  const tokens = tokenizeQuery(raw);
  if (tokens.length === 0) {
    return { status: 'not-found', reason: 'none', best: null, candidates: [] };
  }

  const scored: Array<{ entry: VisualEntrySummary; score: number }> = [];
  for (const e of entries) {
    const hayId = e.id.toLowerCase();
    const hayEn = e.nameEn.toLowerCase();
    const hayRu = (e.nameRu || '').toLowerCase();

    let hits = 0;
    let bonus = 0;
    for (const tok of tokens) {
      if (hayRu.includes(tok) || hayEn.includes(tok) || hayId.includes(tok)) {
        hits++;
        if (hayRu.startsWith(tok) || hayEn.startsWith(tok) || e.pathId.startsWith(tok)) {
          bonus += 2;
        }
      }
    }
    if (hits === tokens.length) {
      scored.push({ entry: e, score: hits * 10 + bonus + (e.hasModel3D ? 1 : 0) });
    }
  }

  if (scored.length === 0) {
    return { status: 'not-found', reason: 'none', best: null, candidates: [] };
  }

  scored.sort((a, b) => b.score - a.score || a.entry.id.localeCompare(b.entry.id));

  if (scored.length === 1) {
    return { status: 'unique', reason: 'fuzzy', best: scored[0].entry, candidates: [scored[0].entry] };
  }

  return {
    status: 'ambiguous',
    reason: 'fuzzy',
    best: null,
    candidates: scored.slice(0, maxCandidates).map((s) => s.entry),
  };
}
