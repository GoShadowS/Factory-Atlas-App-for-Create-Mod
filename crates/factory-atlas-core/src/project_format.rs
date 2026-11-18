use crate::model::{
    FactoryEdge, FactoryNode, FactoryProject, ModLoader, ModpackProfile, NodeKind, RateMode,
    ResourceKind,
};
use serde_json::{Map, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn default_profile() -> ModpackProfile {
    ModpackProfile {
        id: "custom-profile-1-21-1".to_owned(),
        name: "Create 6.0.11 (MC 1.21.1 · NeoForge)".to_owned(),
        minecraft_version: "1.21.1".to_owned(),
        loader: ModLoader::Neoforge,
        create_version: "6.0.11".to_owned(),
        addons: Vec::new(),
    }
}

pub fn preset_profiles() -> Vec<ModpackProfile> {
    vec![
        default_profile(),
        ModpackProfile {
            id: "create-only-1-21-1".to_owned(),
            name: "Только Create 6.0.11 (MC 1.21.1 · NeoForge)".to_owned(),
            minecraft_version: "1.21.1".to_owned(),
            loader: ModLoader::Neoforge,
            create_version: "6.0.11".to_owned(),
            addons: Vec::new(),
        },
        ModpackProfile {
            id: "create-fabric-1-20-1".to_owned(),
            name: "Create Fabric 0.5.1 (MC 1.20.1 · Fabric)".to_owned(),
            minecraft_version: "1.20.1".to_owned(),
            loader: ModLoader::Fabric,
            create_version: "0.5.1-j (Fabric)".to_owned(),
            addons: Vec::new(),
        },
    ]
}

pub fn now_iso_timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

/// Always produces a completely EMPTY project (0 nodes, 0 edges, 0 pre-filled
/// resources, 0 pre-filled machines, 0 pre-filled recipes).
pub fn create_empty_project(name: impl Into<String>) -> FactoryProject {
    let ts = now_iso_timestamp();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(1);
    FactoryProject {
        schema_version: 2,
        id: format!("proj-{nanos}"),
        name: name.into(),
        created_at: ts.clone(),
        updated_at: ts,
        profile: default_profile(),
        nodes: Vec::new(),
        edges: Vec::new(),
        custom_resources: Vec::new(),
        custom_machines: Vec::new(),
        custom_recipes: Vec::new(),
    }
}

#[derive(Debug, Clone)]
pub enum ProjectParseResult {
    Ok {
        project: FactoryProject,
        migrated_from_version: Option<u32>,
    },
    Err {
        error_ru: String,
        error_en: String,
    },
}

pub fn serialize_project(project: &FactoryProject) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(project)
}

pub fn parse_and_migrate_project(raw_json: &str) -> ProjectParseResult {
    let parsed: Value = match serde_json::from_str(raw_json) {
        Ok(v) => v,
        Err(_) => {
            return ProjectParseResult::Err {
                error_ru: "Файл проекта не является корректным JSON.".to_owned(),
                error_en: "Project file is not valid JSON.".to_owned(),
            };
        }
    };

    let Some(obj) = parsed.as_object() else {
        return ProjectParseResult::Err {
            error_ru: "Корневая структура файла проекта должна быть JSON-объектом.".to_owned(),
            error_en: "Project root must be a JSON object.".to_owned(),
        };
    };

    let schema_ver = obj
        .get("schemaVersion")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let (working_val, migrated_from) = match schema_ver {
        None | Some(1) => (migrate_v1_to_v2(obj), Some(1)),
        Some(2) => (Value::Object(obj.clone()), None),
        Some(other) => {
            return ProjectParseResult::Err {
                error_ru: format!(
                    "Неподдерживаемая версия формата схемы: {other}. Поддерживаются v1 и v2."
                ),
                error_en: format!("Unsupported project schema version: {other}."),
            };
        }
    };

    let working = working_val.as_object().expect("object");
    let Some(nodes_arr) = working.get("nodes").and_then(|v| v.as_array()) else {
        return ProjectParseResult::Err {
            error_ru: "В файле проекта отсутствуют обязательные массивы \"nodes\" или \"edges\"."
                .to_owned(),
            error_en: "Project file is missing required \"nodes\" or \"edges\" arrays.".to_owned(),
        };
    };
    let Some(edges_arr) = working.get("edges").and_then(|v| v.as_array()) else {
        return ProjectParseResult::Err {
            error_ru: "В файле проекта отсутствуют обязательные массивы \"nodes\" или \"edges\"."
                .to_owned(),
            error_en: "Project file is missing required \"nodes\" or \"edges\" arrays.".to_owned(),
        };
    };

    let nodes: Vec<FactoryNode> = nodes_arr
        .iter()
        .filter_map(|v| serde_json::from_value::<FactoryNode>(v.clone()).ok())
        .map(|mut n| {
            if !n.x.is_finite() {
                n.x = 100.0;
            }
            if !n.y.is_finite() {
                n.y = 100.0;
            }
            n
        })
        .collect();

    let edges: Vec<FactoryEdge> = edges_arr
        .iter()
        .filter_map(|v| serde_json::from_value::<FactoryEdge>(v.clone()).ok())
        .collect();

    let ts = now_iso_timestamp();
    let profile: ModpackProfile = working
        .get("profile")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_else(default_profile);

    let project = FactoryProject {
        schema_version: 2,
        id: working
            .get("id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "proj-imported".to_owned()),
        name: working
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "Без названия".to_owned()),
        created_at: working
            .get("createdAt")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| ts.clone()),
        updated_at: working
            .get("updatedAt")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned)
            .unwrap_or(ts),
        profile,
        nodes,
        edges,
        custom_resources: working
            .get("customResources")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        custom_machines: working
            .get("customMachines")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        custom_recipes: working
            .get("customRecipes")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
    };

    ProjectParseResult::Ok {
        project,
        migrated_from_version: migrated_from,
    }
}

