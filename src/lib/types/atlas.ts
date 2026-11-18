export type ResourceKind = 'item' | 'fluid';
export type FlowUnit = 'items/min' | 'mB/min';
export type Locale = 'ru' | 'en';
export type ThemeMode = 'dark' | 'light';
export type ModLoader = 'neoforge' | 'forge' | 'fabric';

export interface AddonVersionEntry {
  id: string;
  name: string;
  version: string;
  verifiedRecipes: boolean;
}

export interface ModpackProfile {
  id: string;
  name: string;
  minecraftVersion: string;
  loader: ModLoader;
  createVersion: string;
  addons: AddonVersionEntry[];
}

export type ModelSupportStatus =
  | '3d-json-elements'
  | '2d-item-generated'
  | 'unsupported-obj'
  | 'procedural-or-blockentity'
  | 'none';

export type MatchReason =
  | 'exact-id'
  | 'exact-path'
  | 'normalized'
  | 'lang-alias'
  | 'fuzzy'
  | 'manual'
  | 'none';

export interface ResourceVisualAssignment {
  visualId: string | null;
  modelRef: string | null;
  modelPath: string | null;
  texturePath: string | null;
  modelSupport: ModelSupportStatus;
  customTextureOverrideDataUrl?: string;
  customModelJson?: string;
  disabled?: boolean;
  matchReason?: MatchReason;
}

export interface VisualSourceRepoMeta {
  namespace: string;
  repo: string;
  branch: string;
  commit: string;
  version: string;
  minecraftVersion: string;
  createVersion: string;
  codeLicense: string;
  assetsLicense: string;
}

export interface VisualEntrySummary {
  id: string;
  namespace: string;
  pathId: string;
  kind: 'block' | 'item' | 'fluid';
  nameEn: string;
  nameRu: string | null;
  normalizedKey: string;
  normalizedEn: string;
  normalizedRu: string;
  sourceRepo: string;
  sourceBranch: string;
  sourceCommit: string;
  modVersion: string;
  minecraftVersion: string;
  license: string;
  blockstatePath: string | null;
  modelRef: string | null;
  modelPath: string | null;
  worldModelRef: string | null;
  worldModelPath: string | null;
  texturePath: string | null;
  modelSupport: ModelSupportStatus;
  hasModel3D: boolean;
  hasTexture2D: boolean;
}

export interface VisualIndexData {
  generatedAt: string;
  sources: VisualSourceRepoMeta[];
  statsByNamespace: Record<string, Record<string, number>>;
  entries: VisualEntrySummary[];
}

export interface ResourceDef {
  id: string;
  kind: ResourceKind;
  nameRu: string;
  nameEn: string;
  modId: string;
  category: 'raw' | 'intermediate' | 'component' | 'fluid' | 'fuel' | 'product';
  color: string;
  visual?: ResourceVisualAssignment;
  userCreated?: boolean;
}

export type MachineSpeedMode = 'formula' | 'manual' | 'assumption' | 'unknown';

export interface MachineDef {
  id: string;
  nameRu: string;
  nameEn: string;
  modId: string;
  modVersion: string;
  stressImpactSU: number | null; // SU per RPM (Create standard: SU = impact * |RPM|)
  defaultRpm: number | null;
  maxRpm: number;
  supportsRpm: boolean;
  speedFormulaId:
    | 'millstone'
    | 'crushing_wheel'
    | 'mechanical_press'
    | 'mechanical_mixer'
    | 'mechanical_saw'
    | 'deployer'
    | 'spout'
    | 'item_drain'
    | 'encased_fan'
    | 'mechanical_crafter'
    | 'fixed_ticks'
    | 'unknown';
  speedFormulaNoteRu: string;
  speedFormulaNoteEn: string;
  sourceRef: string;
  visual?: ResourceVisualAssignment;
  userCreated?: boolean;
}

export interface RecipeIngredient {
  resourceId: string;
  kind: ResourceKind;
  amount: number;
  isTag?: boolean;
  tagId?: string;
}

export type OutputProbabilityKind = 'guaranteed' | 'bernoulli' | 'weighted';

export interface RecipeOutput {
  resourceId: string;
  kind: ResourceKind;
  amount: number;
  chance: number; // 0..1 effective probability per craft
  probabilityKind: OutputProbabilityKind;
  weight?: number;
}

export interface RecipeSourceMeta {
  origin: 'builtin' | 'imported-jar' | 'imported-json' | 'manual';
  modId: string;
  modVersion: string;
  minecraftVersion: string;
  loader: ModLoader;
  recipePath?: string;
}

export interface RecipeDef {
  id: string;
  type: string;
  machineId: string;
  nameRu: string;
  nameEn: string;
  inputs: RecipeIngredient[];
  outputs: RecipeOutput[];
  processingTimeTicks: number | null;
  heatRequirement: 'none' | 'heated' | 'superheated';
  loops?: number;
  source: RecipeSourceMeta;
  notesRu?: string;
  notesEn?: string;
}

export type NodeKind = 'source' | 'machine' | 'recipe' | 'byproduct' | 'target' | 'note';

export interface FactoryNode {
  id: string;
  kind: NodeKind;
  title: string;
  x: number;
  y: number;
  resourceId?: string;
  resourceKind?: ResourceKind;
  machineId?: string;
  recipeId?: string;
  sourceRatePerMin?: number;
  sourceUnlimited?: boolean;
  targetRatePerMin?: number;
  rpm?: number;
  speedMode?: MachineSpeedMode;
  manualCraftsPerMin?: number;
  stackSizeAssumption?: number;
  machineCountOverride?: number;
  noteText?: string;
  colorTag?: string;
}

