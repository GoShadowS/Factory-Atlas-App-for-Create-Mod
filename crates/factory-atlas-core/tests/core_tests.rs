use factory_atlas_core::exporters::{export_balance_csv, export_graphviz_dot, export_summary_markdown};
use factory_atlas_core::fixtures::build_benchmark_project;
use factory_atlas_core::jar_importer::{JarImportOptions, import_mod_archive_reader};
use factory_atlas_core::machine_speeds::compute_machine_throughput;
use factory_atlas_core::mc_model::{
    AssetSource, BoundedModelCache, resolve_texture_var, resolve_visual_entry_asset,
};
use factory_atlas_core::model::{
    FactoryEdge, Locale, MachineDef, MachineSpeedMode, ModelKind, RateMode, RecipeDef,
    ResourceKind, SpeedFormulaKind, SpeedSource, VisualEntrySummary, WarningCode,
};
use factory_atlas_core::nbt_parser::{NbtTag, encode_test_nbt_compound, parse_nbt_schematic_bytes};
use factory_atlas_core::project_format::{
    create_empty_project, load_project_json_str, save_project_json_str,
};
use factory_atlas_core::solver::{IncrementalSolverCache, SolverCatalog, solve_factory_project};
use factory_atlas_core::visual_matcher::{VisualMatchStatus, match_visual_resources, normalize_visual_query};
use factory_atlas_core::worker_pool::{BackgroundJob, BackgroundOutput, WorkerPool};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};
use std::io::{Cursor, Write};
use std::time::{Duration, Instant};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn empty_start_project_has_zero_nodes_and_catalog() {
    let p = create_empty_project("Пустой проект");
    assert!(p.nodes.is_empty());
    assert!(p.edges.is_empty());
    assert!(p.custom_resources.is_empty());
    assert!(p.custom_machines.is_empty());
    assert!(p.custom_recipes.is_empty());
    assert_eq!(p.schema_version, 2);

    let saved = save_project_json_str(&p).unwrap();
    let loaded = load_project_json_str(&saved).unwrap();
    assert!(loaded.nodes.is_empty());
    assert!(loaded.custom_recipes.is_empty());
}

#[test]
fn legacy_v1_project_migrates_to_v2() {
    let v1_json = r#"{
      "schemaVersion": 1,
      "meta": { "id": "p1", "name": "Legacy", "createdAt": "2026-01-01T00:00:00Z", "updatedAt": "2026-01-01T00:00:00Z" },
      "nodes": [
        { "id": "s1", "kind": "source", "title": "Iron", "x": 10, "y": 20, "resourceId": "minecraft:iron_ore", "resourceKind": "item", "sourceRatePerMin": 30 },
        { "id": "t1", "kind": "target", "title": "Out", "x": 210, "y": 20, "resourceId": "minecraft:iron_ore", "resourceKind": "item", "targetRatePerMin": 30 }
      ],
      "edges": [
        { "id": "e1", "fromNodeId": "s1", "toNodeId": "t1", "resourceId": "minecraft:iron_ore", "resourceKind": "item" }
      ],
      "customResources": [],
      "customMachines": [],
      "customRecipes": []
    }"#;
    let p = load_project_json_str(v1_json).unwrap();
    assert_eq!(p.schema_version, 2);
    assert_eq!(p.nodes.len(), 2);
    assert_eq!(p.edges.len(), 1);
    assert_eq!(p.nodes[0].speed_mode, MachineSpeedMode::CreateFormula);
}

