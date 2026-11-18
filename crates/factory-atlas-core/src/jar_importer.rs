use crate::model::{
    JarImportReport, ModLoader, ProbabilityKind, RecipeDef, RecipeIngredient, RecipeOrigin,
    RecipeOutput, RecipeSourceMeta, ResourceKind, SkippedImportDetail,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use zip::ZipArchive;

const MAX_JAR_ENTRY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_EXTRACT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JarImportOptions {
    pub extract_visual_assets_to: Option<PathBuf>,
    pub import_recipes: bool,
    pub max_recipes: usize,
}

impl Default for JarImportOptions {
    fn default() -> Self {
        Self {
            extract_visual_assets_to: None,
            import_recipes: true,
            max_recipes: 5000,
        }
    }
}

fn sanitize_zip_rel_path(name: &str) -> Option<PathBuf> {
    let mut clean = PathBuf::new();
    for comp in Path::new(name).components() {
        match comp {
            Component::Normal(seg) => clean.push(seg),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

fn map_create_recipe_type_to_machine(raw_type: &str) -> (&'static str, Option<f64>) {
    match raw_type {
        "create:crushing" => ("create:crushing_wheel", Some(120.0)),
        "create:milling" => ("create:millstone", Some(100.0)),
        "create:mixing" => ("create:mechanical_mixer", Some(100.0)),
        "create:compacting" => ("create:mechanical_press", Some(80.0)),
        "create:pressing" => ("create:mechanical_press", Some(60.0)),
        "create:cutting" => ("create:mechanical_saw", Some(80.0)),
        "create:deploying" => ("create:deployer", Some(20.0)),
        "create:filling" => ("create:spout", Some(20.0)),
        "create:emptying" => ("create:item_drain", Some(20.0)),
        "create:splashing" | "create:haunting" => ("create:encased_fan", Some(150.0)),
        "create:mechanical_crafting" => ("create:mechanical_crafter", Some(60.0)),
        "minecraft:smelting" | "minecraft:blasting" | "minecraft:smoking" => {
            ("create:encased_fan", Some(150.0))
        }
        "minecraft:crafting_shaped" | "minecraft:crafting_shapeless" => {
            ("create:manual_craft", Some(20.0))
        }
        _ => ("create:custom_machine", None),
    }
}

fn parse_ingredient_val(v: &Value) -> Option<RecipeIngredient> {
    let obj = v.as_object()?;
    if let Some(fluid) = obj.get("fluid").and_then(|f| f.as_str()) {
        let amt = obj.get("amount").and_then(|a| a.as_f64()).unwrap_or(1000.0);
        return Some(RecipeIngredient {
            resource_id: fluid.to_owned(),
            kind: ResourceKind::Fluid,
            amount: amt,
            tag_name: None,
        });
    }
    if let Some(fluid_tag) = obj.get("fluidTag").and_then(|f| f.as_str()) {
        let amt = obj.get("amount").and_then(|a| a.as_f64()).unwrap_or(1000.0);
        return Some(RecipeIngredient {
            resource_id: format!("#{fluid_tag}"),
            kind: ResourceKind::Fluid,
            amount: amt,
            tag_name: Some(fluid_tag.to_owned()),
        });
    }
    let tag_name = obj
        .get("tag")
        .and_then(|t| t.as_str())
        .map(|s| s.to_owned());
    let item_id = obj
        .get("item")
        .or_else(|| obj.get("id"))
        .and_then(|s| s.as_str())
        .map(|s| s.to_owned())
        .or_else(|| tag_name.as_ref().map(|t| format!("#{t}")))?;
    let amt = obj.get("count").and_then(|c| c.as_f64()).unwrap_or(1.0);
    Some(RecipeIngredient {
        resource_id: item_id,
        kind: ResourceKind::Item,
        amount: amt,
        tag_name,
    })
}

fn parse_output_val(v: &Value) -> Option<RecipeOutput> {
    let obj = v.as_object()?;
    if let Some(fluid) = obj.get("fluid").and_then(|f| f.as_str()) {
        let amt = obj.get("amount").and_then(|a| a.as_f64()).unwrap_or(1000.0);
        let chance = obj.get("chance").and_then(|c| c.as_f64()).unwrap_or(1.0);
        return Some(RecipeOutput {
            resource_id: fluid.to_owned(),
            kind: ResourceKind::Fluid,
            amount: amt,
            chance: chance.clamp(0.0, 1.0),
            weight: None,
            probability_kind: if chance < 0.9999 {
                ProbabilityKind::Bernoulli
            } else {
                ProbabilityKind::Guaranteed
            },
        });
    }
    let item_id = obj
        .get("item")
        .or_else(|| obj.get("id"))
        .and_then(|s| s.as_str())?
        .to_owned();
    let amt = obj.get("count").and_then(|c| c.as_f64()).unwrap_or(1.0);
    let chance = obj.get("chance").and_then(|c| c.as_f64()).unwrap_or(1.0);
    Some(RecipeOutput {
        resource_id: item_id,
        kind: ResourceKind::Item,
        amount: amt,
        chance: chance.clamp(0.0, 1.0),
        weight: None,
        probability_kind: if chance < 0.9999 {
            ProbabilityKind::Bernoulli
        } else {
            ProbabilityKind::Guaranteed
        },
    })
}

pub fn parse_mc_recipe_json(
    recipe_id: &str,
    mod_id: &str,
    recipe_path: &str,
    json: &Value,
    lang_en: &HashMap<String, String>,
    lang_ru: &HashMap<String, String>,
) -> Option<RecipeDef> {
    let obj = json.as_object()?;
    let rtype = obj.get("type").and_then(|t| t.as_str())?;
    let (machine_id, default_ticks) = map_create_recipe_type_to_machine(rtype);
    let processing_time_ticks = obj
        .get("processingTime")
        .or_else(|| obj.get("processing_time"))
        .or_else(|| obj.get("cookingtime"))
        .and_then(|v| v.as_f64())
        .or(default_ticks);

    let mut inputs: Vec<RecipeIngredient> = Vec::new();
    if let Some(arr) = obj.get("ingredients").and_then(|v| v.as_array()) {
        for item in arr {
            let picked = if item.is_array() {
                item.as_array().and_then(|a| a.first())
            } else {
                Some(item)
            };
            if let Some(ing) = picked.and_then(parse_ingredient_val) {
                inputs.push(ing);
            }
        }
    } else if let Some(ing) = obj.get("ingredient").and_then(parse_ingredient_val) {
        inputs.push(ing);
    } else if let Some(key_map) = obj.get("key").and_then(|k| k.as_object()) {
        let mut counts: HashMap<String, (ResourceKind, f64, Option<String>)> = HashMap::new();
        if let Some(pat) = obj.get("pattern").and_then(|p| p.as_array()) {
            for row in pat {
                if let Some(rs) = row.as_str() {
                    for ch in rs.chars() {
                        if ch == ' ' {
                            continue;
                        }
                        if let Some(v) = key_map.get(&ch.to_string())
                            && let Some(ing) = parse_ingredient_val(v)
                        {
                            let e = counts
                                .entry(ing.resource_id)
                                .or_insert((ing.kind, 0.0, ing.tag_name));
                            e.1 += ing.amount;
                        }
                    }
                }
            }
        }
        for (rid, (kind, amt, tag_name)) in counts {
            inputs.push(RecipeIngredient {
                resource_id: rid,
                kind,
                amount: amt,
                tag_name,
            });
        }
    }

    let mut outputs: Vec<RecipeOutput> = Vec::new();
    if let Some(arr) = obj.get("results").and_then(|r| r.as_array()) {
        for o in arr {
            if let Some(out) = parse_output_val(o) {
                outputs.push(out);
            }
        }
    } else if let Some(out) = obj.get("result").and_then(parse_output_val) {
        outputs.push(out);
    } else if let Some(res_str) = obj.get("result").and_then(|r| r.as_str()) {
        let cnt = obj.get("count").and_then(|c| c.as_f64()).unwrap_or(1.0);
        outputs.push(RecipeOutput {
            resource_id: res_str.to_owned(),
            kind: ResourceKind::Item,
            amount: cnt,
            chance: 1.0,
            weight: None,
            probability_kind: ProbabilityKind::Guaranteed,
        });
    }

    if inputs.is_empty() && outputs.is_empty() {
        return None;
    }

    let name_en = outputs
        .first()
        .and_then(|o| lookup_lang_name(&o.resource_id, lang_en))
        .unwrap_or_else(|| recipe_id.to_owned());
    let name_ru = outputs
        .first()
        .and_then(|o| lookup_lang_name(&o.resource_id, lang_ru))
        .unwrap_or_else(|| name_en.clone());

    Some(RecipeDef {
        id: recipe_id.to_owned(),
        recipe_type: rtype.to_owned(),
        machine_id: machine_id.to_owned(),
        name_ru,
        name_en,
        inputs,
        outputs,
        processing_time_ticks,
        heat_requirement: obj
            .get("heatRequirement")
            .and_then(|h| h.as_str())
            .map(|s| s.to_owned()),
        source: RecipeSourceMeta {
            origin: RecipeOrigin::ImportedJar,
            mod_id: mod_id.to_owned(),
            mod_version: "imported".to_owned(),
            minecraft_version: "1.21.1".to_owned(),
            loader: ModLoader::Neoforge,
            recipe_path: Some(recipe_path.to_owned()),
        },
        notes_ru: Some(format!("Импортировано из рецепта {rtype}")),
        notes_en: Some(format!("Imported from recipe type {rtype}")),
    })
}

fn lookup_lang_name(id: &str, map: &HashMap<String, String>) -> Option<String> {
    let (ns, rest) = id.split_once(':').unwrap_or(("minecraft", id));
    let k1 = format!("item.{ns}.{}", rest.replace('/', "."));
    let k2 = format!("block.{ns}.{}", rest.replace('/', "."));
    let k3 = format!("fluid.{ns}.{}", rest.replace('/', "."));
    map.get(&k1).or_else(|| map.get(&k2)).or_else(|| map.get(&k3)).cloned()
}

pub fn import_mod_archive_reader<R: Read + Seek>(
    archive_name: &str,
    archive_size_bytes: usize,
    reader: R,
    options: &JarImportOptions,
    cancel: Option<&AtomicBool>,
    mut on_progress: impl FnMut(usize, usize, &str),
) -> Result<JarImportReport, String> {
    let mut zip = ZipArchive::new(reader).map_err(|e| format!("Failed to open archive: {e}"))?;
    let total = zip.len();
    let mut namespaces: BTreeSet<String> = BTreeSet::new();
    let mut lang_en: HashMap<String, String> = HashMap::new();
    let mut lang_ru: HashMap<String, String> = HashMap::new();
    let mut raw_recipes: Vec<(String, String, String, Value)> = Vec::new();
    let mut extracted_textures_count = 0usize;
    let mut total_extracted_bytes = 0u64;
    let mut warnings = Vec::new();
    let mut skipped_details = Vec::new();

    for idx in 0..total {
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            warnings.push("Import cancelled by user".to_owned());
            break;
        }
        if idx % 32 == 0 {
            on_progress(idx, total, "Scanning archive entries");
        }

        let mut file = match zip.by_index(idx) {
            Ok(f) => f,
            Err(e) => {
                warnings.push(format!("Skipped unreadable entry #{idx}: {e}"));
                continue;
            }
        };
        if file.is_dir() {
            continue;
        }
        let Ok(raw_name) = file.name() else {
            continue;
        };
        let name = raw_name.replace('\\', "/");
        if file.size() > MAX_JAR_ENTRY_BYTES {
            skipped_details.push(SkippedImportDetail {
                path: name,
                reason_code: "OVERSIZED".to_owned(),
                message_ru: "Запись превышает лимит 16 МиБ".to_owned(),
                message_en: "Entry exceeds 16 MiB limit".to_owned(),
            });
            continue;
        }

        if let Some(rest) = name.strip_prefix("assets/") {
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() >= 2 {
                let ns = parts[0];
                namespaces.insert(ns.to_owned());
                let sub = parts[1];
                let tail = parts.get(2).copied().unwrap_or("");
                let is_wanted_asset = (sub == "lang"
                    && (tail == "en_us.json" || tail == "ru_ru.json"))
                    || ((sub == "models" || sub == "blockstates") && tail.ends_with(".json"))
                    || (sub == "textures" && tail.ends_with(".png"));

                if is_wanted_asset {
                    let mut buf = Vec::new();
                    if (&mut file).take(MAX_JAR_ENTRY_BYTES).read_to_end(&mut buf).is_ok() {
                        if sub == "lang" && tail == "en_us.json" {
                            if let Ok(m) = serde_json::from_slice::<HashMap<String, String>>(&buf) {
                                lang_en.extend(m);
                            }
                        } else if sub == "lang"
                            && tail == "ru_ru.json"
                            && let Ok(m) = serde_json::from_slice::<HashMap<String, String>>(&buf)
                        {
                            lang_ru.extend(m);
                        }

                        if let Some(root) = options.extract_visual_assets_to.as_ref()
                            && let Some(clean_rel) = sanitize_zip_rel_path(rest)
                        {
                            total_extracted_bytes =
                                total_extracted_bytes.saturating_add(buf.len() as u64);
                            if total_extracted_bytes > MAX_TOTAL_EXTRACT_BYTES {
                                warnings.push(
                                    "Stopped asset extraction at 512 MiB safety limit".to_owned(),
                                );
                            } else {
                                let dest = root.join(clean_rel);
                                if let Some(parent) = dest.parent() {
                                    let _ = fs::create_dir_all(parent);
                                }
                                if fs::write(&dest, &buf).is_ok() && tail.ends_with(".png") {
                                    extracted_textures_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        } else if options.import_recipes
            && raw_recipes.len() < options.max_recipes
            && let Some(rest) = name.strip_prefix("data/")
            && rest.ends_with(".json")
        {
            let parts: Vec<&str> = rest.splitn(3, '/').collect();
            if parts.len() == 3 && (parts[1] == "recipes" || parts[1] == "recipe") {
                let ns = parts[0];
                namespaces.insert(ns.to_owned());
                let stem = parts[2].trim_end_matches(".json");
                let recipe_id = format!("{ns}:{stem}");
                let mut buf = Vec::new();
                if (&mut file).take(MAX_JAR_ENTRY_BYTES).read_to_end(&mut buf).is_ok()
                    && let Ok(val) = serde_json::from_slice::<Value>(&buf)
                {
                    raw_recipes.push((recipe_id, ns.to_owned(), name, val));
                }
            }
        }
    }

    let mut recipes = Vec::new();
    for (rid, ns, path, val) in &raw_recipes {
        if let Some(rec) = parse_mc_recipe_json(rid, ns, path, val, &lang_en, &lang_ru) {
            recipes.push(rec);
        } else {
            skipped_details.push(SkippedImportDetail {
                path: path.clone(),
                reason_code: "UNSUPPORTED_RECIPE".to_owned(),
                message_ru: "Формат рецепта не распознан (например динамический или условие)".to_owned(),
                message_en: "Recipe format not recognized".to_owned(),
            });
        }
    }

    on_progress(total, total, "Archive scan complete");

    let primary_ns = namespaces
        .iter()
        .find(|n| *n != "minecraft")
        .cloned()
        .unwrap_or_else(|| "unknown".to_owned());

    Ok(JarImportReport {
        file_name: archive_name.to_owned(),
        detected_mod_id: primary_ns.clone(),
        detected_mod_name: primary_ns,
        detected_mod_version: "detected".to_owned(),
        detected_loader: "NeoForge/Forge/Fabric".to_owned(),
        detected_minecraft_version: "1.21.1".to_owned(),
        archive_size_bytes,
        entries_scanned: total,
        imported_count: recipes.len(),
        skipped_count: skipped_details.len(),
        extracted_textures_count,
        recipes,
        skipped_details,
        warnings,
    })
}

pub fn import_mod_archive_file(
    path: &Path,
    options: &JarImportOptions,
    cancel: Option<&AtomicBool>,
    on_progress: impl FnMut(usize, usize, &str),
) -> Result<JarImportReport, String> {
    let size = fs::metadata(path).map(|m| m.len() as usize).unwrap_or(0);
    let file = fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archive.jar");
    import_mod_archive_reader(name, size, file, options, cancel, on_progress)
}
