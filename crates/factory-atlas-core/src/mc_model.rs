use crate::model::{ModelKind, VisualEntrySummary};
use crate::visual_matcher::normalize_visual_query;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FaceDir {
    North,
    South,
    East,
    West,
    Up,
    Down,
}

impl FaceDir {
    pub const ALL: [FaceDir; 6] = [
        FaceDir::North,
        FaceDir::South,
        FaceDir::East,
        FaceDir::West,
        FaceDir::Up,
        FaceDir::Down,
    ];

    pub fn parse_dir(s: &str) -> Option<Self> {
        match s {
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "east" => Some(Self::East),
            "west" => Some(Self::West),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            _ => None,
        }
    }

    pub fn normal(self) -> [f32; 3] {
        match self {
            FaceDir::North => [0.0, 0.0, -1.0],
            FaceDir::South => [0.0, 0.0, 1.0],
            FaceDir::East => [1.0, 0.0, 0.0],
            FaceDir::West => [-1.0, 0.0, 0.0],
            FaceDir::Up => [0.0, 1.0, 0.0],
            FaceDir::Down => [0.0, -1.0, 0.0],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelFace {
    pub uv: [f32; 4],
    pub texture: String,
    pub rotation: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRotation {
    pub origin: [f32; 3],
    pub axis: char,
    pub angle: f32,
    pub rescale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<ModelRotation>,
    pub faces: HashMap<FaceDir, ModelFace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartTransform {
    pub x_deg: f32,
    pub y_deg: f32,
    pub z_deg: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedModelPart {
    pub model_ref: String,
    pub transform: PartTransform,
    pub textures: HashMap<String, String>,
    pub elements: Vec<ModelElement>,
    pub is_2d_item: bool,
    pub unsupported_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedMcAsset {
    pub kind: ModelKind,
    pub parts: Vec<ResolvedModelPart>,
    pub fallback_texture: Option<String>,
    pub warning: Option<String>,
}

pub trait AssetSource {
    fn read_json(&self, rel_path: &str) -> Option<Value>;
    fn exists(&self, rel_path: &str) -> bool;
}

/// Safe filesystem asset reader confined strictly under `root_dir`.
pub struct DirAssetSource {
    pub root_dir: PathBuf,
}

impl DirAssetSource {
    fn safe_join(&self, rel_path: &str) -> Option<PathBuf> {
        let clean = rel_path.trim().trim_start_matches('/');
        let mut p = PathBuf::new();
        for comp in Path::new(clean).components() {
            match comp {
                Component::Normal(seg) => p.push(seg),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
            }
        }
        Some(self.root_dir.join(p))
    }
}

impl AssetSource for DirAssetSource {
    fn read_json(&self, rel_path: &str) -> Option<Value> {
        let p = self.safe_join(rel_path)?;
        let text = fs::read_to_string(p).ok()?;
        serde_json::from_str(&text).ok()
    }

    fn exists(&self, rel_path: &str) -> bool {
        self.safe_join(rel_path).is_some_and(|p| p.is_file())
    }
}

pub fn normalize_namespaced_ref(
    raw_ref: &str,
    kind: &str,
    default_ns: &str,
) -> Option<(String, String, String)> {
    let trimmed = raw_ref.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (ns, rest) = if let Some((a, b)) = trimmed.split_once(':') {
        (a.trim().to_lowercase(), b.trim())
    } else {
        (default_ns.to_lowercase(), trimmed)
    };
    let clean_rest = rest
        .trim_start_matches('/')
        .trim_end_matches(".json")
        .trim_end_matches(".png");
    let rel_path = match kind {
        "model" => format!("{ns}/models/{clean_rest}.json"),
        "texture" => format!("{ns}/textures/{clean_rest}.png"),
        "blockstate" => format!("{ns}/blockstates/{clean_rest}.json"),
        _ => format!("{ns}/{clean_rest}"),
    };
    Some((ns, clean_rest.to_owned(), rel_path))
}

pub fn resolve_texture_var(
    raw_tex: &str,
    textures: &HashMap<String, String>,
    max_depth: usize,
) -> Option<String> {
    let mut cur = raw_tex.trim().to_owned();
    let mut visited = HashSet::new();
    for _ in 0..max_depth {
        if !cur.starts_with('#') {
            return Some(cur);
        }
        let key = cur.trim_start_matches('#').to_owned();
        if !visited.insert(key.clone()) {
            return None;
        }
        cur = textures.get(&key)?.clone();
    }
    None
}

fn vanilla_primitive_elements(parent_norm: &str) -> Option<Vec<ModelElement>> {
    let is_cube_all = parent_norm.ends_with("/block/cube_all");
    let is_cube_col = parent_norm.ends_with("/block/cube_column")
        || parent_norm.ends_with("/block/cube_column_horizontal");
    let is_cube = parent_norm.ends_with("/block/cube")
        || parent_norm.ends_with("/block/orientable")
        || parent_norm.ends_with("/block/orientable_with_bottom");

    if is_cube_all || is_cube_col || is_cube {
        let mut faces = HashMap::new();
        for dir in FaceDir::ALL {
            let tex = if is_cube_all {
                "#all"
            } else if is_cube_col {
                if matches!(dir, FaceDir::Up | FaceDir::Down) {
                    "#end"
                } else {
                    "#side"
                }
            } else {
                match dir {
                    FaceDir::North => "#north",
                    FaceDir::South => "#south",
                    FaceDir::East => "#east",
                    FaceDir::West => "#west",
                    FaceDir::Up => "#up",
                    FaceDir::Down => "#down",
                }
            };
            faces.insert(
                dir,
                ModelFace {
                    uv: [0.0, 0.0, 16.0, 16.0],
                    texture: tex.to_owned(),
                    rotation: 0.0,
                },
            );
        }
        return Some(vec![ModelElement {
            from: [0.0, 0.0, 0.0],
            to: [16.0, 16.0, 16.0],
            rotation: None,
            faces,
        }]);
    }

    if parent_norm.ends_with("/block/cross") || parent_norm.ends_with("/block/tinted_cross") {
        let mut f1 = HashMap::new();
        f1.insert(
            FaceDir::North,
            ModelFace {
                uv: [0.0, 0.0, 16.0, 16.0],
                texture: "#cross".to_owned(),
                rotation: 0.0,
            },
        );
        f1.insert(
            FaceDir::South,
            ModelFace {
                uv: [0.0, 0.0, 16.0, 16.0],
                texture: "#cross".to_owned(),
                rotation: 0.0,
            },
        );
        return Some(vec![
            ModelElement {
                from: [0.0, 0.0, 8.0],
                to: [16.0, 16.0, 8.0],
                rotation: Some(ModelRotation {
                    origin: [8.0, 8.0, 8.0],
                    axis: 'y',
                    angle: 45.0,
                    rescale: false,
                }),
                faces: f1.clone(),
            },
            ModelElement {
                from: [0.0, 0.0, 8.0],
                to: [16.0, 16.0, 8.0],
                rotation: Some(ModelRotation {
                    origin: [8.0, 8.0, 8.0],
                    axis: 'y',
                    angle: -45.0,
                    rescale: false,
                }),
                faces: f1,
            },
        ]);
    }

    None
}

fn parse_vec3(v: Option<&Value>, default: [f32; 3]) -> [f32; 3] {
    let Some(arr) = v.and_then(|x| x.as_array()) else {
        return default;
    };
    if arr.len() < 3 {
        return default;
    }
    [
        arr[0].as_f64().unwrap_or(f64::from(default[0])) as f32,
        arr[1].as_f64().unwrap_or(f64::from(default[1])) as f32,
        arr[2].as_f64().unwrap_or(f64::from(default[2])) as f32,
    ]
}

fn parse_elements(val: &Value) -> Vec<ModelElement> {
    let Some(arr) = val.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let from = parse_vec3(obj.get("from"), [0.0, 0.0, 0.0]);
        let to = parse_vec3(obj.get("to"), [16.0, 16.0, 16.0]);
        let rotation = obj.get("rotation").and_then(|r| r.as_object()).map(|robj| {
            let axis = robj
                .get("axis")
                .and_then(|a| a.as_str())
                .and_then(|s| s.chars().next())
                .unwrap_or('y');
            ModelRotation {
                origin: parse_vec3(robj.get("origin"), [8.0, 8.0, 8.0]),
                axis,
                angle: robj.get("angle").and_then(|a| a.as_f64()).unwrap_or(0.0) as f32,
                rescale: robj
                    .get("rescale")
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false),
            }
        });
        let mut faces = HashMap::new();
        if let Some(fobj) = obj.get("faces").and_then(|f| f.as_object()) {
            for (k, fv) in fobj {
                let Some(dir) = FaceDir::parse_dir(k) else {
                    continue;
                };
                let Some(fmap) = fv.as_object() else {
                    continue;
                };
                let uv = fmap
                    .get("uv")
                    .and_then(|u| u.as_array())
                    .filter(|a| a.len() >= 4)
                    .map(|a| {
                        [
                            a[0].as_f64().unwrap_or(0.0) as f32,
                            a[1].as_f64().unwrap_or(0.0) as f32,
                            a[2].as_f64().unwrap_or(16.0) as f32,
                            a[3].as_f64().unwrap_or(16.0) as f32,
                        ]
                    })
                    .unwrap_or([0.0, 0.0, 16.0, 16.0]);
                let tex = fmap
                    .get("texture")
                    .and_then(|t| t.as_str())
                    .unwrap_or("#missing")
                    .to_owned();
                let rot = fmap
                    .get("rotation")
                    .and_then(|r| r.as_f64())
                    .unwrap_or(0.0) as f32;
                faces.insert(
                    dir,
                    ModelFace {
                        uv,
                        texture: tex,
                        rotation: rot,
                    },
                );
            }
        }
        out.push(ModelElement {
            from,
            to,
            rotation,
            faces,
        });
    }
    out
}

pub fn resolve_model_part<S: AssetSource>(
    model_ref: &str,
    default_ns: &str,
    transform: PartTransform,
    source: &S,
) -> ResolvedModelPart {
    let mut chain: Vec<(String, Value)> = Vec::new();
    let mut visited = HashSet::new();
    let mut cur_ref = Some(model_ref.to_owned());
    let mut vanilla_elements: Option<Vec<ModelElement>> = None;
    let mut is_2d_item = false;
    let mut unsupported_reason: Option<String> = None;

    for _ in 0..24 {
        let Some(r) = cur_ref.take() else {
            break;
        };
        let Some((_ns, clean, rel)) = normalize_namespaced_ref(&r, "model", default_ns) else {
            break;
        };
        if !visited.insert(rel.clone()) {
            break;
        }
        if rel.starts_with("minecraft/models/item/generated")
            || rel.starts_with("minecraft/models/item/handheld")
            || clean == "builtin/generated"
        {
            is_2d_item = true;
            break;
        }
        if let Some(prim) = vanilla_primitive_elements(&rel) {
            vanilla_elements = Some(prim);
            break;
        }
        if let Some(json) = source.read_json(&rel) {
            if json.get("loader").and_then(|l| l.as_str()).is_some_and(|l| l.contains("obj"))
                || json.get("model").and_then(|m| m.as_str()).is_some_and(|m| m.ends_with(".obj"))
            {
                unsupported_reason = Some("OBJ model loader requires external mesh".to_owned());
            }
            cur_ref = json
                .get("parent")
                .and_then(|p| p.as_str())
                .map(|s| s.to_owned());
            chain.push((rel, json));
        } else {
            break;
        }
    }

    let mut textures: HashMap<String, String> = HashMap::new();
    let mut elements: Vec<ModelElement> = vanilla_elements.unwrap_or_default();

    // Apply root -> leaf
    for (_, json) in chain.iter().rev() {
        if let Some(tobj) = json.get("textures").and_then(|t| t.as_object()) {
            for (k, v) in tobj {
                if let Some(vs) = v.as_str() {
                    textures.insert(k.clone(), vs.to_owned());
                }
            }
        }
        if let Some(elems_val) = json.get("elements") {
            let parsed = parse_elements(elems_val);
            if !parsed.is_empty() {
                elements = parsed;
            }
        }
    }

    // Resolve face texture variables to relative PNG paths
    let resolved_textures: HashMap<String, String> = textures
        .iter()
        .filter_map(|(k, v)| {
            let final_ref = resolve_texture_var(v, &textures, 16)?;
            let (_, _, tex_rel) = normalize_namespaced_ref(&final_ref, "texture", default_ns)?;
            Some((k.clone(), tex_rel))
        })
        .collect();

    for el in &mut elements {
        for face in el.faces.values_mut() {
            if face.texture.starts_with('#')
                && let Some(resolved) = resolve_texture_var(&face.texture, &textures, 16)
                && let Some((_, _, tex_rel)) =
                    normalize_namespaced_ref(&resolved, "texture", default_ns)
            {
                face.texture = tex_rel;
            }
        }
    }

    ResolvedModelPart {
        model_ref: model_ref.to_owned(),
        transform,
        textures: resolved_textures,
        elements,
        is_2d_item,
        unsupported_reason,
    }
}

pub fn resolve_visual_entry_asset<S: AssetSource>(
    entry: &VisualEntrySummary,
    source: &S,
) -> ResolvedMcAsset {
    if let Some(bs_path) = entry.blockstate_path.as_deref()
        && let Some(bs_json) = source.read_json(bs_path)
    {
        let mut parts: Vec<ResolvedModelPart> = Vec::new();
        if let Some(variants) = bs_json.get("variants").and_then(|v| v.as_object()) {
            let picked = variants.get("").or_else(|| variants.values().next());
            let first = picked.and_then(|v| {
                if v.is_array() {
                    v.as_array().and_then(|a| a.first())
                } else {
                    Some(v)
                }
            });
            if let Some(obj) = first.and_then(|v| v.as_object())
                && let Some(mref) = obj.get("model").and_then(|m| m.as_str())
            {
                let tr = PartTransform {
                    x_deg: obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    y_deg: obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                    z_deg: obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                };
                parts.push(resolve_model_part(mref, &entry.namespace, tr, source));
            }
        } else if let Some(mp) = bs_json.get("multipart").and_then(|m| m.as_array()) {
            for rule in mp {
                if let Some(apply) = rule.get("apply") {
                    let item = if apply.is_array() {
                        apply.as_array().and_then(|a| a.first())
                    } else {
                        Some(apply)
                    };
                    if let Some(obj) = item.and_then(|v| v.as_object())
                        && let Some(mref) = obj.get("model").and_then(|m| m.as_str())
                    {
                        let tr = PartTransform {
                            x_deg: obj.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            y_deg: obj.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            z_deg: obj.get("z").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                        };
                        parts.push(resolve_model_part(mref, &entry.namespace, tr, source));
                    }
                }
            }
        }

        let elem_count: usize = parts.iter().map(|p| p.elements.len()).sum();
        if elem_count > 0 {
            return ResolvedMcAsset {
                kind: ModelKind::JsonElements3d,
                parts,
                fallback_texture: entry.texture_path.clone(),
                warning: None,
            };
        }
    }

    if let Some(mref) = entry
        .model_ref
        .as_deref()
        .or(entry.world_model_path.as_deref())
        .or(entry.model_path.as_deref())
    {
        let part = resolve_model_part(
            mref,
            &entry.namespace,
            PartTransform {
                x_deg: 0.0,
                y_deg: 0.0,
                z_deg: 0.0,
            },
            source,
        );
        if let Some(reason) = part.unsupported_reason.clone() {
            return ResolvedMcAsset {
                kind: ModelKind::UnsupportedObj,
                parts: vec![part],
                fallback_texture: entry.texture_path.clone(),
                warning: Some(reason),
            };
        }
        if !part.elements.is_empty() {
            return ResolvedMcAsset {
                kind: ModelKind::JsonElements3d,
                parts: vec![part],
                fallback_texture: entry.texture_path.clone(),
                warning: None,
            };
        }
        if part.is_2d_item {
            return ResolvedMcAsset {
                kind: ModelKind::ItemGenerated2d,
                parts: vec![part],
                fallback_texture: entry.texture_path.clone(),
                warning: None,
            };
        }
    }

    if entry.texture_path.is_some() {
        return ResolvedMcAsset {
            kind: ModelKind::ItemGenerated2d,
            parts: Vec::new(),
            fallback_texture: entry.texture_path.clone(),
            warning: None,
        };
    }

    ResolvedMcAsset {
        kind: if entry.model_kind == ModelKind::None {
            ModelKind::ProceduralOrBlockEntity
        } else {
            entry.model_kind
        },
        parts: Vec::new(),
        fallback_texture: None,
        warning: Some("No static JSON geometry elements available.".to_owned()),
    }
}

pub struct BoundedModelCache {
    entries: HashMap<String, ResolvedMcAsset>,
    order: VecDeque<String>,
    max_entries: usize,
}

impl Default for BoundedModelCache {
    fn default() -> Self {
        Self::new(128)
    }
}

impl BoundedModelCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            max_entries: max_entries.max(1),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get_or_resolve<S: AssetSource>(
        &mut self,
        entry: &VisualEntrySummary,
        source: &S,
    ) -> &ResolvedMcAsset {
        if self.entries.contains_key(&entry.id) {
            self.order.retain(|k| k != &entry.id);
            self.order.push_back(entry.id.clone());
            return &self.entries[&entry.id];
        }
        let resolved = resolve_visual_entry_asset(entry, source);
        if self.entries.len() >= self.max_entries
            && let Some(oldest) = self.order.pop_front()
        {
            self.entries.remove(&oldest);
        }
        self.order.push_back(entry.id.clone());
        self.entries.insert(entry.id.clone(), resolved);
        &self.entries[&entry.id]
    }
}

#[derive(Debug, Deserialize)]
struct RawVisualIndexFile {
    #[serde(default)]
    entries: Vec<VisualEntrySummary>,
}

/// Loads `visual-index.json` if present, or returns empty list cleanly when offline/without assets.
pub fn load_visual_index_file(path: &Path) -> Vec<VisualEntrySummary> {
    let Ok(bytes) = fs::read_to_string(path) else {
        return Vec::new();
    };
    if let Ok(wrapped) = serde_json::from_str::<RawVisualIndexFile>(&bytes)
        && !wrapped.entries.is_empty()
    {
        return wrapped.entries;
    }
    serde_json::from_str::<Vec<VisualEntrySummary>>(&bytes).unwrap_or_default()
}

/// Scans an arbitrary local directory containing `<namespace>/{models,textures,blockstates,lang}`
/// (like `assets-local/upstream-originals` or `assets-overrides`) into `VisualEntrySummary` rows.
/// Read-only index only — never creates factory items or recipes.
pub fn build_local_visual_index_from_dir(root_dir: &Path) -> Vec<VisualEntrySummary> {
    let Ok(read_ns) = fs::read_dir(root_dir) else {
        return Vec::new();
    };
    let source = DirAssetSource {
        root_dir: root_dir.to_path_buf(),
    };
    let mut entries: Vec<VisualEntrySummary> = Vec::new();

    for ns_ent in read_ns.flatten() {
        if !ns_ent.file_type().is_ok_and(|ft| ft.is_dir()) {
            continue;
        }
        let ns = ns_ent.file_name().to_string_lossy().to_string();
        if ns.starts_with('.') {
            continue;
        }
        let ru_lang = load_lang_map(&ns_ent.path().join("lang/ru_ru.json"));
        let en_lang = load_lang_map(&ns_ent.path().join("lang/en_us.json"));

        let mut seen_ids: HashMap<String, VisualEntrySummary> = HashMap::new();
        for kind_dir in [
            "models/item",
            "models/block",
            "blockstates",
            "textures/item",
            "textures/block",
        ] {
            let dir_path = ns_ent.path().join(kind_dir);
            collect_files_recursive(&dir_path, &dir_path, &mut |rel_sub| {
                let stem = rel_sub
                    .trim_end_matches(".json")
                    .trim_end_matches(".png")
                    .to_owned();
                if stem.is_empty() {
                    return;
                }
                let id = format!("{ns}:{stem}");
                let e = seen_ids.entry(id.clone()).or_insert_with(|| {
                    let key_item = format!("item.{ns}.{}", stem.replace('/', "."));
                    let key_block = format!("block.{ns}.{}", stem.replace('/', "."));
                    let name_en = en_lang
                        .get(&key_item)
                        .or_else(|| en_lang.get(&key_block))
                        .cloned()
                        .unwrap_or_else(|| humanize_stem(&stem));
                    let name_ru = ru_lang
                        .get(&key_item)
                        .or_else(|| ru_lang.get(&key_block))
                        .cloned();
                    VisualEntrySummary {
                        id: id.clone(),
                        namespace: ns.clone(),
                        path_id: stem.clone(),
                        name_en: name_en.clone(),
                        name_ru: name_ru.clone(),
                        normalized_key: normalize_visual_query(&id),
                        normalized_en: normalize_visual_query(&name_en),
                        normalized_ru: name_ru
                            .as_deref()
                            .map(normalize_visual_query)
                            .unwrap_or_default(),
                        model_path: None,
                        world_model_path: None,
                        blockstate_path: None,
                        texture_path: None,
                        model_ref: None,
                        has_model_3d: false,
                        model_kind: ModelKind::None,
                    }
                });

                let full_rel = format!("{ns}/{kind_dir}/{rel_sub}");
                match kind_dir {
                    "models/item" => {
                        e.model_path = Some(full_rel.clone());
                        if e.model_ref.is_none() {
                            e.model_ref = Some(format!("{ns}:item/{stem}"));
                        }
                    }
                    "models/block" => {
                        e.world_model_path = Some(full_rel);
                        e.model_ref = Some(format!("{ns}:block/{stem}"));
                    }
                    "blockstates" => {
                        e.blockstate_path = Some(full_rel);
                    }
                    "textures/item" => {
                        e.texture_path = Some(full_rel);
                    }
                    "textures/block" if e.texture_path.is_none() => {
                        e.texture_path = Some(full_rel);
                    }
                    _ => {}
                }
            });
        }

        for mut entry in seen_ids.into_values() {
            let resolved = resolve_visual_entry_asset(&entry, &source);
            entry.model_kind = resolved.kind;
            entry.has_model_3d = resolved.kind == ModelKind::JsonElements3d;
            if entry.texture_path.is_none() {
                entry.texture_path = resolved
                    .parts
                    .iter()
                    .flat_map(|p| p.textures.values())
                    .find(|tex| source.exists(tex))
                    .cloned();
            }
            entries.push(entry);
        }
    }

    entries.sort_by(|a, b| a.id.cmp(&b.id));
    entries
}

fn collect_files_recursive(base: &Path, cur: &Path, cb: &mut dyn FnMut(&str)) {
    let Ok(rd) = fs::read_dir(cur) else {
        return;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if ent.file_type().is_ok_and(|ft| ft.is_dir()) {
            collect_files_recursive(base, &p, cb);
        } else if ent.file_type().is_ok_and(|ft| ft.is_file())
            && let Ok(rel) = p.strip_prefix(base)
        {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if rel_str.ends_with(".json") || rel_str.ends_with(".png") {
                cb(&rel_str);
            }
        }
    }
}

fn load_lang_map(path: &Path) -> HashMap<String, String> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn humanize_stem(stem: &str) -> String {
    let tail = stem.rsplit('/').next().unwrap_or(stem);
    tail.split('_')
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