#[test]
fn create_kinetic_machine_formulas_match_6_0_11() {
    let mixer = MachineDef {
        id: "create:mechanical_mixer".into(),
        name_ru: "Миксер".into(),
        name_en: "Mechanical Mixer".into(),
        speed_formula_id: SpeedFormulaKind::MechanicalMixer,
        stress_impact_su: Some(4.0),
        default_rpm: Some(64.0),
        custom: true,
    };
    let rec = RecipeDef {
        id: "r".into(),
        machine_id: mixer.id.clone(),
        title_ru: "Смесь".into(),
        title_en: "Mix".into(),
        processing_time_ticks: Some(100),
        heat_requirement: None,
        inputs: vec![],
        outputs: vec![],
        notes: None,
        custom: true,
    };
    // At 64 RPM, mixer speed factor = clamp(log2(64)/2, 1, 2) = 1.5 -> duration = 100/1.5 = 67 ticks -> 1200/67 ops/min
    let calc = compute_machine_throughput(
        Some(&mixer),
        Some(&rec),
        Some(64.0),
        MachineSpeedMode::CreateFormula,
        None,
        None,
    );
    assert_eq!(calc.source, SpeedSource::Formula);
    let cpm = calc.crafts_per_min.unwrap();
    assert!((cpm - (1200.0 / 67.0)).abs() < 0.01);

    // Millstone at 64 RPM, 100 ticks: step = 64/16 = 4 -> ceil(100/4) + 1 = 26 ticks -> 1200/26 ops/min
    let mill = MachineDef {
        speed_formula_id: SpeedFormulaKind::Millstone,
        ..mixer.clone()
    };
    let mill_calc = compute_machine_throughput(
        Some(&mill),
        Some(&rec),
        Some(64.0),
        MachineSpeedMode::CreateFormula,
        None,
        None,
    );
    assert!((mill_calc.crafts_per_min.unwrap() - (1200.0 / 26.0)).abs() < 0.01);

    // Manual machine without formula falls back cleanly to Unknown unless manual rate supplied
    let custom_mach = MachineDef {
        speed_formula_id: SpeedFormulaKind::Unknown,
        stress_impact_su: None,
        ..mixer
    };
    let unk = compute_machine_throughput(
        Some(&custom_mach),
        Some(&rec),
        Some(32.0),
        MachineSpeedMode::CreateFormula,
        None,
        None,
    );
    assert_eq!(unk.source, SpeedSource::Unknown);
}

#[test]
fn solver_detects_unit_mismatch_cycles_and_caches_components() {
    let mut p = build_benchmark_project(5);
    // Inject a cycle in component 0 (wash-0 -> crush-0)
    p.edges.push(FactoryEdge {
        id: "cycle-e".into(),
        from_node_id: "wash-0".into(),
        to_node_id: "crush-0".into(),
        resource_id: "create:raw_iron".into(),
        resource_kind: ResourceKind::Item,
        rate_mode: RateMode::Auto,
        manual_rate_per_min: None,
        is_byproduct_port: false,
    });
    // Inject an item->fluid unit mismatch edge in component 1
    p.edges.push(FactoryEdge {
        id: "mismatch-e".into(),
        from_node_id: "src-1".into(),
        to_node_id: "tgt-1".into(),
        resource_id: "minecraft:water".into(),
        resource_kind: ResourceKind::Fluid,
        rate_mode: RateMode::Auto,
        manual_rate_per_min: None,
        is_byproduct_port: false,
    });

    let cat = SolverCatalog::from_project(&p);
    let mut cache = IncrementalSolverCache::new(100);
    let res1 = solve_factory_project(&p, &cat, Some(&mut cache));
    assert_eq!(res1.stats.recomputed_components, 5);
    assert_eq!(res1.stats.cached_components, 0);
    assert_eq!(res1.stats.cycle_count, 1);
    assert!(res1.warnings.iter().any(|w| w.code == WarningCode::UnitMismatch));
    assert!(res1.warnings.iter().any(|w| w.code == WarningCode::CycleDetected));

    // Mutate only node src-2 rate; components 0, 1, 3, 4 must be cache hits!
    p.nodes
        .iter_mut()
        .find(|n| n.id == "src-2")
        .unwrap()
        .source_rate_per_min = Some(120.0);
    let cat2 = SolverCatalog::from_project(&p);
    let res2 = solve_factory_project(&p, &cat2, Some(&mut cache));
    assert_eq!(res2.stats.recomputed_components, 1);
    assert_eq!(res2.stats.cached_components, 4);

    // Exporters produce non-empty valid output
    assert!(export_balance_csv(&res2).contains("minecraft:iron_nugget"));
    assert!(export_summary_markdown(&p, &res2, Locale::Ru).contains("Баланс ресурсов"));
    assert!(export_graphviz_dot(&p, &res2).contains("digraph FactoryAtlas"));
}