export interface FactoryEdge {
  id: string;
  fromNodeId: string;
  toNodeId: string;
  resourceId: string;
  resourceKind: ResourceKind;
  rateMode: 'auto' | 'manual';
  manualRatePerMin?: number;
  isByproductPort?: boolean;
}

export interface FactoryProject {
  schemaVersion: 2;
  id: string;
  name: string;
  createdAt: string;
  updatedAt: string;
  profile: ModpackProfile;
  nodes: FactoryNode[];
  edges: FactoryEdge[];
  customResources: ResourceDef[];
  customMachines: MachineDef[];
  customRecipes: RecipeDef[];
}

export type WarningSeverity = 'error' | 'warning' | 'info';

export interface SolverWarning {
  id: string;
  severity: WarningSeverity;
  code:
    | 'CYCLE_DETECTED'
    | 'UNIT_MISMATCH'
    | 'RESOURCE_MISMATCH'
    | 'MISSING_RECIPE'
    | 'UNKNOWN_SPEED'
    | 'ASSUMPTION_ACTIVE'
    | 'DEFICIT'
    | 'BOTTLENECK'
    | 'PROBABILISTIC_OUTPUT'
    | 'DISCONNECTED_TARGET'
    | 'UNSUPPORTED_RECIPE';
  nodeId?: string;
  edgeId?: string;
  resourceId?: string;
  messageRu: string;
  messageEn: string;
  hintRu?: string;
  hintEn?: string;
}

export interface NodeFlowLine {
  resourceId: string;
  kind: ResourceKind;
  unit: FlowUnit;
  requiredPerMin: number;
  suppliedPerMin: number;
  producedExpectedPerMin: number;
  producedGuaranteedPerMin: number;
  probabilistic: boolean;
  isByproduct: boolean;
}

export interface NodeCalculation {
  nodeId: string;
  componentId: number;
  inCycle: boolean;
  requestedCraftsPerMin: number;
  effectiveCraftsPerMin: number;
  craftsPerMachinePerMin: number | null;
  speedSource: 'exact-formula' | 'manual' | 'assumption' | 'unknown' | 'not-applicable';
  requiredMachinesExact: number | null;
  requiredMachinesCeil: number | null;
  assignedMachines: number | null;
  utilizationRatio: number | null;
  stressTotalSU: number | null;
  isBottleneck: boolean;
  deficitDetected: boolean;
  inputs: NodeFlowLine[];
  outputs: NodeFlowLine[];
  warnings: string[]; // warning ids
}

export interface EdgeCalculation {
  edgeId: string;
  resourceId: string;
  resourceKind: ResourceKind;
  unit: FlowUnit;
  flowPerMin: number;
  demandPerMin: number;
  supplyPerMin: number;
  deficitPerMin: number;
  isUnitMismatch: boolean;
  isResourceMismatch: boolean;
  isBottleneck: boolean;
  inCycle: boolean;
}

export interface ResourceBalanceRow {
  resourceId: string;
  kind: ResourceKind;
  unit: FlowUnit;
  totalSourceSupplyPerMin: number;
  totalConsumedPerMin: number;
  totalProducedMainPerMin: number;
  totalProducedByproductPerMin: number;
  totalTargetDemandPerMin: number;
  netBalancePerMin: number; // positive = surplus, negative = deficit
  hasProbabilisticSupply: boolean;
}

export interface SolverStats {
  totalNodes: number;
  totalEdges: number;
  componentCount: number;
  recomputedComponents: number;
  cachedComponents: number;
  solveTimeMs: number;
  totalStressSU: number;
  totalMachinesRounded: number;
  cycleCount: number;
}

export interface SolverResult {
  nodeMap: Record<string, NodeCalculation>;
  edgeMap: Record<string, EdgeCalculation>;
  balances: ResourceBalanceRow[];
  warnings: SolverWarning[];
  cycles: string[][];
  stats: SolverStats;
}

export type TextureStatus = 'user-override' | 'temporary-original' | 'fallback';

export interface AssetManifestEntry {
  id: string;
  nameRu: string;
  nameEn: string;
  kind: 'item' | 'fluid' | 'machine' | 'block';
  modId: string;
  modVersion: string;
  sourceRepo: string;
  sourcePath: string;
  recommendedSize: string;
  license: string;
  status: TextureStatus;
  url: string;
  animatedStrip?: boolean;
  frameWidth?: number;
  frameHeight?: number;
  missingUpstreamReason?: string;
}

export interface JarImportReport {
  fileName: string;
  detectedModId: string;
  detectedModName: string;
  detectedModVersion: string;
  detectedLoader: ModLoader | 'unknown';
  detectedMinecraftVersion: string;
  archiveSizeBytes: number;
  entriesScanned: number;
  importedCount: number;
  skippedCount: number;
  extractedTexturesCount: number;
  recipes: RecipeDef[];
  skippedDetails: Array<{
    path: string;
    reasonCode: string;
    messageRu: string;
    messageEn: string;
  }>;
  warnings: string[];
}

export interface NbtBlockEntry {
  pos: [number, number, number];
  stateIndex: number;
  blockId: string;
  properties: Record<string, string>;
  hasBlockEntityNbt: boolean;
}

export interface NbtPaletteEntry {
  index: number;
  name: string;
  properties: Record<string, string>;
  count: number;
}

export interface ParsedSchematic {
  fileName: string;
  formatLabel: string;
  dataVersion: number | null;
  size: [number, number, number];
  totalPlacedBlocks: number;
  nonAirBlocks: number;
  entityCount: number;
  hasCreateEntities: boolean;
  palette: NbtPaletteEntry[];
  blocks: NbtBlockEntry[];
  warnings: string[];
  isSyntheticDemo?: boolean;
}
