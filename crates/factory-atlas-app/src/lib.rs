use factory_atlas_core::model::{
    FactoryEdge, FactoryNode, FactoryProject, NodeKind, RateMode, ResourceDef, ResourceKind,
};
use factory_atlas_core::project_format::now_iso_timestamp;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn add_resource(
    project: &mut FactoryProject,
    id: &str,
    name_ru: &str,
    name_en: &str,
    kind: ResourceKind,
) -> Result<(), String> {
    let id = id.trim();
    let name_ru = name_ru.trim();
    let name_en = name_en.trim();
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        return Err("Укажите ID без пробелов, например custom:iron_plate.".to_owned());
    }
    if name_ru.is_empty() || name_en.is_empty() {
        return Err("Заполните названия на русском и английском.".to_owned());
    }
    if project.custom_resources.iter().any(|item| item.id == id) {
        return Err(format!("Ресурс с ID {id} уже существует."));
    }

    let mod_id = id
        .split_once(':')
        .map(|(namespace, _)| namespace)
        .unwrap_or("custom");
    project.custom_resources.push(ResourceDef {
        id: id.to_owned(),
        kind,
        name_ru: name_ru.to_owned(),
        name_en: name_en.to_owned(),
        mod_id: mod_id.to_owned(),
        category: "intermediate".to_owned(),
        color: "#f59e0b".to_owned(),
        visual: None,
        user_created: Some(true),
    });
    project.updated_at = now_iso_timestamp();
    Ok(())
}

pub fn add_node(
    project: &mut FactoryProject,
    kind: NodeKind,
    title: &str,
    resource_id: Option<&str>,
    note: &str,
    rate_per_min: Option<f64>,
) -> Result<String, String> {
    if !matches!(kind, NodeKind::Source | NodeKind::Target | NodeKind::Note) {
        return Err("На этом этапе доступны узлы источника, цели и заметки.".to_owned());
    }
    let title = title.trim();
    if title.is_empty() {
        return Err("Введите название узла.".to_owned());
    }

    let resource = match (kind, resource_id) {
        (NodeKind::Source | NodeKind::Target, Some(id)) => Some(
            project
                .custom_resources
                .iter()
                .find(|item| item.id == id)
                .ok_or_else(|| "Выбранный ресурс не найден в проекте.".to_owned())?,
        ),
        (NodeKind::Source | NodeKind::Target, None) => {
            return Err("Для источника и цели выберите ресурс.".to_owned());
        }
        (NodeKind::Note, _) => None,
        _ => None,
    };
    let rate = match kind {
        NodeKind::Source | NodeKind::Target => Some(
            rate_per_min
                .filter(|rate| rate.is_finite() && *rate > 0.0)
                .ok_or_else(|| "Укажите положительный конечный темп для узла.".to_owned())?,
        ),
        _ => None,
    };

    let index = project.nodes.len() as f64;
    let node_id = unique_id("node");
    project.nodes.push(FactoryNode {
        id: node_id.clone(),
        kind,
        title: title.to_owned(),
        x: 80.0 + (index % 4.0) * 220.0,
        y: 80.0 + (index / 4.0).floor() * 140.0,
        resource_id: resource.map(|item| item.id.clone()),
        resource_kind: resource.map(|item| item.kind),
        machine_id: None,
        recipe_id: None,
        source_rate_per_min: if kind == NodeKind::Source { rate } else { None },
        source_unlimited: None,
        target_rate_per_min: if kind == NodeKind::Target { rate } else { None },
        rpm: None,
        speed_mode: None,
        manual_crafts_per_min: None,
        stack_size_assumption: None,
        machine_count_override: None,
        note_text: (kind == NodeKind::Note).then(|| note.to_owned()),
        color_tag: None,
    });
    project.updated_at = now_iso_timestamp();
    Ok(node_id)
}

