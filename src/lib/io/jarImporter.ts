import { unzipSync, type UnzipFileInfo } from 'fflate';
import type {
  JarImportReport,
  ModLoader,
  RecipeDef,
  RecipeIngredient,
  RecipeOutput,
  ResourceKind,
} from '../types/atlas';

export const JAR_LIMITS = {
  maxArchiveBytes: 64 * 1024 * 1024, // 64 MiB
  maxEntries: 15000,
  maxEntryUncompressedBytes: 2 * 1024 * 1024, // 2 MiB per JSON/PNG
  maxTotalExtractedBytes: 32 * 1024 * 1024, // 32 MiB total
  maxCompressionRatio: 250,
  maxParseTimeMs: 8000,
} as const;

export interface JarImportOptions {
  extractTextures?: boolean;
  convertFabricDroplets?: boolean;
}

export interface ImportedArchiveOutput {
  report: JarImportReport;
  extractedTextures: Record<string, Uint8Array>;
}

const CREATE_TYPE_TO_MACHINE: Record<string, string> = {
  'create:pressing': 'create:mechanical_press',
  'create:mixing': 'create:mechanical_mixer',
  'create:compacting': 'create:mechanical_press',
  'create:milling': 'create:millstone',
  'create:crushing': 'create:crushing_wheel',
  'create:cutting': 'create:mechanical_saw',
  'create:deploying': 'create:deployer',
  'create:filling': 'create:spout',
  'create:emptying': 'create:item_drain',
  'create:splashing': 'create:encased_fan',
  'create:haunting': 'create:encased_fan',
  'create:blasting': 'create:encased_fan',
  'create:smoking': 'create:encased_fan',
  'create:mechanical_crafting': 'create:mechanical_crafter',
  'create:sequenced_assembly': 'create:sequenced_assembly_line',
  'createaddition:rolling': 'createaddition:rolling_mill',
  'createdieselgenerators:distillation': 'createdieselgenerators:distillation_tower',
  'createdieselgenerators:basin_fermenting': 'create:mechanical_mixer',
  'minecraft:crafting_shaped': 'create:mechanical_crafter',
  'minecraft:crafting_shapeless': 'create:mechanical_crafter',
  'minecraft:smelting': 'create:encased_fan',
  'minecraft:blasting': 'create:encased_fan',
  'minecraft:smoking': 'create:encased_fan',
};

export function isSafeArchivePath(entryName: string): boolean {
  if (!entryName || entryName.includes('\0') || entryName.includes('\\')) return false;
  if (entryName.startsWith('/') || /^[A-Za-z]:/.test(entryName)) return false;
  const parts = entryName.split('/');
  return !parts.some((p) => p === '..');
}

function isRecipeJsonPath(entryName: string): boolean {
  // Strictly allow data/<ns>/recipe/*.json or data/<ns>/recipes/*.json
  // Explicitly reject advancement/recipes/*!
  if (!entryName.startsWith('data/') || !entryName.endsWith('.json')) return false;
  if (entryName.includes('/advancement/') || entryName.includes('/advancements/')) return false;
  if (entryName.includes('/loot_table/') || entryName.includes('/loot_tables/')) return false;
  const parts = entryName.split('/');
  return parts.length >= 4 && (parts[2] === 'recipe' || parts[2] === 'recipes');
}

function isTexturePngPath(entryName: string): boolean {
  // assets/<ns>/textures/(item|block)/*.png
  if (!entryName.startsWith('assets/') || !entryName.endsWith('.png')) return false;
  const parts = entryName.split('/');
  return parts.length >= 5 && parts[2] === 'textures' && (parts[3] === 'item' || parts[3] === 'block');
}

/**
 * Safely imports recipes (and optional item/block PNG textures) from a local .jar / .zip archive
 * without executing any Java code or making any network requests.
 */