#[test]
fn visual_matcher_five_tiers_and_no_auto_creation() {
    let entries = vec![
        VisualEntrySummary {
            id: "create:cogwheel".into(),
            namespace: "create".into(),
            path_id: "cogwheel".into(),
            name_en: "Cogwheel".into(),
            name_ru: Some("Шестерёнка".into()),
            normalized_key: normalize_visual_query("create:cogwheel"),
            normalized_en: normalize_visual_query("Cogwheel"),
            normalized_ru: normalize_visual_query("Шестерёнка"),
            model_path: Some("create/models/item/cogwheel.json".into()),
            world_model_path: Some("create/models/block/cogwheel.json".into()),
            blockstate_path: Some("create/blockstates/cogwheel.json".into()),
            texture_path: Some("create/textures/block/cogwheel.png".into()),
            model_ref: Some("create:block/cogwheel".into()),
            has_model_3d: true,
            model_kind: ModelKind::JsonElements3d,
        },
        VisualEntrySummary {
            id: "create:large_cogwheel".into(),
            namespace: "create".into(),
            path_id: "large_cogwheel".into(),
            name_en: "Large Cogwheel".into(),
            name_ru: Some("Большая шестерёнка".into()),
            normalized_key: normalize_visual_query("create:large_cogwheel"),
            normalized_en: normalize_visual_query("Large Cogwheel"),
            normalized_ru: normalize_visual_query("Большая шестерёнка"),
            model_path: Some("create/models/item/large_cogwheel.json".into()),
            world_model_path: None,
            blockstate_path: None,
            texture_path: None,
            model_ref: None,
            has_model_3d: false,
            model_kind: ModelKind::ItemGenerated2d,
        },
    ];

    let r1 = match_visual_resources("create:cogwheel", &entries, 8);
    assert_eq!(r1.status, VisualMatchStatus::Unique);
    assert_eq!(r1.best.unwrap().id, "create:cogwheel");

    let r2 = match_visual_resources("create/models/item/cogwheel.json", &entries, 8);
    assert_eq!(r2.status, VisualMatchStatus::Unique);

    let r3 = match_visual_resources("Шестерёнка", &entries, 8);
    assert_eq!(r3.status, VisualMatchStatus::Unique);

    let r4 = match_visual_resources("cogwheel", &entries, 8);
    assert_eq!(r4.status, VisualMatchStatus::Ambiguous);
    assert_eq!(r4.candidates.len(), 2);

    // Offline missing index gracefully returns NotFound
    let r5 = match_visual_resources("create:cogwheel", &[], 8);
    assert_eq!(r5.status, VisualMatchStatus::NotFound);
}

#[test]
fn mc_model_resolves_parent_chains_blockstates_and_texture_cycles() {
    struct MockAssets(HashMap<String, Value>);
    impl AssetSource for MockAssets {
        fn read_json(&self, rel_path: &str) -> Option<Value> {
            self.0.get(rel_path).cloned()
        }
        fn exists(&self, rel_path: &str) -> bool {
            self.0.contains_key(rel_path)
        }
    }

    let mut files = HashMap::new();
    files.insert(
        "create/blockstates/brass_casing.json".into(),
        json!({
            "variants": { "": { "model": "create:block/brass_casing" } }
        }),
    );
    files.insert(
        "create/models/block/brass_casing.json".into(),
        json!({
            "parent": "minecraft:block/cube_all",
            "textures": { "all": "create:block/brass_casing" }
        }),
    );
    files.insert(
        "create/models/item/brass_ingot.json".into(),
        json!({
            "parent": "minecraft:item/generated",
            "textures": { "layer0": "create:item/brass_ingot" }
        }),
    );

    let src = MockAssets(files);
    let mut cache = BoundedModelCache::new(16);

    let block_entry = VisualEntrySummary {
        id: "create:brass_casing".into(),
        namespace: "create".into(),
        path_id: "brass_casing".into(),
        name_en: "Brass Casing".into(),
        name_ru: None,
        normalized_key: "createbrasscasing".into(),
        normalized_en: "brasscasing".into(),
        normalized_ru: String::new(),
        model_path: None,
        world_model_path: Some("create/models/block/brass_casing.json".into()),
        blockstate_path: Some("create/blockstates/brass_casing.json".into()),
        texture_path: None,
        model_ref: Some("create:block/brass_casing".into()),
        has_model_3d: true,
        model_kind: ModelKind::JsonElements3d,
    };

    let resolved_3d = cache.get_or_resolve(&block_entry, &src).clone();
    assert_eq!(resolved_3d.kind, ModelKind::JsonElements3d);
    assert_eq!(resolved_3d.parts[0].elements.len(), 1);
    assert_eq!(
        resolved_3d.parts[0].textures.get("all").map(String::as_str),
        Some("create/textures/block/brass_casing.png")
    );

    let item_entry = VisualEntrySummary {
        id: "create:brass_ingot".into(),
        namespace: "create".into(),
        path_id: "brass_ingot".into(),
        name_en: "Brass Ingot".into(),
        name_ru: None,
        normalized_key: "createbrassingot".into(),
        normalized_en: "brassingot".into(),
        normalized_ru: String::new(),
        model_path: Some("create/models/item/brass_ingot.json".into()),
        world_model_path: None,
        blockstate_path: None,
        texture_path: Some("create/textures/item/brass_ingot.png".into()),
        model_ref: Some("create:item/brass_ingot".into()),
        has_model_3d: false,
        model_kind: ModelKind::ItemGenerated2d,
    };
    let resolved_2d = resolve_visual_entry_asset(&item_entry, &src);
    assert_eq!(resolved_2d.kind, ModelKind::ItemGenerated2d);

    // Cycle protection in #texture variable resolution
    let mut cyc = HashMap::new();
    cyc.insert("a".into(), "#b".into());
    cyc.insert("b".into(), "#a".into());
    assert!(resolve_texture_var("#a", &cyc, 16).is_none());
}

