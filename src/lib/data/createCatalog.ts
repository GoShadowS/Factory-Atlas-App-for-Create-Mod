import type { FactoryProject, MachineDef, ModpackProfile } from '../types/atlas';

export const DEFAULT_PROFILE: ModpackProfile = {
  id: 'custom-profile-1-21-1',
  name: 'Create 6.0.11 (MC 1.21.1 · NeoForge)',
  minecraftVersion: '1.21.1',
  loader: 'neoforge',
  createVersion: '6.0.11',
  addons: [],
};

export const PRESET_PROFILES: ModpackProfile[] = [
  DEFAULT_PROFILE,
  {
    id: 'create-only-1-21-1',
    name: 'Только Create 6.0.11 (MC 1.21.1 · NeoForge)',
    minecraftVersion: '1.21.1',
    loader: 'neoforge',
    createVersion: '6.0.11',
    addons: [],
  },
  {
    id: 'create-fabric-1-20-1',
    name: 'Create Fabric 0.5.1 (MC 1.20.1 · Fabric)',
    minecraftVersion: '1.20.1',
    loader: 'fabric',
    createVersion: '0.5.1-j (Fabric)',
    addons: [],
  },
];

/**
 * Pure formula reference metadata (verified against Create 6.0.11 Java classes).
 * These are NOT pre-populated machines in a user project; they are optional calculation
 * rules a user can assign when creating their own machine.
 */
export interface VerifiedSpeedFormulaSpec {
  id: MachineDef['speedFormulaId'];
  nameRu: string;
  nameEn: string;
  supportsRpm: boolean;
  defaultStressSU: number | null;
  noteRu: string;
  noteEn: string;
}

export const VERIFIED_SPEED_FORMULAS: VerifiedSpeedFormulaSpec[] = [
  {
    id: 'unknown',
    nameRu: 'Неизвестно / задать вручную (честно)',
    nameEn: 'Unknown / Manual rate (honest)',
    supportsRpm: false,
    defaultStressSU: null,
    noteRu: 'Скорость задаётся вручную пользователем или остаётся неизвестной.',
    noteEn: 'Speed is user-entered manually or left explicitly unknown.',
  },
  {
    id: 'millstone',
    nameRu: 'Create 6.0.11: Жёрнов (MillstoneBlockEntity)',
    nameEn: 'Create 6.0.11: Millstone',
    supportsRpm: true,
    defaultStressSU: 4,
    noteRu: 'timer -= clamp(floor(|RPM|/16), 1, 512)',
    noteEn: 'timer -= clamp(floor(|RPM|/16), 1, 512)',
  },
  {
    id: 'mechanical_press',
    nameRu: 'Create 6.0.11: Механический пресс (PressingBehaviour)',
    nameEn: 'Create 6.0.11: Mechanical Press',
    supportsRpm: true,
    defaultStressSU: 8,
    noteRu: 'CYCLE=240, шаг=lerp(|RPM|/512, 1, 60)',
    noteEn: 'CYCLE=240, tickSpeed=lerp(|RPM|/512, 1, 60)',
  },
  {
    id: 'mechanical_mixer',
    nameRu: 'Create 6.0.11: Механический смешиватель (Mixer)',
    nameEn: 'Create 6.0.11: Mechanical Mixer',
    supportsRpm: true,
    defaultStressSU: 4,
    noteRu: '40 + max(log2(512/|RPM|)*15*(dur/100)+1, 1)',
    noteEn: '40 + max(log2(512/|RPM|)*15*(dur/100)+1, 1)',
  },
  {
    id: 'mechanical_saw',
    nameRu: 'Create 6.0.11: Механическая пила (Saw)',
    nameEn: 'Create 6.0.11: Mechanical Saw',
    supportsRpm: true,
    defaultStressSU: 4,
    noteRu: 'processingSpeed = clamp(|RPM|/24, 1, 128)',
    noteEn: 'processingSpeed = clamp(|RPM|/24, 1, 128)',
  },
  {
    id: 'deployer',
    nameRu: 'Create 6.0.11: Автономный активатор (Deployer)',
    nameEn: 'Create 6.0.11: Deployer',
    supportsRpm: true,
    defaultStressSU: 4,
    noteRu: 'timerSpeed = clamp(|RPM|*2, 8, 512), цикл 2500',
    noteEn: 'timerSpeed = clamp(|RPM|*2, 8, 512), 2500 ticks',
  },
  {
    id: 'spout',
    nameRu: 'Create 6.0.11: Дозатор (Spout, 20 тиков)',
    nameEn: 'Create 6.0.11: Spout (20 ticks)',
    supportsRpm: false,
    defaultStressSU: 0,
    noteRu: 'FILLING_TIME = 20 тиков (без кинетики)',
    noteEn: 'FILLING_TIME = 20 ticks (no kinetic load)',
  },
  {
    id: 'item_drain',
    nameRu: 'Create 6.0.11: Осушитель предметов (Item Drain, 20 тиков)',
    nameEn: 'Create 6.0.11: Item Drain (20 ticks)',
    supportsRpm: false,
    defaultStressSU: 0,
    noteRu: 'FILLING_TIME = 20 тиков',
    noteEn: 'FILLING_TIME = 20 ticks',
  },
  {
    id: 'crushing_wheel',
    nameRu: 'Create 6.0.11: Дробильные колёса (допущение пачки)',
    nameEn: 'Create 6.0.11: Crushing Wheels (batch assumption)',
    supportsRpm: true,
    defaultStressSU: 16,
    noteRu: 'Зависит от RPM и размера пачки (помечается как допущение)',
    noteEn: 'Depends on RPM & batch size (explicitly marked assumption)',
  },
  {
    id: 'encased_fan',
    nameRu: 'Create 6.0.11: Вентилятор в корпусе (допущение пачки)',
    nameEn: 'Create 6.0.11: Encased Fan (batch assumption)',
    supportsRpm: true,
    defaultStressSU: 2,
    noteRu: '150 тиков * ((пачка-1)/16 + 1) (помечается как допущение)',
    noteEn: '150 ticks * ((stack-1)/16 + 1) (marked assumption)',
  },
  {
    id: 'mechanical_crafter',
    nameRu: 'Create 6.0.11: Механический сборщик (допущение)',
    nameEn: 'Create 6.0.11: Mechanical Crafter (assumption)',
    supportsRpm: true,
    defaultStressSU: 2,
    noteRu: 'Зависит от числа ячеек сетки и RPM (допущение)',
    noteEn: 'Depends on grid cell count and RPM (assumption)',
  },
  {
    id: 'fixed_ticks',
    nameRu: 'Фиксированная длительность из рецепта (тики)',
    nameEn: 'Fixed ticks from recipe duration',
    supportsRpm: false,
    defaultStressSU: 0,
    noteRu: 'Использует processingTimeTicks рецепта (1200 тиков = 1 мин)',
    noteEn: 'Uses recipe processingTimeTicks (1200 ticks = 1 min)',
  },
];

/**
 * Always returns a completely EMPTY project (0 nodes, 0 edges, 0 pre-filled
 * resources, 0 pre-filled machines, 0 pre-filled recipes).
 */
export function createEmptyProject(name = 'Новый проект'): FactoryProject {
  const now = new Date().toISOString();
  return {
    schemaVersion: 2,
    id: `proj-${Date.now()}-${Math.floor(Math.random() * 10000)}`,
    name,
    createdAt: now,
    updatedAt: now,
    profile: structuredClone(DEFAULT_PROFILE),
    nodes: [],
    edges: [],
    customResources: [],
    customMachines: [],
    customRecipes: [],
  };
}