export function importModArchive(
  bytes: Uint8Array,
  fileName: string,
  options: JarImportOptions = {}
): ImportedArchiveOutput {
  const startMs = Date.now();
  if (bytes.byteLength > JAR_LIMITS.maxArchiveBytes) {
    throw new Error(
      `Архив «${fileName}» (${(bytes.byteLength / (1024 * 1024)).toFixed(1)} МБ) превышает безопасный лимит 64 МБ.`
    );
  }

  let entriesScanned = 0;
  let totalUncompressedSelected = 0;
  const securityErrors: string[] = [];
  const pathTraversalBlocked: string[] = [];

  let files: Record<string, Uint8Array>;
  try {
    files = unzipSync(bytes, {
      filter(info: UnzipFileInfo) {
        entriesScanned++;
        if (entriesScanned > JAR_LIMITS.maxEntries) {
          securityErrors.push(`Превышен лимит числа записей (${JAR_LIMITS.maxEntries}).`);
          return false;
        }
        if (Date.now() - startMs > JAR_LIMITS.maxParseTimeMs) {
          securityErrors.push('Превышен тайм-аут чтения архива.');
          return false;
        }
        if (!isSafeArchivePath(info.name)) {
          pathTraversalBlocked.push(info.name);
          return false;
        }
        const wantMeta =
          info.name === 'META-INF/neoforge.mods.toml' ||
          info.name === 'META-INF/mods.toml' ||
          info.name === 'fabric.mod.json';
        const wantRecipe = isRecipeJsonPath(info.name);
        const wantTex = Boolean(options.extractTextures) && isTexturePngPath(info.name);

        if (!wantMeta && !wantRecipe && !wantTex) return false;

        if (info.originalSize > JAR_LIMITS.maxEntryUncompressedBytes) {
          securityErrors.push(`Файл «${info.name}» превышает лимит 2 МБ.`);
          return false;
        }
        if (
          info.originalSize > 512 * 1024 &&
          info.size > 0 &&
          info.originalSize / info.size > JAR_LIMITS.maxCompressionRatio
        ) {
          securityErrors.push(`Подозрение на архивную бомбу в «${info.name}» (сжатие > ${JAR_LIMITS.maxCompressionRatio}:1).`);
          return false;
        }
        if (totalUncompressedSelected + info.originalSize > JAR_LIMITS.maxTotalExtractedBytes) {
          securityErrors.push('Превышен общий лимит распаковки данных (32 МБ).');
          return false;
        }
        totalUncompressedSelected += info.originalSize;
        return true;
      },
    });
  } catch (err) {
    throw new Error(`Не удалось безопасно прочитать ZIP/JAR архив «${fileName}»: ${(err as Error).message}`);
  }

  const decoder = new TextDecoder('utf-8');
  let detectedModId = 'unknown';
  let detectedModName = fileName;
  let detectedModVersion = 'unknown';
  let detectedLoader: ModLoader | 'unknown' = 'unknown';
  let detectedMinecraftVersion = 'unknown';

  const neoforgeToml = files['META-INF/neoforge.mods.toml'];
  const forgeToml = files['META-INF/mods.toml'];
  const fabricJson = files['fabric.mod.json'];

  if (neoforgeToml || forgeToml) {
    detectedLoader = neoforgeToml ? 'neoforge' : 'forge';
    const text = decoder.decode(neoforgeToml ?? forgeToml);
    const modIdMatch = text.match(/modId\s*=\s*["']([^"']+)["']/);
    const verMatch = text.match(/version\s*=\s*["']([^"']+)["']/);
    const nameMatch = text.match(/displayName\s*=\s*["']([^"']+)["']/);
    const mcMatch = text.match(/modId\s*=\s*["']minecraft["'][\s\S]{0,160}?versionRange\s*=\s*["']([^"']+)["']/);
    if (modIdMatch) detectedModId = modIdMatch[1];
    if (verMatch) detectedModVersion = verMatch[1];
    if (nameMatch) detectedModName = nameMatch[1];
    if (mcMatch) detectedMinecraftVersion = mcMatch[1];
  } else if (fabricJson) {
    detectedLoader = 'fabric';
    try {
      const parsed = JSON.parse(decoder.decode(fabricJson)) as Record<string, unknown>;
      if (typeof parsed.id === 'string') detectedModId = parsed.id;
      if (typeof parsed.name === 'string') detectedModName = parsed.name;
      if (typeof parsed.version === 'string') detectedModVersion = parsed.version;
      const deps = parsed.depends as Record<string, unknown> | undefined;
      if (deps && typeof deps.minecraft === 'string') detectedMinecraftVersion = deps.minecraft;
    } catch {
      // Ignore malformed fabric.mod.json cleanly
    }
  }

  const shouldConvertDroplets =
    options.convertFabricDroplets ?? detectedLoader === 'fabric';

  const recipes: RecipeDef[] = [];
  const skippedDetails: JarImportReport['skippedDetails'] = [];
  const extractedTextures: Record<string, Uint8Array> = {};
  const warnings = [...securityErrors];

  if (pathTraversalBlocked.length > 0) {
    warnings.push(
      `Заблокировано подозрительных путей (path traversal): ${pathTraversalBlocked.length}`
    );
  }

  for (const [entryPath, data] of Object.entries(files)) {
    if (isTexturePngPath(entryPath)) {
      const parts = entryPath.split('/');
      const ns = parts[1];
      const base = parts[parts.length - 1].replace(/\.png$/, '');
      if (data.byteLength > 8 && data[0] === 0x89 && data[1] === 0x50) {
        extractedTextures[`${ns}:${base}`] = data;
      }
      continue;
    }

    if (!isRecipeJsonPath(entryPath)) continue;

    let rawJson: unknown;
    try {
      rawJson = JSON.parse(decoder.decode(data));
    } catch {
      skippedDetails.push({
        path: entryPath,
        reasonCode: 'INVALID_JSON',
        messageRu: 'Повреждённый JSON рецепта',
        messageEn: 'Invalid recipe JSON',
      });
      continue;
    }

    const parsed = parseSingleRecipeJson(rawJson, entryPath, {
      modId: detectedModId,
      modVersion: detectedModVersion,
      minecraftVersion: detectedMinecraftVersion,
      loader: detectedLoader === 'unknown' ? 'neoforge' : detectedLoader,
      convertDroplets: shouldConvertDroplets,
    });

    if (parsed.ok) {
      recipes.push(parsed.recipe);
    } else {
      skippedDetails.push({
        path: entryPath,
        reasonCode: parsed.reasonCode,
        messageRu: parsed.reasonRu,
        messageEn: parsed.reasonEn,
      });
    }
  }

  return {
    report: {
      fileName,
      detectedModId,
      detectedModName,
      detectedModVersion,
      detectedLoader,
      detectedMinecraftVersion,
      archiveSizeBytes: bytes.byteLength,
      entriesScanned,
      importedCount: recipes.length,
      skippedCount: skippedDetails.length,
      extractedTexturesCount: Object.keys(extractedTextures).length,
      recipes,
      skippedDetails,
      warnings,
    },
    extractedTextures,
  };
}

export type SingleRecipeParseResult =
  | { ok: true; recipe: RecipeDef }
  | { ok: false; reasonCode: string; reasonRu: string; reasonEn: string };

export function parseSingleRecipeJson(
  raw: unknown,
  recipePath: string,
  meta: {
    modId: string;
    modVersion: string;
    minecraftVersion: string;
    loader: ModLoader;
    convertDroplets?: boolean;
  }
): SingleRecipeParseResult {
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
    return {
      ok: false,
      reasonCode: 'NOT_OBJECT',
      reasonRu: 'JSON рецепта не является объектом.',
      reasonEn: 'Recipe JSON is not an object.',
    };
  }
  const obj = raw as Record<string, unknown>;
  const type = typeof obj.type === 'string' ? obj.type : '';
  if (!type) {
    return {
      ok: false,
      reasonCode: 'MISSING_TYPE',
      reasonRu: 'Отсутствует поле type.',
      reasonEn: 'Missing recipe "type" field.',
    };
  }

  if (type.startsWith('minecraft:crafting_special')) {
    return {
      ok: false,
      reasonCode: 'SPECIAL_CRAFTING',
      reasonRu: `Специальный динамический рецепт (${type}) не имеет фиксированных входов/выходов.`,
      reasonEn: `Special dynamic crafting type (${type}) has no static I/O.`,
    };
  }

  const machineId = CREATE_TYPE_TO_MACHINE[type];
  if (!machineId) {
    return {
      ok: false,
      reasonCode: 'UNSUPPORTED_TYPE',
      reasonRu: `Неподдерживаемый или неизвестный тип рецепта: ${type}`,
      reasonEn: `Unsupported or unknown recipe type: ${type}`,
    };
  }

  const dropletDiv = meta.convertDroplets ? 81 : 1;
  const inputs: RecipeIngredient[] = [];
  const outputs: RecipeOutput[] = [];

  const parseIng = (rawIng: unknown): RecipeIngredient | null => {
    if (!rawIng || typeof rawIng !== 'object') return null;
    if (Array.isArray(rawIng)) {
      return rawIng.length > 0 ? parseIng(rawIng[0]) : null;
    }
    const g = rawIng as Record<string, unknown>;
    if (typeof g.fluid === 'string') {
      const amt = typeof g.amount === 'number' ? g.amount / dropletDiv : 1000 / dropletDiv;
      return { resourceId: g.fluid, kind: 'fluid', amount: round3(amt) };
    }
    if (typeof g.fluid_tag === 'string' || typeof g.fluidTag === 'string') {
      const tag = String(g.fluid_tag ?? g.fluidTag);
      const amt = typeof g.amount === 'number' ? g.amount / dropletDiv : 1000 / dropletDiv;
      return { resourceId: `#${tag}`, kind: 'fluid', amount: round3(amt), isTag: true, tagId: tag };
    }
    if (g.type === 'neoforge:tag' && typeof g.tag === 'string' && typeof g.amount === 'number') {
      return {
        resourceId: `#${g.tag}`,
        kind: 'fluid',
        amount: round3(g.amount / dropletDiv),
        isTag: true,
        tagId: g.tag,
      };
    }
    if (typeof g.item === 'string') {
      return { resourceId: g.item, kind: 'item', amount: typeof g.count === 'number' ? g.count : 1 };
    }
    if (typeof g.tag === 'string') {
      return {
        resourceId: `#${g.tag}`,
        kind: 'item',
        amount: typeof g.count === 'number' ? g.count : 1,
        isTag: true,
        tagId: g.tag,
      };
    }
    if (g.value && typeof g.value === 'object') return parseIng(g.value);
    return null;
  };

  if (type === 'minecraft:crafting_shaped' && Array.isArray(obj.pattern) && obj.key && typeof obj.key === 'object') {
    const counts = new Map<string, number>();
    for (const row of obj.pattern) {
      if (typeof row !== 'string') continue;
      for (const ch of row) {
        if (ch !== ' ') counts.set(ch, (counts.get(ch) ?? 0) + 1);
      }
    }
    const keyMap = obj.key as Record<string, unknown>;
    for (const [ch, cnt] of counts) {
      const parsed = parseIng(keyMap[ch]);
      if (parsed) {
        parsed.amount *= cnt;
        mergeIngredient(inputs, parsed);
      }
    }
  } else if (Array.isArray(obj.ingredients)) {
    for (const ing of obj.ingredients) {
      const parsed = parseIng(ing);
      if (!parsed) {
        return {
          ok: false,
          reasonCode: 'UNSUPPORTED_INGREDIENT',
          reasonRu: `Неизвестный формат ингредиента в ${type}`,
          reasonEn: `Unsupported ingredient structure in ${type}`,
        };
      }
      mergeIngredient(inputs, parsed);
    }
  } else if (obj.ingredient) {
    const parsed = parseIng(obj.ingredient);
    if (parsed) inputs.push(parsed);
  }

  // Outputs
  if (Array.isArray(obj.results)) {
    const rawPool = obj.results as Array<Record<string, unknown>>;
    const isWeightedSeq = type === 'create:sequenced_assembly' && rawPool.length > 1;
    const totalWeight = isWeightedSeq
      ? rawPool.reduce((acc, r) => acc + (typeof r.chance === 'number' ? r.chance : 1), 0)
      : 1;

    for (const r of rawPool) {
      if (!r || typeof r !== 'object') continue;
      const resId = typeof r.id === 'string' ? r.id : typeof r.item === 'string' ? r.item : typeof r.fluid === 'string' ? r.fluid : '';
      if (!resId) continue;
      const isFluid = typeof r.amount === 'number' || typeof r.fluid === 'string';
      const kind: ResourceKind = isFluid ? 'fluid' : 'item';
      const amount = isFluid
        ? round3(((typeof r.amount === 'number' ? r.amount : 1000) as number) / dropletDiv)
        : typeof r.count === 'number'
          ? r.count
          : 1;

      if (isWeightedSeq) {
        const w = typeof r.chance === 'number' ? r.chance : 1;
        outputs.push({
          resourceId: resId,
          kind,
          amount,
          chance: totalWeight > 0 ? w / totalWeight : 0,
          probabilityKind: 'weighted',
          weight: w,
        });
      } else {
        const ch = typeof r.chance === 'number' ? Math.min(1, Math.max(0, r.chance)) : 1;
        outputs.push({
          resourceId: resId,
          kind,
          amount,
          chance: ch,
          probabilityKind: ch < 1 ? 'bernoulli' : 'guaranteed',
        });
      }
    }
  } else if (obj.result && typeof obj.result === 'object') {
    const r = obj.result as Record<string, unknown>;
    const resId = typeof r.id === 'string' ? r.id : typeof r.item === 'string' ? r.item : '';
    if (resId) {
      outputs.push({
        resourceId: resId,
        kind: 'item',
        amount: typeof r.count === 'number' ? r.count : 1,
        chance: 1,
        probabilityKind: 'guaranteed',
      });
    }
  } else if (typeof obj.result === 'string') {
    outputs.push({
      resourceId: obj.result,
      kind: 'item',
      amount: typeof obj.count === 'number' ? obj.count : 1,
      chance: 1,
      probabilityKind: 'guaranteed',
    });
  }

  if (outputs.length === 0) {
    return {
      ok: false,
      reasonCode: 'MISSING_OUTPUTS',
      reasonRu: 'В рецепте не найдено ни одного результата.',
      reasonEn: 'No recipe outputs could be parsed.',
    };
  }

  const procTime =
    typeof obj.processing_time === 'number'
      ? obj.processing_time
      : typeof obj.processingTime === 'number'
        ? obj.processingTime
        : typeof obj.cookingtime === 'number'
          ? obj.cookingtime
          : null;

  const heatRaw = obj.heat_requirement ?? obj.heatRequirement;
  const heatRequirement =
    heatRaw === 'superheated' ? 'superheated' : heatRaw === 'heated' ? 'heated' : 'none';

  const nsMatch = recipePath.match(/^data\/([^/]+)\/recipes?\/(.+)\.json$/);
  const id = nsMatch ? `${nsMatch[1]}:${nsMatch[2]}` : `${meta.modId}:${recipePath}`;
  const niceName = id.split(':')[1]?.split('/').pop()?.replace(/_/g, ' ') ?? id;

  return {
    ok: true,
    recipe: {
      id,
      type,
      machineId,
      nameRu: `${niceName} [${type.split(':')[1] ?? type}]`,
      nameEn: `${niceName} [${type.split(':')[1] ?? type}]`,
      inputs,
      outputs,
      processingTimeTicks: procTime,
      heatRequirement,
      loops: typeof obj.loops === 'number' ? obj.loops : undefined,
      source: {
        origin: 'imported-jar',
        modId: meta.modId,
        modVersion: meta.modVersion,
        minecraftVersion: meta.minecraftVersion,
        loader: meta.loader,
        recipePath,
      },
    },
  };
}

function mergeIngredient(list: RecipeIngredient[], ing: RecipeIngredient): void {
  const existing = list.find((x) => x.resourceId === ing.resourceId && x.kind === ing.kind);
  if (existing) existing.amount = round3(existing.amount + ing.amount);
  else list.push(ing);
}

function round3(n: number): number {
  return Math.round(n * 1000) / 1000;
}