fn migrate_v1_to_v2(v1: &Map<String, Value>) -> Value {
    let mut out = v1.clone();
    out.insert("schemaVersion".to_owned(), Value::from(2));

    if !out.contains_key("profile") {
        out.insert(
            "profile".to_owned(),
            serde_json::to_value(default_profile()).unwrap_or(Value::Null),
        );
    }

    if let Some(nodes) = out.get("nodes").and_then(|v| v.as_array()).cloned() {
        let mapped: Vec<Value> = nodes
            .into_iter()
            .map(|item| {
                let Some(mut obj) = item.as_object().cloned() else {
                    return item;
                };
                if !obj.contains_key("title")
                    && let Some(lbl) = obj.get("label").cloned()
                {
                    obj.insert("title".to_owned(), lbl);
                }
                let kind_str = obj.get("kind").and_then(|k| k.as_str()).unwrap_or("");
                if kind_str == "source"
                    && !obj.contains_key("sourceRatePerMin")
                    && let Some(r) = obj.get("rate").cloned()
                {
                    obj.insert("sourceRatePerMin".to_owned(), r);
                }
                if kind_str == "target"
                    && !obj.contains_key("targetRatePerMin")
                    && let Some(r) = obj.get("rate").cloned()
                {
                    obj.insert("targetRatePerMin".to_owned(), r);
                }
                if !obj.contains_key("resourceKind")
                    && let Some(unit) = obj.get("unit").and_then(|u| u.as_str())
                {
                    let rk = if unit.to_lowercase().contains("mb") {
                        ResourceKind::Fluid
                    } else {
                        ResourceKind::Item
                    };
                    obj.insert(
                        "resourceKind".to_owned(),
                        serde_json::to_value(rk).unwrap_or(Value::Null),
                    );
                }
                if obj.get("x").and_then(|v| v.as_f64()).is_none() {
                    obj.insert("x".to_owned(), Value::from(100.0));
                }
                if obj.get("y").and_then(|v| v.as_f64()).is_none() {
                    obj.insert("y".to_owned(), Value::from(100.0));
                }
                if obj
                    .get("kind")
                    .and_then(|k| serde_json::from_value::<NodeKind>(k.clone()).ok())
                    .is_none()
                {
                    obj.insert("kind".to_owned(), Value::from("note"));
                }
                Value::Object(obj)
            })
            .collect();
        out.insert("nodes".to_owned(), Value::Array(mapped));
    }

    if let Some(edges) = out.get("edges").and_then(|v| v.as_array()).cloned() {
        let mapped: Vec<Value> = edges
            .into_iter()
            .map(|item| {
                let Some(mut obj) = item.as_object().cloned() else {
                    return item;
                };
                if !obj.contains_key("fromNodeId")
                    && let Some(f) = obj.get("from").cloned()
                {
                    obj.insert("fromNodeId".to_owned(), f);
                }
                if !obj.contains_key("toNodeId")
                    && let Some(t) = obj.get("to").cloned()
                {
                    obj.insert("toNodeId".to_owned(), t);
                }
                if !obj.contains_key("resourceKind")
                    && let Some(unit) = obj.get("unit").and_then(|u| u.as_str())
                {
                    let rk = if unit.to_lowercase().contains("mb") {
                        ResourceKind::Fluid
                    } else {
                        ResourceKind::Item
                    };
                    obj.insert(
                        "resourceKind".to_owned(),
                        serde_json::to_value(rk).unwrap_or(Value::Null),
                    );
                }
                if !obj.contains_key("resourceId") {
                    obj.insert("resourceId".to_owned(), Value::from("custom:unknown"));
                }
                if !obj.contains_key("resourceKind") {
                    obj.insert("resourceKind".to_owned(), Value::from("item"));
                }
                if !obj.contains_key("rateMode") {
                    obj.insert(
                        "rateMode".to_owned(),
                        serde_json::to_value(RateMode::Auto).unwrap_or(Value::Null),
                    );
                }
                Value::Object(obj)
            })
            .collect();
        out.insert("edges".to_owned(), Value::Array(mapped));
    }

    Value::Object(out)
}
