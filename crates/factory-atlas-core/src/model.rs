use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    #[default]
    Ru,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    pub locale: Locale,
    #[serde(default)]
    pub theme: ThemeMode,
    #[serde(default = "default_rpm_setting")]
    pub default_rpm: f64,
    #[serde(default)]
    pub recent_projects: Vec<String>,
    #[serde(default)]
    pub local_assets_dir: Option<String>,
}

fn default_rpm_setting() -> f64 {
    64.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: Locale::Ru,
            theme: ThemeMode::Dark,
            default_rpm: 64.0,
            recent_projects: Vec::new(),
            local_assets_dir: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceKind {
    Item,
    Fluid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowUnit {
    #[serde(rename = "items/min")]
    ItemsPerMin,
    #[serde(rename = "mB/min")]
    MbPerMin,
}

impl FlowUnit {
    pub fn from_kind(kind: ResourceKind) -> Self {
        match kind {
            ResourceKind::Item => Self::ItemsPerMin,
            ResourceKind::Fluid => Self::MbPerMin,
        }
    }

    pub fn label(self, locale: Locale) -> &'static str {
        match (self, locale) {
            (Self::ItemsPerMin, Locale::Ru) => "предм./мин",
            (Self::ItemsPerMin, Locale::En) => "items/min",
            (Self::MbPerMin, Locale::Ru) => "мБ/мин",
            (Self::MbPerMin, Locale::En) => "mB/min",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModLoader {
    #[default]
    Neoforge,
    Forge,
    Fabric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonRef {
    pub id: String,
    pub name: String,
    pub version: String,
    pub verified_recipes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackProfile {
    pub id: String,
    pub name: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub create_version: String,
    #[serde(default)]
    pub addons: Vec<AddonRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModelSupportStatus {
    #[serde(rename = "3d-json-elements")]
    ThreeDJsonElements,
    #[serde(rename = "2d-item-generated")]
    TwoDItemGenerated,
    #[serde(rename = "unsupported-obj")]
    UnsupportedObj,
    #[serde(rename = "procedural-or-blockentity")]
    ProceduralOrBlockEntity,
    #[serde(rename = "none")]
    #[default]
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MatchReason {
    ExactId,
    ExactPath,
    Normalized,
    LangAlias,
    Fuzzy,
    Manual,
    #[default]
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceVisualAssignment {
    pub visual_id: Option<String>,
    pub model_ref: Option<String>,
    pub model_path: Option<String>,
    pub texture_path: Option<String>,
    #[serde(default)]
    pub model_support: ModelSupportStatus,
    pub custom_texture_override_path: Option<String>,
    pub custom_texture_override_data_url: Option<String>,
    pub custom_model_json: Option<String>,
    #[serde(default)]
    pub disabled: bool,
    pub match_reason: Option<MatchReason>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualSourceRepoMeta {
    pub namespace: String,
    pub repo: String,
    pub branch: String,
    pub commit: String,
    pub version: String,
    pub minecraft_version: String,
    pub create_version: String,
    pub code_license: String,
    pub assets_license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualEntrySummary {
    pub id: String,
    pub namespace: String,
    pub path_id: String,
    pub kind: String,
    pub name_en: String,
    pub name_ru: Option<String>,
    pub normalized_key: String,
    pub normalized_en: String,
    pub normalized_ru: String,
    pub source_repo: String,
    pub source_branch: String,
    pub source_commit: String,
    pub mod_version: String,
    pub minecraft_version: String,
    pub license: String,
    pub blockstate_path: Option<String>,
    pub model_ref: Option<String>,
    pub model_path: Option<String>,
    pub world_model_ref: Option<String>,
    pub world_model_path: Option<String>,
    pub texture_path: Option<String>,
    #[serde(default)]
    pub model_support: ModelSupportStatus,
    #[serde(default)]
    pub has_model_3d: bool,
    #[serde(default)]
    pub has_texture_2d: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisualIndexData {
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub sources: Vec<VisualSourceRepoMeta>,
    #[serde(default)]
    pub stats_by_namespace: HashMap<String, HashMap<String, usize>>,
    #[serde(default)]
    pub entries: Vec<VisualEntrySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDef {
    pub id: String,
    pub kind: ResourceKind,
    pub name_ru: String,
    pub name_en: String,
    pub mod_id: String,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_color")]
    pub color: String,
    pub visual: Option<ResourceVisualAssignment>,
    pub user_created: Option<bool>,
}

fn default_category() -> String {
    "intermediate".to_owned()
}
fn default_color() -> String {
    "#f59e0b".to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SpeedFormulaId {
    Millstone,
    MechanicalPress,
    MechanicalMixer,
    CrushingWheel,
    MechanicalSaw,
    Deployer,
    Spout,
    ItemDrain,
    EncasedFan,
    MechanicalCrafter,
    FixedTicks,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDef {
    pub id: String,
    pub name_ru: String,
    pub name_en: String,
    pub mod_id: String,
    pub mod_version: String,
    #[serde(rename = "stressImpactSU")]
    pub stress_impact_su: Option<f64>,
    pub default_rpm: Option<f64>,
    pub max_rpm: f64,
    pub supports_rpm: bool,
    #[serde(default)]
    pub speed_formula_id: SpeedFormulaId,
    pub speed_formula_note_ru: String,
    pub speed_formula_note_en: String,
    pub source_ref: String,
    pub visual: Option<ResourceVisualAssignment>,
    pub user_created: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ProbabilityKind {
    #[default]
    Guaranteed,
    Bernoulli,
    Weighted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeIngredient {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub amount: f64,
    pub tag_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeOutput {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub amount: f64,
    pub chance: f64,
    pub weight: Option<f64>,
    #[serde(default)]
    pub probability_kind: ProbabilityKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum RecipeOrigin {
    Builtin,
    ImportedJar,
    ImportedJson,
    #[default]
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeSourceMeta {
    pub origin: RecipeOrigin,
    pub mod_id: String,
    pub mod_version: String,
    pub minecraft_version: String,
    pub loader: ModLoader,
    pub recipe_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDef {
    pub id: String,
    #[serde(rename = "type")]
    pub recipe_type: String,
    pub machine_id: String,
    pub name_ru: String,
    pub name_en: String,
    pub inputs: Vec<RecipeIngredient>,
    pub outputs: Vec<RecipeOutput>,
    pub processing_time_ticks: Option<f64>,
    pub heat_requirement: Option<String>,
    pub source: RecipeSourceMeta,
    pub notes_ru: Option<String>,
    pub notes_en: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    Source,
    Machine,
    Recipe,
    Byproduct,
    Target,
    Note,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MachineSpeedMode {
    #[default]
    Formula,
    Manual,
    Assumption,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryNode {
    pub id: String,
    pub kind: NodeKind,
    pub title: String,
    pub x: f64,
    pub y: f64,
    pub resource_id: Option<String>,
    pub resource_kind: Option<ResourceKind>,
    pub machine_id: Option<String>,
    pub recipe_id: Option<String>,
    pub source_rate_per_min: Option<f64>,
    pub source_unlimited: Option<bool>,
    pub target_rate_per_min: Option<f64>,
    pub rpm: Option<f64>,
    pub speed_mode: Option<MachineSpeedMode>,
    pub manual_crafts_per_min: Option<f64>,
    pub stack_size_assumption: Option<f64>,
    pub machine_count_override: Option<u32>,
    pub note_text: Option<String>,
    pub color_tag: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RateMode {
    #[default]
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryEdge {
    pub id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub resource_id: String,
    pub resource_kind: ResourceKind,
    #[serde(default)]
    pub rate_mode: RateMode,
    pub manual_rate_per_min: Option<f64>,
    pub is_byproduct_port: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FactoryProject {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub profile: ModpackProfile,
    #[serde(default)]
    pub nodes: Vec<FactoryNode>,
    #[serde(default)]
    pub edges: Vec<FactoryEdge>,
    #[serde(default)]
    pub custom_resources: Vec<ResourceDef>,
    #[serde(default)]
    pub custom_machines: Vec<MachineDef>,
    #[serde(default)]
    pub custom_recipes: Vec<RecipeDef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WarningCode {
    CycleDetected,
    UnitMismatch,
    ResourceMismatch,
    MissingRecipe,
    UnknownSpeed,
    AssumptionActive,
    Deficit,
    Bottleneck,
    ProbabilisticOutput,
    DisconnectedTarget,
    UnsupportedRecipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverWarning {
    pub id: String,
    pub severity: WarningSeverity,
    pub code: WarningCode,
    pub node_id: Option<String>,
    pub edge_id: Option<String>,
    pub resource_id: Option<String>,
    pub message_ru: String,
    pub message_en: String,
    pub hint_ru: Option<String>,
    pub hint_en: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeFlowLine {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub unit: FlowUnit,
    pub required_per_min: f64,
    pub supplied_per_min: f64,
    pub produced_expected_per_min: f64,
    pub produced_guaranteed_per_min: f64,
    pub probabilistic: bool,
    pub is_byproduct: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeedSource {
    ExactFormula,
    Manual,
    Assumption,
    Unknown,
    NotApplicable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCalculation {
    pub node_id: String,
    pub component_id: usize,
    pub in_cycle: bool,
    pub requested_crafts_per_min: f64,
    pub effective_crafts_per_min: f64,
    pub crafts_per_machine_per_min: Option<f64>,
    pub speed_source: SpeedSource,
    pub required_machines_exact: Option<f64>,
    pub required_machines_ceil: Option<u32>,
    pub assigned_machines: Option<u32>,
    pub utilization_ratio: Option<f64>,
    #[serde(rename = "stressTotalSU")]
    pub stress_total_su: Option<f64>,
    pub is_bottleneck: bool,
    pub deficit_detected: bool,
    pub inputs: Vec<NodeFlowLine>,
    pub outputs: Vec<NodeFlowLine>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeCalculation {
    pub edge_id: String,
    pub resource_id: String,
    pub resource_kind: ResourceKind,
    pub unit: FlowUnit,
    pub flow_per_min: f64,
    pub demand_per_min: f64,
    pub supply_per_min: f64,
    pub deficit_per_min: f64,
    pub is_unit_mismatch: bool,
    pub is_resource_mismatch: bool,
    pub is_bottleneck: bool,
    pub in_cycle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceBalanceRow {
    pub resource_id: String,
    pub kind: ResourceKind,
    pub unit: FlowUnit,
    pub total_source_supply_per_min: f64,
    pub total_consumed_per_min: f64,
    pub total_produced_main_per_min: f64,
    pub total_produced_byproduct_per_min: f64,
    pub total_target_demand_per_min: f64,
    pub net_balance_per_min: f64,
    pub has_probabilistic_supply: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub component_count: usize,
    pub recomputed_components: usize,
    pub cached_components: usize,
    pub solve_time_ms: f64,
    #[serde(rename = "totalStressSU")]
    pub total_stress_su: f64,
    pub total_machines_rounded: u32,
    pub cycle_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverResult {
    pub node_map: HashMap<String, NodeCalculation>,
    pub edge_map: HashMap<String, EdgeCalculation>,
    pub balances: Vec<ResourceBalanceRow>,
    pub warnings: Vec<SolverWarning>,
    pub cycles: Vec<Vec<String>>,
    pub stats: SolverStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedImportDetail {
    pub path: String,
    pub reason_code: String,
    pub message_ru: String,
    pub message_en: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JarImportReport {
    pub file_name: String,
    pub detected_mod_id: String,
    pub detected_mod_name: String,
    pub detected_mod_version: String,
    pub detected_loader: String,
    pub detected_minecraft_version: String,
    pub archive_size_bytes: usize,
    pub entries_scanned: usize,
    pub imported_count: usize,
    pub skipped_count: usize,
    pub extracted_textures_count: usize,
    pub recipes: Vec<RecipeDef>,
    pub skipped_details: Vec<SkippedImportDetail>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NbtBlockEntry {
    pub pos: [i32; 3],
    pub state_index: usize,
    pub block_id: String,
    pub properties: HashMap<String, String>,
    pub has_block_entity_nbt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NbtPaletteEntry {
    pub index: usize,
    pub name: String,
    pub properties: HashMap<String, String>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSchematic {
    pub file_name: String,
    pub format_label: String,
    pub data_version: Option<i32>,
    pub size: [i32; 3],
    pub total_placed_blocks: usize,
    pub non_air_blocks: usize,
    pub entity_count: usize,
    pub has_create_entities: bool,
    pub palette: Vec<NbtPaletteEntry>,
    pub blocks: Vec<NbtBlockEntry>,
    pub warnings: Vec<String>,
    pub is_synthetic_demo: bool,
}
