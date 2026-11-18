use crate::model::{
    FactoryEdge, FactoryNode, FactoryProject, MachineDef, MachineSpeedMode, ModLoader, NodeKind,
    ProbabilityKind, RateMode, RecipeDef, RecipeIngredient, RecipeOrigin, RecipeOutput,
    RecipeSourceMeta, ResourceDef, ResourceKind, SpeedFormulaId,
};
use crate::project_format::create_empty_project;

/// Builds a synthetic stress-test / benchmark graph with `chains` parallel lines
/// purely for automated solver & RSS benchmarks. Never auto-loaded on startup!
pub fn build_benchmark_project(chains: usize) -> FactoryProject {
    let mut p = create_empty_project("Benchmark Graph");
    for (id, name_ru, name_en, color) in [
        ("create:raw_iron", "Рудное железо", "Raw Iron", "#d18c54"),
        (
            "create:crushed_raw_iron",
            "Дроблёная руда",
            "Crushed Raw Iron",
            "#96a2b3",
        ),
        (
            "minecraft:iron_nugget",
            "Железный самородок",
            "Iron Nugget",
            "#d8d8d8",
        ),
        (
            "create:redstone_nugget",
            "Пыль редстоуна",
            "Redstone Dust",
            "#d93838",
        ),
    ] {
        p.custom_resources.push(ResourceDef {
            id: id.to_owned(),
            kind: ResourceKind::Item,
            name_ru: name_ru.to_owned(),
            name_en: name_en.to_owned(),
            mod_id: id.split(':').next().unwrap_or("create").to_owned(),
            category: "intermediate".to_owned(),
            color: color.to_owned(),
            visual: None,
            user_created: Some(true),
        });
    }

    p.custom_machines.push(MachineDef {
        id: "create:crushing_wheel".to_owned(),
        name_ru: "Дробильное колесо".to_owned(),
        name_en: "Crushing Wheel".to_owned(),
        mod_id: "create".to_owned(),
        mod_version: "6.0.11".to_owned(),
        stress_impact_su: Some(8.0),
        default_rpm: Some(128.0),
        max_rpm: 256.0,
        supports_rpm: true,
        speed_formula_id: SpeedFormulaId::CrushingWheel,
        speed_formula_note_ru: "Create 6.0.11 Crushing Wheel".to_owned(),
        speed_formula_note_en: "Create 6.0.11 Crushing Wheel".to_owned(),
        source_ref: "bench".to_owned(),
        visual: None,
        user_created: Some(true),
    });
    p.custom_machines.push(MachineDef {
        id: "create:encased_fan".to_owned(),
        name_ru: "Вентилятор в корпусе".to_owned(),
        name_en: "Encased Fan".to_owned(),
        mod_id: "create".to_owned(),
        mod_version: "6.0.11".to_owned(),
        stress_impact_su: Some(2.0),
        default_rpm: Some(64.0),
        max_rpm: 256.0,
        supports_rpm: true,
        speed_formula_id: SpeedFormulaId::EncasedFan,
        speed_formula_note_ru: "Create 6.0.11 Encased Fan".to_owned(),
        speed_formula_note_en: "Create 6.0.11 Encased Fan".to_owned(),
        source_ref: "bench".to_owned(),
        visual: None,
        user_created: Some(true),
    });

    p.custom_recipes.push(RecipeDef {
        id: "bench:crush_iron".to_owned(),
        recipe_type: "create:crushing".to_owned(),
        machine_id: "create:crushing_wheel".to_owned(),
        name_ru: "Дробление железа".to_owned(),
        name_en: "Crush Raw Iron".to_owned(),
        inputs: vec![RecipeIngredient {
            resource_id: "create:raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
            tag_name: None,
        }],
        outputs: vec![RecipeOutput {
            resource_id: "create:crushed_raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
            chance: 1.0,
            weight: None,
            probability_kind: ProbabilityKind::Guaranteed,
        }],
        processing_time_ticks: Some(150.0),
        heat_requirement: None,
        source: RecipeSourceMeta {
            origin: RecipeOrigin::Manual,
            mod_id: "create".to_owned(),
            mod_version: "6.0.11".to_owned(),
            minecraft_version: "1.21.1".to_owned(),
            loader: ModLoader::Neoforge,
            recipe_path: None,
        },
        notes_ru: None,
        notes_en: None,
    });
    p.custom_recipes.push(RecipeDef {
        id: "bench:wash_iron".to_owned(),
        recipe_type: "create:splashing".to_owned(),
        machine_id: "create:encased_fan".to_owned(),
        name_ru: "Промывка железа".to_owned(),
        name_en: "Wash Crushed Iron".to_owned(),
        inputs: vec![RecipeIngredient {
            resource_id: "create:crushed_raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
            tag_name: None,
        }],
        outputs: vec![
            RecipeOutput {
                resource_id: "minecraft:iron_nugget".to_owned(),
                kind: ResourceKind::Item,
                amount: 9.0,
                chance: 1.0,
                weight: None,
                probability_kind: ProbabilityKind::Guaranteed,
            },
            RecipeOutput {
                resource_id: "create:redstone_nugget".to_owned(),
                kind: ResourceKind::Item,
                amount: 1.0,
                chance: 0.75,
                weight: None,
                probability_kind: ProbabilityKind::Bernoulli,
            },
        ],
        processing_time_ticks: Some(150.0),
        heat_requirement: None,
        source: RecipeSourceMeta {
            origin: RecipeOrigin::Manual,
            mod_id: "create".to_owned(),
            mod_version: "6.0.11".to_owned(),
            minecraft_version: "1.21.1".to_owned(),
            loader: ModLoader::Neoforge,
            recipe_path: None,
        },
        notes_ru: None,
        notes_en: None,
    });

    for i in 0..chains {
        let y = (i as f64) * 200.0;
        let n_src = format!("src-{i}");
        let n_crush = format!("crush-{i}");
        let n_wash = format!("wash-{i}");
        let n_tgt = format!("tgt-{i}");

        p.nodes.push(FactoryNode {
            id: n_src.clone(),
            kind: NodeKind::Source,
            title: format!("Source #{i}"),
            x: 60.0,
            y,
            resource_id: Some("create:raw_iron".to_owned()),
            resource_kind: Some(ResourceKind::Item),
            machine_id: None,
            recipe_id: None,
            source_rate_per_min: Some(60.0),
            source_unlimited: Some(false),
            target_rate_per_min: None,
            rpm: None,
            speed_mode: Some(MachineSpeedMode::Formula),
            manual_crafts_per_min: None,
            stack_size_assumption: None,
            machine_count_override: None,
            note_text: None,
            color_tag: None,
        });
        p.nodes.push(FactoryNode {
            id: n_crush.clone(),
            kind: NodeKind::Recipe,
            title: format!("Crush #{i}"),
            x: 360.0,
            y,
            resource_id: None,
            resource_kind: None,
            machine_id: Some("create:crushing_wheel".to_owned()),
            recipe_id: Some("bench:crush_iron".to_owned()),
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: None,
            rpm: Some(128.0),
            speed_mode: Some(MachineSpeedMode::Formula),
            manual_crafts_per_min: None,
            stack_size_assumption: Some(1.0),
            machine_count_override: None,
            note_text: None,
            color_tag: None,
        });
        p.nodes.push(FactoryNode {
            id: n_wash.clone(),
            kind: NodeKind::Recipe,
            title: format!("Wash #{i}"),
            x: 680.0,
            y,
            resource_id: None,
            resource_kind: None,
            machine_id: Some("create:encased_fan".to_owned()),
            recipe_id: Some("bench:wash_iron".to_owned()),
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: None,
            rpm: Some(64.0),
            speed_mode: Some(MachineSpeedMode::Formula),
            manual_crafts_per_min: None,
            stack_size_assumption: Some(1.0),
            machine_count_override: None,
            note_text: None,
            color_tag: None,
        });
        p.nodes.push(FactoryNode {
            id: n_tgt.clone(),
            kind: NodeKind::Target,
            title: format!("Target #{i}"),
            x: 1000.0,
            y,
            resource_id: Some("minecraft:iron_nugget".to_owned()),
            resource_kind: Some(ResourceKind::Item),
            machine_id: None,
            recipe_id: None,
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: Some(450.0),
            rpm: None,
            speed_mode: Some(MachineSpeedMode::Formula),
            manual_crafts_per_min: None,
            stack_size_assumption: None,
            machine_count_override: None,
            note_text: None,
            color_tag: None,
        });

        p.edges.push(FactoryEdge {
            id: format!("e1-{i}"),
            from_node_id: n_src,
            to_node_id: n_crush.clone(),
            resource_id: "create:raw_iron".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: Some(false),
        });
        p.edges.push(FactoryEdge {
            id: format!("e2-{i}"),
            from_node_id: n_crush,
            to_node_id: n_wash.clone(),
            resource_id: "create:crushed_raw_iron".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: Some(false),
        });
        p.edges.push(FactoryEdge {
            id: format!("e3-{i}"),
            from_node_id: n_wash,
            to_node_id: n_tgt,
            resource_id: "minecraft:iron_nugget".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: Some(false),
        });
    }

    p
}
