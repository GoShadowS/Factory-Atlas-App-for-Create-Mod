use crate::model::{
    FactoryEdge, FactoryNode, FactoryProject, MachineDef, MachineSpeedMode, NodeKind,
    ProbabilityKind, RateMode, RecipeDef, RecipeIngredient, RecipeOutput, ResourceDef,
    ResourceKind, SpeedFormulaKind,
};
use crate::project_format::create_empty_project;

/// Builds a synthetic Stress-test / Benchmark graph with `chains` parallel processing lines
/// (default e.g. 50 chains = 200 nodes, 150 edges) purely for automated solver & RSS benchmarks.
/// Never auto-loaded on app startup!
pub fn build_benchmark_project(chains: usize) -> FactoryProject {
    let mut p = create_empty_project("Benchmark Graph");
    p.custom_resources.push(ResourceDef {
        id: "create:raw_iron".to_owned(),
        kind: ResourceKind::Item,
        name_ru: "Рудное железо".to_owned(),
        name_en: "Raw Iron".to_owned(),
        color_hex: Some("#d18c54".to_owned()),
        stack_size: Some(64),
        custom: true,
        visual: None,
    });
    p.custom_resources.push(ResourceDef {
        id: "create:crushed_raw_iron".to_owned(),
        kind: ResourceKind::Item,
        name_ru: "Дроблёная железная руда".to_owned(),
        name_en: "Crushed Raw Iron".to_owned(),
        color_hex: Some("#96a2b3".to_owned()),
        stack_size: Some(64),
        custom: true,
        visual: None,
    });
    p.custom_resources.push(ResourceDef {
        id: "minecraft:iron_nugget".to_owned(),
        kind: ResourceKind::Item,
        name_ru: "Железный самородок".to_owned(),
        name_en: "Iron Nugget".to_owned(),
        color_hex: Some("#d8d8d8".to_owned()),
        stack_size: Some(64),
        custom: true,
        visual: None,
    });
    p.custom_resources.push(ResourceDef {
        id: "create:redstone_nugget".to_owned(),
        kind: ResourceKind::Item,
        name_ru: "Пыль редстоуна".to_owned(),
        name_en: "Redstone Dust".to_owned(),
        color_hex: Some("#d93838".to_owned()),
        stack_size: Some(64),
        custom: true,
        visual: None,
    });

    p.custom_machines.push(MachineDef {
        id: "create:crushing_wheel".to_owned(),
        name_ru: "Дробильное колесо".to_owned(),
        name_en: "Crushing Wheel".to_owned(),
        speed_formula_id: SpeedFormulaKind::CrushingWheel,
        stress_impact_su: Some(8.0),
        default_rpm: Some(128.0),
        custom: true,
    });
    p.custom_machines.push(MachineDef {
        id: "create:encased_fan".to_owned(),
        name_ru: "Вентилятор в корпусе".to_owned(),
        name_en: "Encased Fan".to_owned(),
        speed_formula_id: SpeedFormulaKind::EncasedFanBulk,
        stress_impact_su: Some(2.0),
        default_rpm: Some(64.0),
        custom: true,
    });

    p.custom_recipes.push(RecipeDef {
        id: "bench:crush_iron".to_owned(),
        machine_id: "create:crushing_wheel".to_owned(),
        title_ru: "Дробление железа".to_owned(),
        title_en: "Crush Raw Iron".to_owned(),
        processing_time_ticks: Some(150),
        heat_requirement: None,
        inputs: vec![RecipeIngredient {
            resource_id: "create:raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
        }],
        outputs: vec![RecipeOutput {
            resource_id: "create:crushed_raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
            chance: 1.0,
            probability_kind: ProbabilityKind::Exact,
            weight: None,
        }],
        notes: None,
        custom: true,
    });
    p.custom_recipes.push(RecipeDef {
        id: "bench:wash_iron".to_owned(),
        machine_id: "create:encased_fan".to_owned(),
        title_ru: "Промывка железа".to_owned(),
        title_en: "Wash Crushed Iron".to_owned(),
        processing_time_ticks: Some(150),
        heat_requirement: None,
        inputs: vec![RecipeIngredient {
            resource_id: "create:crushed_raw_iron".to_owned(),
            kind: ResourceKind::Item,
            amount: 1.0,
        }],
        outputs: vec![
            RecipeOutput {
                resource_id: "minecraft:iron_nugget".to_owned(),
                kind: ResourceKind::Item,
                amount: 9.0,
                chance: 1.0,
                probability_kind: ProbabilityKind::Exact,
                weight: None,
            },
            RecipeOutput {
                resource_id: "create:redstone_nugget".to_owned(),
                kind: ResourceKind::Item,
                amount: 1.0,
                chance: 0.75,
                probability_kind: ProbabilityKind::IndependentChance,
                weight: None,
            },
        ],
        notes: None,
        custom: true,
    });

    for i in 0..chains {
        let y = (i as f32) * 200.0;
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
            source_rate_per_min: Some(60.0),
            source_unlimited: Some(false),
            target_rate_per_min: None,
            recipe_id: None,
            machine_id: None,
            rpm: None,
            speed_mode: MachineSpeedMode::CreateFormula,
            manual_crafts_per_min: None,
            stack_size_assumption: None,
            machine_count_override: None,
            note_text: None,
        });
        p.nodes.push(FactoryNode {
            id: n_crush.clone(),
            kind: NodeKind::Recipe,
            title: format!("Crush #{i}"),
            x: 360.0,
            y,
            resource_id: None,
            resource_kind: None,
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: None,
            recipe_id: Some("bench:crush_iron".to_owned()),
            machine_id: Some("create:crushing_wheel".to_owned()),
            rpm: Some(128.0),
            speed_mode: MachineSpeedMode::CreateFormula,
            manual_crafts_per_min: None,
            stack_size_assumption: Some(1),
            machine_count_override: None,
            note_text: None,
        });
        p.nodes.push(FactoryNode {
            id: n_wash.clone(),
            kind: NodeKind::Recipe,
            title: format!("Wash #{i}"),
            x: 680.0,
            y,
            resource_id: None,
            resource_kind: None,
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: None,
            recipe_id: Some("bench:wash_iron".to_owned()),
            machine_id: Some("create:encased_fan".to_owned()),
            rpm: Some(64.0),
            speed_mode: MachineSpeedMode::CreateFormula,
            manual_crafts_per_min: None,
            stack_size_assumption: Some(1),
            machine_count_override: None,
            note_text: None,
        });
        p.nodes.push(FactoryNode {
            id: n_tgt.clone(),
            kind: NodeKind::Target,
            title: format!("Target #{i}"),
            x: 1000.0,
            y,
            resource_id: Some("minecraft:iron_nugget".to_owned()),
            resource_kind: Some(ResourceKind::Item),
            source_rate_per_min: None,
            source_unlimited: None,
            target_rate_per_min: Some(450.0),
            recipe_id: None,
            machine_id: None,
            rpm: None,
            speed_mode: MachineSpeedMode::CreateFormula,
            manual_crafts_per_min: None,
            stack_size_assumption: None,
            machine_count_override: None,
            note_text: None,
        });

        p.edges.push(FactoryEdge {
            id: format!("e1-{i}"),
            from_node_id: n_src,
            to_node_id: n_crush.clone(),
            resource_id: "create:raw_iron".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: false,
        });
        p.edges.push(FactoryEdge {
            id: format!("e2-{i}"),
            from_node_id: n_crush,
            to_node_id: n_wash.clone(),
            resource_id: "create:crushed_raw_iron".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: false,
        });
        p.edges.push(FactoryEdge {
            id: format!("e3-{i}"),
            from_node_id: n_wash,
            to_node_id: n_tgt,
            resource_id: "minecraft:iron_nugget".to_owned(),
            resource_kind: ResourceKind::Item,
            rate_mode: RateMode::Auto,
            manual_rate_per_min: None,
            is_byproduct_port: false,
        });
    }

    p
}