#[test]
fn nbt_schematic_parses_gzipped_structure() {
    let mut pal0 = BTreeMap::new();
    pal0.insert("Name".into(), NbtTag::String("create:cogwheel".into()));
    let mut pal1 = BTreeMap::new();
    pal1.insert("Name".into(), NbtTag::String("create:andesite_casing".into()));

    let mut b0 = BTreeMap::new();
    b0.insert("state".into(), NbtTag::Int(0));
    b0.insert("pos".into(), NbtTag::List(3, vec![NbtTag::Int(0), NbtTag::Int(0), NbtTag::Int(0)]));
    let mut b1 = BTreeMap::new();
    b1.insert("state".into(), NbtTag::Int(1));
    b1.insert("pos".into(), NbtTag::List(3, vec![NbtTag::Int(1), NbtTag::Int(0), NbtTag::Int(0)]));

    let raw_nbt = encode_test_nbt_compound(
        "",
        &[
            ("DataVersion", NbtTag::Int(3955)),
            ("size", NbtTag::List(3, vec![NbtTag::Int(2), NbtTag::Int(1), NbtTag::Int(1)])),
            ("palette", NbtTag::List(10, vec![NbtTag::Compound(pal0), NbtTag::Compound(pal1)])),
            ("blocks", NbtTag::List(10, vec![NbtTag::Compound(b0), NbtTag::Compound(b1)])),
        ],
    );

    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&raw_nbt).unwrap();
    let gz_bytes = gz.finish().unwrap();

    let parsed = parse_nbt_schematic_bytes(&gz_bytes).unwrap();
    assert_eq!(parsed.size, [2, 1, 1]);
    assert_eq!(parsed.blocks.len(), 2);
    assert_eq!(parsed.block_counts.len(), 2);
}

#[test]
fn jar_importer_reads_create_recipes_and_lang() {
    let buf = Cursor::new(Vec::new());
    let mut zw = ZipWriter::new(buf);
    let opt = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zw.start_file("assets/create/lang/en_us.json", opt).unwrap();
    zw.write_all(br#"{"item.create.wheat_flour":"Wheat Flour"}"#).unwrap();
    zw.start_file("assets/create/lang/ru_ru.json", opt).unwrap();
    zw.write_all(r#"{"item.create.wheat_flour":"Пшеничная мука"}"#.as_bytes()).unwrap();

    zw.start_file("data/create/recipes/milling/wheat.json", opt).unwrap();
    zw.write_all(
        br#"{
          "type": "create:milling",
          "ingredients": [{ "item": "minecraft:wheat" }],
          "processingTime": 150,
          "results": [{ "item": "create:wheat_flour", "count": 1 }]
        }"#,
    )
    .unwrap();

    let cursor = zw.finish().unwrap();
    let report = import_mod_archive_reader(
        "create-test.jar",
        Cursor::new(cursor.into_inner()),
        &JarImportOptions::default(),
        None,
        |_, _, _| {},
    )
    .unwrap();

    assert_eq!(report.discovered_recipes.len(), 1);
    assert_eq!(report.discovered_recipes[0].title_ru, "Пшеничная мука");
    assert_eq!(report.discovered_recipes[0].machine_id, "create:millstone");
    assert!(!report.discovered_resources.is_empty());
}

#[test]
fn worker_pool_solves_and_benchmarks_200_nodes() {
    let pool = WorkerPool::new(Some(2));
    let project = build_benchmark_project(50); // 200 nodes, 150 edges

    let t0 = Instant::now();
    let tid = pool.submit(BackgroundJob::SolveProject {
        project: project.clone(),
    });

    let mut got = None;
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        for msg in pool.poll_outputs() {
            if let BackgroundOutput::Solved { task_id, result } = msg
                && task_id == tid
            {
                got = Some(result);
            }
        }
        if got.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }

    let elapsed = t0.elapsed();
    let res = got.expect("worker pool did not solve within timeout");
    assert_eq!(res.stats.total_nodes, 200);
    assert_eq!(res.stats.component_count, 50);
    assert!(elapsed < Duration::from_secs(2));
}
