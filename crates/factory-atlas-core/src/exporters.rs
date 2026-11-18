use crate::model::{FactoryProject, FlowUnit, Locale, SolverResult};
use std::fmt::Write as _;

pub fn export_project_json(project: &FactoryProject) -> Result<String, String> {
    serde_json::to_string_pretty(project).map_err(|e| format!("Failed to serialize project: {e}"))
}

pub fn export_balance_csv(solver: &SolverResult) -> String {
    let mut out = String::from(
        "resource_id,kind,unit,source_supply_per_min,consumed_per_min,produced_main_per_min,produced_byproduct_per_min,target_demand_per_min,net_balance_per_min,probabilistic\n",
    );
    for row in &solver.balances {
        let _ = writeln!(
            out,
            "\"{}\",{:?},{},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{}",
            row.resource_id.replace('"', "\"\""),
            row.kind,
            row.unit.label(Locale::En),
            row.total_source_supply_per_min,
            row.total_consumed_per_min,
            row.total_produced_main_per_min,
            row.total_produced_byproduct_per_min,
            row.total_target_demand_per_min,
            row.net_balance_per_min,
            row.has_probabilistic_supply
        );
    }
    out
}

pub fn export_summary_markdown(
    project: &FactoryProject,
    solver: &SolverResult,
    locale: Locale,
) -> String {
    let mut out = String::new();
    match locale {
        Locale::Ru => {
            let _ = writeln!(out, "# {}\n", project.name);
            let _ = writeln!(
                out,
                "- Узлов: **{}**, связей: **{}**, компонент: **{}**",
                solver.stats.total_nodes, solver.stats.total_edges, solver.stats.component_count
            );
            let _ = writeln!(
                out,
                "- Суммарно машин (округл.): **{}**, нагрузка SU: **{:.0} SU**\n",
                solver.stats.total_machines_rounded, solver.stats.total_stress_su
            );
            let _ = writeln!(out, "## Баланс ресурсов\n");
            let _ = writeln!(out, "| Ресурс | Потребление | Выход | Цель | Итог |");
            let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        }
        Locale::En => {
            let _ = writeln!(out, "# {}\n", project.name);
            let _ = writeln!(
                out,
                "- Nodes: **{}**, edges: **{}**, components: **{}**",
                solver.stats.total_nodes, solver.stats.total_edges, solver.stats.component_count
            );
            let _ = writeln!(
                out,
                "- Total machines (ceil): **{}**, stress load: **{:.0} SU**\n",
                solver.stats.total_machines_rounded, solver.stats.total_stress_su
            );
            let _ = writeln!(out, "## Resource Balance\n");
            let _ = writeln!(out, "| Resource | Consumed | Produced | Target | Net |");
            let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        }
    }

    for row in &solver.balances {
        let unit = FlowUnit::from_kind(row.kind).label(locale);
        let prod = row.total_source_supply_per_min
            + row.total_produced_main_per_min
            + row.total_produced_byproduct_per_min;
        let _ = writeln!(
            out,
            "| {} | {:.2} {unit} | {:.2} {unit} | {:.2} {unit} | {:+.2} {unit} |",
            row.resource_id,
            row.total_consumed_per_min,
            prod,
            row.total_target_demand_per_min,
            row.net_balance_per_min
        );
    }

    if !solver.warnings.is_empty() {
        let _ = writeln!(
            out,
            "\n## {}\n",
            if locale == Locale::Ru {
                "Предупреждения"
            } else {
                "Warnings"
            }
        );
        for w in &solver.warnings {
            let msg = if locale == Locale::Ru {
                &w.message_ru
            } else {
                &w.message_en
            };
            let _ = writeln!(out, "- [{:?}] {msg}", w.severity);
        }
    }

    out
}

pub fn export_graphviz_dot(project: &FactoryProject, solver: &SolverResult) -> String {
    let mut out = String::from("digraph FactoryAtlas {\n  rankdir=LR;\n");
    for n in &project.nodes {
        let safe_title = n.title.replace('"', "\\\"");
        let label = if let Some(calc) = solver.node_map.get(&n.id) {
            if let Some(m) = calc.assigned_machines {
                format!("{safe_title}\\n({m}x)")
            } else {
                safe_title
            }
        } else {
            safe_title
        };
        let _ = writeln!(out, "  \"{}\" [label=\"{label}\"];", n.id);
    }
    for e in &project.edges {
        let flow = solver
            .edge_map
            .get(&e.id)
            .map(|c| c.flow_per_min)
            .unwrap_or(0.0);
        let _ = writeln!(
            out,
            "  \"{}\" -> \"{}\" [label=\"{} ({flow:.1})\"];",
            e.from_node_id, e.to_node_id, e.resource_id
        );
    }
    out.push_str("}\n");
    out
}