pub fn connect_source_to_target(
    project: &mut FactoryProject,
    source_id: &str,
    target_id: &str,
) -> Result<(), String> {
    let source = project
        .nodes
        .iter()
        .find(|node| node.id == source_id)
        .ok_or_else(|| "Узел источника не найден.".to_owned())?;
    let target = project
        .nodes
        .iter()
        .find(|node| node.id == target_id)
        .ok_or_else(|| "Узел цели не найден.".to_owned())?;
    if source.kind != NodeKind::Source || target.kind != NodeKind::Target {
        return Err("Соединение должно идти от источника к цели.".to_owned());
    }
    if source.resource_id != target.resource_id || source.resource_kind != target.resource_kind {
        return Err("Источник и цель должны использовать один ресурс и его единицы.".to_owned());
    }
    let resource_id = source
        .resource_id
        .clone()
        .ok_or_else(|| "У узла отсутствует ресурс.".to_owned())?;
    let resource_kind = source
        .resource_kind
        .ok_or_else(|| "У узла отсутствует тип ресурса.".to_owned())?;
    if project
        .edges
        .iter()
        .any(|edge| edge.from_node_id == source_id && edge.to_node_id == target_id)
    {
        return Err("Такое соединение уже есть.".to_owned());
    }

    project.edges.push(FactoryEdge {
        id: unique_id("edge"),
        from_node_id: source_id.to_owned(),
        to_node_id: target_id.to_owned(),
        resource_id,
        resource_kind,
        rate_mode: RateMode::Auto,
        manual_rate_per_min: None,
        is_byproduct_port: Some(false),
    });
    project.updated_at = now_iso_timestamp();
    Ok(())
}

fn unique_id(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{prefix}-{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::{add_node, add_resource, connect_source_to_target};
    use factory_atlas_core::model::{NodeKind, ResourceKind};
    use factory_atlas_core::project_format::create_empty_project;

    #[test]
    fn new_project_stays_empty_until_user_adds_data() {
        let project = create_empty_project("Пустой проект");
        assert!(project.nodes.is_empty());
        assert!(project.edges.is_empty());
        assert!(project.custom_resources.is_empty());
        assert!(project.custom_machines.is_empty());
        assert!(project.custom_recipes.is_empty());
    }

    #[test]
    fn manually_created_resource_and_nodes_round_trip_in_existing_format() {
        let mut project = create_empty_project("Тестовый проект");
        add_resource(
            &mut project,
            "custom:iron_plate",
            "Железная пластина",
            "Iron Plate",
            ResourceKind::Item,
        )
        .unwrap();
        let source = add_node(
            &mut project,
            NodeKind::Source,
            "Ручной источник",
            Some("custom:iron_plate"),
            "",
            Some(60.0),
        )
        .unwrap();
        let target = add_node(
            &mut project,
            NodeKind::Target,
            "Ручная цель",
            Some("custom:iron_plate"),
            "",
            Some(60.0),
        )
        .unwrap();
        connect_source_to_target(&mut project, &source, &target).unwrap();

        let folder = std::env::temp_dir().join(super::unique_id("factory-atlas-app-test"));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("round-trip.factoryatlas");
        factory_atlas_core::storage::save_project_file(&file, &project).unwrap();
        let (reopened, migration) = factory_atlas_core::storage::load_project_file(&file).unwrap();
        assert_eq!(reopened.custom_resources.len(), 1);
        assert_eq!(reopened.nodes.len(), 2);
        assert_eq!(reopened.edges.len(), 1);
        assert_eq!(reopened.nodes[0].source_rate_per_min, Some(60.0));
        assert_eq!(reopened.nodes[1].target_rate_per_min, Some(60.0));
        assert!(reopened.custom_machines.is_empty());
        assert!(reopened.custom_recipes.is_empty());
        assert_eq!(migration, None);
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn resource_and_edge_validation_rejects_invalid_changes() {
        let mut project = create_empty_project("Пустой проект");
        assert!(add_resource(&mut project, "bad id", "A", "A", ResourceKind::Item).is_err());
        add_resource(
            &mut project,
            "custom:iron",
            "Железо",
            "Iron",
            ResourceKind::Item,
        )
        .unwrap();
        assert!(
            add_resource(
                &mut project,
                "custom:iron",
                "Железо",
                "Iron",
                ResourceKind::Item
            )
            .is_err()
        );
        let target = add_node(
            &mut project,
            NodeKind::Target,
            "Цель",
            Some("custom:iron"),
            "",
            Some(10.0),
        )
        .unwrap();
        assert!(connect_source_to_target(&mut project, "missing", &target).is_err());
    }
}
