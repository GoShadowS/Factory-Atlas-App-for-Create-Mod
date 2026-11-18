use crate::machine_speeds::compute_machine_throughput;
use crate::model::{
    EdgeCalculation, FactoryEdge, FactoryNode, FactoryProject, FlowUnit, MachineDef,
    NodeCalculation, NodeFlowLine, NodeKind, ProbabilityKind, RateMode, RecipeDef,
    ResourceBalanceRow, ResourceDef, ResourceKind, SolverResult, SolverStats, SolverWarning,
    SpeedSource, WarningCode, WarningSeverity,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

#[derive(Debug, Default)]
struct CachedComponent {
    node_map: HashMap<String, NodeCalculation>,
    edge_map: HashMap<String, EdgeCalculation>,
    warnings: Vec<SolverWarning>,
    cycles: Vec<Vec<String>>,
}

pub struct IncrementalSolverCache {
    entries: HashMap<String, CachedComponent>,
    lru_order: VecDeque<String>,
    max_entries: usize,
}

impl Default for IncrementalSolverCache {
    fn default() -> Self {
        Self::new(500)
    }
}

impl IncrementalSolverCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            lru_order: VecDeque::new(),
            max_entries: max_entries.max(1),
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.lru_order.clear();
    }

    fn get(&mut self, sig: &str) -> Option<&CachedComponent> {
        if self.entries.contains_key(sig) {
            self.lru_order.retain(|k| k != sig);
            self.lru_order.push_back(sig.to_owned());
            self.entries.get(sig)
        } else {
            None
        }
    }

    fn set(&mut self, sig: String, val: CachedComponent) {
        if self.entries.contains_key(&sig) {
            self.lru_order.retain(|k| k != &sig);
        } else if self.entries.len() >= self.max_entries
            && let Some(oldest) = self.lru_order.pop_front()
        {
            self.entries.remove(&oldest);
        }
        self.lru_order.push_back(sig.clone());
        self.entries.insert(sig, val);
    }
}

pub struct SolverCatalog<'a> {
    pub recipes_by_id: HashMap<String, &'a RecipeDef>,
    pub machines_by_id: HashMap<String, &'a MachineDef>,
    pub resources_by_id: HashMap<String, &'a ResourceDef>,
}

impl<'a> SolverCatalog<'a> {
    pub fn from_project(project: &'a FactoryProject) -> Self {
        Self {
            recipes_by_id: project
                .custom_recipes
                .iter()
                .map(|r| (r.id.clone(), r))
                .collect(),
            machines_by_id: project
                .custom_machines
                .iter()
                .map(|m| (m.id.clone(), m))
                .collect(),
            resources_by_id: project
                .custom_resources
                .iter()
                .map(|r| (r.id.clone(), r))
                .collect(),
        }
    }
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

fn find_cycles_tarjan(node_ids: &[String], adj: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    struct State<'a> {
        adj: &'a HashMap<String, Vec<String>>,
        index: usize,
        indices: HashMap<String, usize>,
        lowlink: HashMap<String, usize>,
        on_stack: HashSet<String>,
        stack: Vec<String>,
        cycles: Vec<Vec<String>>,
    }

    fn strong_connect(v: &str, st: &mut State<'_>) {
        st.indices.insert(v.to_owned(), st.index);
        st.lowlink.insert(v.to_owned(), st.index);
        st.index += 1;
        st.stack.push(v.to_owned());
        st.on_stack.insert(v.to_owned());

        if let Some(neighbors) = st.adj.get(v) {
            for w in neighbors {
                if !st.indices.contains_key(w) {
                    strong_connect(w, st);
                    let lw = st.lowlink[w];
                    if let Some(lv) = st.lowlink.get_mut(v) {
                        *lv = (*lv).min(lw);
                    }
                } else if st.on_stack.contains(w) {
                    let iw = st.indices[w];
                    if let Some(lv) = st.lowlink.get_mut(v) {
                        *lv = (*lv).min(iw);
                    }
                }
            }
        }

        if st.lowlink[v] == st.indices[v] {
            let mut scc = Vec::new();
            while let Some(w) = st.stack.pop() {
                st.on_stack.remove(&w);
                let done = w == v;
                scc.push(w);
                if done {
                    break;
                }
            }
            if scc.len() > 1 {
                st.cycles.push(scc);
            } else if let Some(only) = scc.first()
                && st.adj.get(only).is_some_and(|ns| ns.contains(only))
            {
                st.cycles.push(scc);
            }
        }
    }

    let mut st = State {
        adj,
        index: 0,
        indices: HashMap::new(),
        lowlink: HashMap::new(),
        on_stack: HashSet::new(),
        stack: Vec::new(),
        cycles: Vec::new(),
    };

    for id in node_ids {
        if !st.indices.contains_key(id) {
            strong_connect(id, &mut st);
        }
    }

    st.cycles
}

fn component_signature(
    nodes: &[&FactoryNode],
    edges: &[&FactoryEdge],
    catalog: &SolverCatalog<'_>,
) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(nodes.len() + edges.len() + 2);
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort_by(|a, b| a.id.cmp(&b.id));
    for n in &sorted_nodes {
        let rec_sig = n
            .recipe_id
            .as_deref()
            .and_then(|rid| catalog.recipes_by_id.get(rid))
            .map(|r| {
                format!(
                    "{}:{:?}:{}:{}",
                    r.id,
                    r.processing_time_ticks,
                    r.inputs
                        .iter()
                        .map(|i| format!("{}:{:?}:{}", i.resource_id, i.kind, i.amount))
                        .collect::<Vec<_>>()
                        .join(","),
                    r.outputs
                        .iter()
                        .map(|o| format!(
                            "{}:{:?}:{}:{}:{:?}",
                            o.resource_id, o.kind, o.amount, o.chance, o.weight
                        ))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .unwrap_or_default();
        let mid = n
            .machine_id
            .as_deref()
            .or_else(|| {
                n.recipe_id
                    .as_deref()
                    .and_then(|rid| catalog.recipes_by_id.get(rid))
                    .map(|r| r.machine_id.as_str())
            })
            .unwrap_or("");
        let mach_sig = catalog
            .machines_by_id
            .get(mid)
            .map(|m| format!("{}:{:?}:{:?}", m.id, m.speed_formula_id, m.stress_impact_su))
            .unwrap_or_default();
        parts.push(format!(
            "N:{}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:{:?}:[{}]:[{}]",
            n.id,
            n.kind,
            n.resource_id,
            n.resource_kind,
            n.source_rate_per_min,
            n.source_unlimited,
            n.target_rate_per_min,
            n.rpm,
            n.speed_mode,
            n.manual_crafts_per_min,
            n.machine_count_override,
            rec_sig,
            mach_sig
        ));
    }

    let mut sorted_edges = edges.to_vec();
    sorted_edges.sort_by(|a, b| a.id.cmp(&b.id));
    for e in sorted_edges {
        parts.push(format!(
            "E:{}:{}->{}:{}:{:?}:{:?}:{:?}:{:?}",
            e.id,
            e.from_node_id,
            e.to_node_id,
            e.resource_id,
            e.resource_kind,
            e.rate_mode,
            e.manual_rate_per_min,
            e.is_byproduct_port
        ));
    }
    parts.join("|")
}

fn check_edge_compatibility(
    edge: &FactoryEdge,
    from: &FactoryNode,
    to: &FactoryNode,
    catalog: &SolverCatalog<'_>,
) -> (bool, bool) {
    let mut from_kind = edge.resource_kind;
    if (from.kind == NodeKind::Source
        || from.kind == NodeKind::Target
        || from.kind == NodeKind::Byproduct)
        && let Some(rk) = from.resource_kind
    {
        from_kind = rk;
    } else if matches!(from.kind, NodeKind::Recipe | NodeKind::Machine)
        && let Some(rid) = from.recipe_id.as_deref()
        && let Some(rec) = catalog.recipes_by_id.get(rid)
        && let Some(out) = rec.outputs.iter().find(|o| o.resource_id == edge.resource_id)
    {
        from_kind = out.kind;
    }

    let mut to_kind = edge.resource_kind;
    let mut to_expects_resource: Option<String> = None;
    if to.kind == NodeKind::Target || to.kind == NodeKind::Byproduct {
        if let Some(rk) = to.resource_kind {
            to_kind = rk;
        }
        to_expects_resource = to.resource_id.clone();
    } else if matches!(to.kind, NodeKind::Recipe | NodeKind::Machine)
        && let Some(rid) = to.recipe_id.as_deref()
        && let Some(rec) = catalog.recipes_by_id.get(rid)
        && let Some(inp) = rec.inputs.iter().find(|i| i.resource_id == edge.resource_id)
    {
        to_kind = inp.kind;
    }

    let unit_mismatch = from_kind != edge.resource_kind
        || to_kind != edge.resource_kind
        || from_kind != to_kind
        || catalog
            .resources_by_id
            .get(&edge.resource_id)
            .is_some_and(|r| r.kind != edge.resource_kind);

    let resource_mismatch = to_expects_resource
        .as_deref()
        .is_some_and(|exp| !exp.is_empty() && exp != edge.resource_id);

    (unit_mismatch, resource_mismatch)
}

fn solve_component(
    comp_id: usize,
    nodes: &[&FactoryNode],
    edges: &[&FactoryEdge],
    catalog: &SolverCatalog<'_>,
) -> CachedComponent {
    let node_by_id: HashMap<&str, &FactoryNode> =
        nodes.iter().map(|n| (n.id.as_str(), *n)).collect();
    let mut warnings: Vec<SolverWarning> = Vec::new();
    let mut node_map: HashMap<String, NodeCalculation> = HashMap::new();
    let mut edge_map: HashMap<String, EdgeCalculation> = HashMap::new();

    let mut dir_adj: HashMap<String, Vec<String>> =
        nodes.iter().map(|n| (n.id.clone(), Vec::new())).collect();
    let mut compat_edges: Vec<&FactoryEdge> = Vec::new();
    let mut unit_bad_edges: HashSet<String> = HashSet::new();
    let mut res_bad_edges: HashSet<String> = HashSet::new();

    for e in edges {
        let Some(from) = node_by_id.get(e.from_node_id.as_str()) else {
            continue;
        };
        let Some(to) = node_by_id.get(e.to_node_id.as_str()) else {
            continue;
        };
        let (unit_mismatch, res_mismatch) = check_edge_compatibility(e, from, to, catalog);
        if unit_mismatch {
            unit_bad_edges.insert(e.id.clone());
            warnings.push(SolverWarning {
                id: format!("w-unit-{}", e.id),
                severity: WarningSeverity::Error,
                code: WarningCode::UnitMismatch,
                node_id: Some(to.id.clone()),
                edge_id: Some(e.id.clone()),
                resource_id: Some(e.resource_id.clone()),
                message_ru: format!(
                    "Несовпадение единиц на линии «{}» -> «{}»: нельзя соединять предметы/мин и мБ/мин.",
                    from.title, to.title
                ),
                message_en: format!(
                    "Unit mismatch on edge \"{}\" -> \"{}\": cannot connect items/min and mB/min.",
                    from.title, to.title
                ),
                hint_ru: Some("Проверьте тип ресурса (предмет или жидкость) на обоих узлах.".to_owned()),
                hint_en: Some("Verify item vs fluid unit on both ends.".to_owned()),
            });
            continue;
        }
        if res_mismatch {
            res_bad_edges.insert(e.id.clone());
            warnings.push(SolverWarning {
                id: format!("w-res-{}", e.id),
                severity: WarningSeverity::Warning,
                code: WarningCode::ResourceMismatch,
                node_id: Some(to.id.clone()),
                edge_id: Some(e.id.clone()),
                resource_id: Some(e.resource_id.clone()),
                message_ru: format!(
                    "Несоответствие ресурса на входе «{}»: передаётся «{}».",
                    to.title, e.resource_id
                ),
                message_en: format!(
                    "Resource mismatch on \"{}\": edge carries \"{}\".",
                    to.title, e.resource_id
                ),
                hint_ru: None,
                hint_en: None,
            });
        }
        compat_edges.push(e);
        if let Some(v) = dir_adj.get_mut(&e.from_node_id) {
            v.push(e.to_node_id.clone());
        }
    }

    let node_ids: Vec<String> = nodes.iter().map(|n| n.id.clone()).collect();
    let cycles = find_cycles_tarjan(&node_ids, &dir_adj);
    let mut cycle_nodes: HashSet<String> = HashSet::new();
    for (idx, c) in cycles.iter().enumerate() {
        for id in c {
            cycle_nodes.insert(id.clone());
        }
        let names = c
            .iter()
            .map(|id| node_by_id.get(id.as_str()).map(|n| n.title.as_str()).unwrap_or(id.as_str()))
            .collect::<Vec<_>>()
            .join(" -> ");
        warnings.push(SolverWarning {
            id: format!("w-cycle-{comp_id}-{idx}"),
            severity: WarningSeverity::Error,
            code: WarningCode::CycleDetected,
            node_id: c.first().cloned(),
            edge_id: None,
            resource_id: None,
            message_ru: format!("Обнаружен цикл потоков (Тарьян SCC): {names}."),
            message_en: format!("Detected cycle in production graph (Tarjan SCC): {names}."),
            hint_ru: Some(
                "Разорвите петлю обратной связи или отметьте один из возвратов как побочный выход."
                    .to_owned(),
            ),
            hint_en: Some("Break the feedback loop to compute deterministic steady-state rates.".to_owned()),
        });
    }

    // Topological order on acyclic subset
    let mut in_deg: HashMap<String, usize> =
        nodes.iter().map(|n| (n.id.clone(), 0usize)).collect();
    for e in &compat_edges {
        if !cycle_nodes.contains(&e.from_node_id)
            && !cycle_nodes.contains(&e.to_node_id)
            && let Some(d) = in_deg.get_mut(&e.to_node_id)
        {
            *d += 1;
        }
    }
    let mut queue: VecDeque<String> = in_deg
        .iter()
        .filter(|(id, deg)| **deg == 0 && !cycle_nodes.contains(*id))
        .map(|(id, _)| id.clone())
        .collect();
    let mut topo_order: Vec<String> = Vec::new();
    while let Some(u) = queue.pop_front() {
        topo_order.push(u.clone());
        if let Some(ns) = dir_adj.get(&u) {
            for v in ns {
                if !cycle_nodes.contains(v)
                    && let Some(d) = in_deg.get_mut(v)
                {
                    *d = d.saturating_sub(1);
                    if *d == 0 {
                        queue.push_back(v.clone());
                    }
                }
            }
        }
    }

    let mut outgoing_by_node: HashMap<&str, Vec<&FactoryEdge>> = HashMap::new();
    let mut incoming_by_node: HashMap<&str, Vec<&FactoryEdge>> = HashMap::new();
    for e in &compat_edges {
        outgoing_by_node
            .entry(e.from_node_id.as_str())
            .or_default()
            .push(e);
        incoming_by_node
            .entry(e.to_node_id.as_str())
            .or_default()
            .push(e);
    }

    // 1. Backward Demand Pass (Target -> Recipe -> Source)
    let mut node_demand_res: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut requested_crafts: HashMap<String, f64> = HashMap::new();
    let mut edge_demand: HashMap<String, f64> = HashMap::new();

    for id in topo_order.iter().rev() {
        let node = node_by_id[id.as_str()];
        let out_edges = outgoing_by_node.get(id.as_str()).map(Vec::as_slice).unwrap_or(&[]);

        if node.kind == NodeKind::Target {
            let tgt_rate = node.target_rate_per_min.unwrap_or(0.0).max(0.0);
            if let Some(rid) = node.resource_id.as_deref().filter(|s| !s.is_empty()) {
                node_demand_res
                    .entry(node.id.clone())
                    .or_default()
                    .insert(rid.to_owned(), tgt_rate);
            }
            let in_edges = incoming_by_node.get(id.as_str()).map(Vec::as_slice).unwrap_or(&[]);
            if in_edges.is_empty() && tgt_rate > 0.0 {
                warnings.push(SolverWarning {
                    id: format!("w-disc-{}", node.id),
                    severity: WarningSeverity::Warning,
                    code: WarningCode::DisconnectedTarget,
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                    resource_id: node.resource_id.clone(),
                    message_ru: format!(
                        "Цель «{}» ({tgt_rate}/мин) не подключена к источнику или рецепту.",
                        node.title
                    ),
                    message_en: format!(
                        "Target \"{}\" ({tgt_rate}/min) has no incoming supply lines.",
                        node.title
                    ),
                    hint_ru: None,
                    hint_en: None,
                });
            }
            if !in_edges.is_empty() && tgt_rate > 0.0 {
                let share = tgt_rate / in_edges.len() as f64;
                for e in in_edges {
                    let capped = if e.rate_mode == RateMode::Manual {
                        share.min(e.manual_rate_per_min.unwrap_or(share))
                    } else {
                        share
                    };
                    edge_demand.insert(e.id.clone(), capped);
                }
            }
            continue;
        }

        if matches!(node.kind, NodeKind::Recipe | NodeKind::Machine) {
            let rec = node
                .recipe_id
                .as_deref()
                .and_then(|rid| catalog.recipes_by_id.get(rid).copied());
            let mut crafts_needed = 0.0_f64;

            if let Some(rec) = rec {
                let mut total_weight = 0.0;
                for o in &rec.outputs {
                    if o.probability_kind == ProbabilityKind::Weighted {
                        total_weight += o.weight.unwrap_or(1.0);
                    }
                }
                for out_e in out_edges {
                    let d = *edge_demand.get(&out_e.id).unwrap_or(&0.0);
                    if d <= 0.0 {
                        continue;
                    }
                    if let Some(out_def) =
                        rec.outputs.iter().find(|o| o.resource_id == out_e.resource_id)
                    {
                        let p = if out_def.probability_kind == ProbabilityKind::Weighted
                            && total_weight > 0.0
                        {
                            out_def.weight.unwrap_or(1.0) / total_weight
                        } else {
                            out_def.chance
                        };
                        let per_craft = out_def.amount * p;
                        if per_craft > 0.0 {
                            crafts_needed = crafts_needed.max(d / per_craft);
                        }
                    }
                }
                requested_crafts.insert(node.id.clone(), crafts_needed);

                for inp in &rec.inputs {
                    if inp.amount <= 0.0 {
                        continue;
                    }
                    let req_amt = crafts_needed * inp.amount;
                    let entry = node_demand_res
                        .entry(node.id.clone())
                        .or_default()
                        .entry(inp.resource_id.clone())
                        .or_insert(0.0);
                    *entry += req_amt;

                    let matching_in: Vec<&FactoryEdge> = incoming_by_node
                        .get(id.as_str())
                        .map(|v| {
                            v.iter()
                                .copied()
                                .filter(|e| e.resource_id == inp.resource_id)
                                .collect()
                        })
                        .unwrap_or_default();

                    if !matching_in.is_empty() && req_amt > 0.0 {
                        let share = req_amt / matching_in.len() as f64;
                        for ie in matching_in {
                            let capped = if ie.rate_mode == RateMode::Manual {
                                share.min(ie.manual_rate_per_min.unwrap_or(share))
                            } else {
                                share
                            };
                            *edge_demand.entry(ie.id.clone()).or_insert(0.0) += capped;
                        }
                    }
                }
            }
        }
    }

    // 2. Forward Supply Pass
    let mut edge_supply: HashMap<String, f64> = HashMap::new();
    let mut edge_actual: HashMap<String, f64> = HashMap::new();

    for id in &topo_order {
        let node = node_by_id[id.as_str()];
        let out_edges = outgoing_by_node.get(id.as_str()).map(Vec::as_slice).unwrap_or(&[]);
        let in_edges = incoming_by_node.get(id.as_str()).map(Vec::as_slice).unwrap_or(&[]);

        let mut node_warnings: Vec<String> = Vec::new();
        let mut inputs_calc: Vec<NodeFlowLine> = Vec::new();
        let mut outputs_calc: Vec<NodeFlowLine> = Vec::new();

        if node.kind == NodeKind::Note {
            node_map.insert(
                node.id.clone(),
                NodeCalculation {
                    node_id: node.id.clone(),
                    component_id: comp_id,
                    in_cycle: false,
                    requested_crafts_per_min: 0.0,
                    effective_crafts_per_min: 0.0,
                    crafts_per_machine_per_min: None,
                    speed_source: SpeedSource::NotApplicable,
                    required_machines_exact: None,
                    required_machines_ceil: None,
                    assigned_machines: None,
                    utilization_ratio: None,
                    stress_total_su: None,
                    is_bottleneck: false,
                    deficit_detected: false,
                    inputs: Vec::new(),
                    outputs: Vec::new(),
                    warnings: Vec::new(),
                },
            );
            continue;
        }

        if node.kind == NodeKind::Source {
            let rk = node.resource_kind.unwrap_or(ResourceKind::Item);
            let rid = node.resource_id.clone().unwrap_or_default();
            let total_demanded: f64 = out_edges
                .iter()
                .map(|e| *edge_demand.get(&e.id).unwrap_or(&0.0))
                .sum();
            let available = if node.source_unlimited.unwrap_or(false) {
                total_demanded.max(node.source_rate_per_min.unwrap_or(0.0))
            } else {
                node.source_rate_per_min.unwrap_or(0.0).max(0.0)
            };

            let mut deficit = false;
            if total_demanded > available + 1e-6 && !node.source_unlimited.unwrap_or(false) {
                deficit = true;
                let wid = format!("w-src-def-{}", node.id);
                node_warnings.push(wid.clone());
                warnings.push(SolverWarning {
                    id: wid,
                    severity: WarningSeverity::Error,
                    code: WarningCode::Deficit,
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                    resource_id: Some(rid.clone()),
                    message_ru: format!(
                        "Дефицит на источнике «{}»: требуется {total_demanded:.1}, подаётся {available:.1} {}.",
                        node.title,
                        FlowUnit::from_kind(rk).label(crate::model::Locale::Ru)
                    ),
                    message_en: format!(
                        "Deficit at source \"{}\": required {total_demanded:.1}, supply {available:.1}.",
                        node.title
                    ),
                    hint_ru: None,
                    hint_en: None,
                });
            }

            if !out_edges.is_empty() {
                let ratio = if total_demanded > 0.0 {
                    (available / total_demanded).min(1.0)
                } else {
                    1.0
                };
                for oe in out_edges {
                    let d = *edge_demand.get(&oe.id).unwrap_or(&0.0);
                    let cap = if oe.rate_mode == RateMode::Manual {
                        oe.manual_rate_per_min.unwrap_or(available)
                    } else {
                        f64::INFINITY
                    };
                    let flow = if total_demanded > 0.0 {
                        (d * ratio).min(cap)
                    } else {
                        (available / out_edges.len() as f64).min(cap)
                    };
                    edge_supply.insert(oe.id.clone(), flow);
                    edge_actual.insert(oe.id.clone(), flow);
                }
            }

            outputs_calc.push(NodeFlowLine {
                resource_id: rid,
                kind: rk,
                unit: FlowUnit::from_kind(rk),
                required_per_min: total_demanded,
                supplied_per_min: available,
                produced_expected_per_min: available,
                produced_guaranteed_per_min: available,
                probabilistic: false,
                is_byproduct: false,
            });

            node_map.insert(
                node.id.clone(),
                NodeCalculation {
                    node_id: node.id.clone(),
                    component_id: comp_id,
                    in_cycle: false,
                    requested_crafts_per_min: 0.0,
                    effective_crafts_per_min: available,
                    crafts_per_machine_per_min: None,
                    speed_source: SpeedSource::NotApplicable,
                    required_machines_exact: None,
                    required_machines_ceil: None,
                    assigned_machines: None,
                    utilization_ratio: None,
                    stress_total_su: None,
                    is_bottleneck: deficit,
                    deficit_detected: deficit,
                    inputs: Vec::new(),
                    outputs: outputs_calc,
                    warnings: node_warnings,
                },
            );
            continue;
        }

        if node.kind == NodeKind::Target || node.kind == NodeKind::Byproduct {
            let rk = node.resource_kind.unwrap_or(ResourceKind::Item);
            let rid = node.resource_id.clone().unwrap_or_default();
            let demanded = if node.kind == NodeKind::Target {
                node.target_rate_per_min.unwrap_or(0.0).max(0.0)
            } else {
                0.0
            };
            let supplied: f64 = in_edges
                .iter()
                .map(|e| *edge_actual.get(&e.id).unwrap_or(&0.0))
                .sum();

            let mut deficit = false;
            if node.kind == NodeKind::Target && demanded > 0.0 && supplied + 1e-6 < demanded {
                deficit = true;
                let wid = format!("w-tgt-def-{}", node.id);
                node_warnings.push(wid.clone());
                warnings.push(SolverWarning {
                    id: wid,
                    severity: WarningSeverity::Error,
                    code: WarningCode::Deficit,
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                    resource_id: Some(rid.clone()),
                    message_ru: format!(
                        "Недобор цели «{}»: поступает {supplied:.1} из {demanded:.1} {}.",
                        node.title,
                        FlowUnit::from_kind(rk).label(crate::model::Locale::Ru)
                    ),
                    message_en: format!(
                        "Target deficit on \"{}\": receiving {supplied:.1} of {demanded:.1}.",
                        node.title
                    ),
                    hint_ru: None,
                    hint_en: None,
                });
            }

            inputs_calc.push(NodeFlowLine {
                resource_id: rid,
                kind: rk,
                unit: FlowUnit::from_kind(rk),
                required_per_min: demanded,
                supplied_per_min: supplied,
                produced_expected_per_min: supplied,
                produced_guaranteed_per_min: supplied,
                probabilistic: false,
                is_byproduct: node.kind == NodeKind::Byproduct,
            });

            node_map.insert(
                node.id.clone(),
                NodeCalculation {
                    node_id: node.id.clone(),
                    component_id: comp_id,
                    in_cycle: false,
                    requested_crafts_per_min: 0.0,
                    effective_crafts_per_min: supplied,
                    crafts_per_machine_per_min: None,
                    speed_source: SpeedSource::NotApplicable,
                    required_machines_exact: None,
                    required_machines_ceil: None,
                    assigned_machines: None,
                    utilization_ratio: None,
                    stress_total_su: None,
                    is_bottleneck: false,
                    deficit_detected: deficit,
                    inputs: inputs_calc,
                    outputs: Vec::new(),
                    warnings: node_warnings,
                },
            );
            continue;
        }

        // Recipe / Machine node
        let rec = node
            .recipe_id
            .as_deref()
            .and_then(|rid| catalog.recipes_by_id.get(rid).copied());
        let mach_id = node
            .machine_id
            .as_deref()
            .or_else(|| rec.map(|r| r.machine_id.as_str()))
            .unwrap_or("");
        let mach = catalog.machines_by_id.get(mach_id).copied();

        if node.kind == NodeKind::Recipe && rec.is_none() {
            let wid = format!("w-miss-rec-{}", node.id);
            node_warnings.push(wid.clone());
            warnings.push(SolverWarning {
                id: wid,
                severity: WarningSeverity::Warning,
                code: WarningCode::MissingRecipe,
                node_id: Some(node.id.clone()),
                edge_id: None,
                resource_id: None,
                message_ru: format!("У операции «{}» не выбран рецепт.", node.title),
                message_en: format!("Recipe node \"{}\" has no recipe selected.", node.title),
                hint_ru: None,
                hint_en: None,
            });
        }

        let speed_calc = compute_machine_throughput(
            mach,
            rec,
            node.rpm,
            node.speed_mode,
            node.manual_crafts_per_min,
            node.stack_size_assumption,
        );

        if speed_calc.source == SpeedSource::Unknown {
            let wid = format!("w-unk-spd-{}", node.id);
            node_warnings.push(wid.clone());
            warnings.push(SolverWarning {
                id: wid,
                severity: WarningSeverity::Warning,
                code: WarningCode::UnknownSpeed,
                node_id: Some(node.id.clone()),
                edge_id: None,
                resource_id: None,
                message_ru: format!("{}: {}", node.title, speed_calc.explanation_ru),
                message_en: format!("{}: {}", node.title, speed_calc.explanation_en),
                hint_ru: Some("Выберите режим «Ручной темп» в инспекторе узла.".to_owned()),
                hint_en: Some("Switch speed mode to Manual in the inspector.".to_owned()),
            });
        } else if speed_calc.source == SpeedSource::Assumption {
            let wid = format!("w-assump-{}", node.id);
            node_warnings.push(wid.clone());
            warnings.push(SolverWarning {
                id: wid,
                severity: WarningSeverity::Info,
                code: WarningCode::AssumptionActive,
                node_id: Some(node.id.clone()),
                edge_id: None,
                resource_id: None,
                message_ru: format!("{}: {}", node.title, speed_calc.explanation_ru),
                message_en: format!("{}: {}", node.title, speed_calc.explanation_en),
                hint_ru: None,
                hint_en: None,
            });
        }

        let req_crafts = *requested_crafts.get(&node.id).unwrap_or(&0.0);
        let mut max_crafts_by_input = if req_crafts > 0.0 {
            req_crafts
        } else {
            f64::INFINITY
        };

        if let Some(rec) = rec {
            for inp in &rec.inputs {
                if inp.amount <= 0.0 {
                    continue;
                }
                let supplied_amt: f64 = in_edges
                    .iter()
                    .filter(|e| e.resource_id == inp.resource_id)
                    .map(|e| *edge_actual.get(&e.id).unwrap_or(&0.0))
                    .sum();
                let crafts_from_in = supplied_amt / inp.amount;
                max_crafts_by_input = max_crafts_by_input.min(crafts_from_in);
            }
            if !max_crafts_by_input.is_finite() {
                max_crafts_by_input = 0.0;
            }
        } else {
            max_crafts_by_input = 0.0;
        }

        let crafts_per_machine = speed_calc.crafts_per_min;
        let mut max_crafts_by_machines = f64::INFINITY;
        let mut is_bottleneck = false;
        if let Some(cap_count) = node.machine_count_override
            && let Some(cpm) = crafts_per_machine
        {
            max_crafts_by_machines = cpm * f64::from(cap_count);
            if req_crafts > max_crafts_by_machines + 1e-6 {
                is_bottleneck = true;
                let wid = format!("w-bot-{}", node.id);
                node_warnings.push(wid.clone());
                warnings.push(SolverWarning {
                    id: wid,
                    severity: WarningSeverity::Warning,
                    code: WarningCode::Bottleneck,
                    node_id: Some(node.id.clone()),
                    edge_id: None,
                    resource_id: None,
                    message_ru: format!(
                        "Узкое место «{}»: {cap_count} маш. дают {max_crafts_by_machines:.1} опер./мин при спросе {req_crafts:.1}.",
                        node.title
                    ),
                    message_en: format!(
                        "Bottleneck at \"{}\": {cap_count} machines yield {max_crafts_by_machines:.1} ops/min vs demand {req_crafts:.1}.",
                        node.title
                    ),
                    hint_ru: None,
                    hint_en: None,
                });
            }
        }

        let effective_crafts = if req_crafts > 0.0 {
            req_crafts.min(max_crafts_by_input).min(max_crafts_by_machines)
        } else {
            max_crafts_by_input.min(max_crafts_by_machines)
        };

        let mut deficit_detected = false;
        if let Some(rec) = rec {
            for inp in &rec.inputs {
                if inp.amount <= 0.0 {
                    continue;
                }
                let req_amt = req_crafts * inp.amount;
                let supplied_amt: f64 = in_edges
                    .iter()
                    .filter(|e| e.resource_id == inp.resource_id)
                    .map(|e| *edge_actual.get(&e.id).unwrap_or(&0.0))
                    .sum();
                if req_amt > supplied_amt + 1e-6 {
                    deficit_detected = true;
                }
                inputs_calc.push(NodeFlowLine {
                    resource_id: inp.resource_id.clone(),
                    kind: inp.kind,
                    unit: FlowUnit::from_kind(inp.kind),
                    required_per_min: req_amt,
                    supplied_per_min: supplied_amt,
                    produced_expected_per_min: 0.0,
                    produced_guaranteed_per_min: 0.0,
                    probabilistic: false,
                    is_byproduct: false,
                });
            }

            let mut total_weight = 0.0;
            for o in &rec.outputs {
                if o.probability_kind == ProbabilityKind::Weighted {
                    total_weight += o.weight.unwrap_or(1.0);
                }
            }

            for (idx, out) in rec.outputs.iter().enumerate() {
                let p = if out.probability_kind == ProbabilityKind::Weighted && total_weight > 0.0 {
                    out.weight.unwrap_or(1.0) / total_weight
                } else {
                    out.chance
                };
                let expected = effective_crafts * out.amount * p;
                let guaranteed = if p >= 0.9999 { expected } else { 0.0 };
                let probabilistic = p < 0.9999;

                if probabilistic && effective_crafts > 0.0 {
                    let wid = format!("w-prob-{}-{}", node.id, out.resource_id);
                    node_warnings.push(wid.clone());
                    warnings.push(SolverWarning {
                        id: wid,
                        severity: WarningSeverity::Info,
                        code: WarningCode::ProbabilisticOutput,
                        node_id: Some(node.id.clone()),
                        edge_id: None,
                        resource_id: Some(out.resource_id.clone()),
                        message_ru: format!(
                            "Вероятностный выход «{}» ({:.1}%): расчёт {expected:.2}/мин — ожидаемое среднее.",
                            out.resource_id,
                            p * 100.0
                        ),
                        message_en: format!(
                            "Probabilistic output \"{}\" ({:.1}%): {expected:.2}/min is expected mean.",
                            out.resource_id,
                            p * 100.0
                        ),
                        hint_ru: None,
                        hint_en: None,
                    });
                }

                outputs_calc.push(NodeFlowLine {
                    resource_id: out.resource_id.clone(),
                    kind: out.kind,
                    unit: FlowUnit::from_kind(out.kind),
                    required_per_min: 0.0,
                    supplied_per_min: expected,
                    produced_expected_per_min: expected,
                    produced_guaranteed_per_min: guaranteed,
                    probabilistic,
                    is_byproduct: idx > 0,
                });

                let match_out: Vec<&FactoryEdge> = out_edges
                    .iter()
                    .copied()
                    .filter(|e| e.resource_id == out.resource_id)
                    .collect();
                if !match_out.is_empty() {
                    let total_dem: f64 = match_out
                        .iter()
                        .map(|e| *edge_demand.get(&e.id).unwrap_or(&0.0))
                        .sum();
                    for oe in &match_out {
                        let d = *edge_demand.get(&oe.id).unwrap_or(&0.0);
                        let cap = if oe.rate_mode == RateMode::Manual {
                            oe.manual_rate_per_min.unwrap_or(expected)
                        } else {
                            f64::INFINITY
                        };
                        let flow = if total_dem > 0.0 {
                            (expected * (d / total_dem)).min(cap)
                        } else {
                            (expected / match_out.len() as f64).min(cap)
                        };
                        edge_supply.insert(oe.id.clone(), flow);
                        edge_actual.insert(oe.id.clone(), flow);
                    }
                }
            }
        }

        let target_crafts = if req_crafts > 0.0 {
            req_crafts
        } else {
            effective_crafts
        };
        let req_machines_exact = crafts_per_machine
            .filter(|cpm| *cpm > 0.0)
            .map(|cpm| target_crafts / cpm);
        let req_machines_ceil = req_machines_exact.map(|x| x.ceil().max(0.0) as u32);
        let assigned_machines = node.machine_count_override.or(req_machines_ceil);
        let utilization_ratio = match (assigned_machines, req_machines_exact) {
            (Some(a), Some(e)) if a > 0 => Some((e / f64::from(a)).min(1.0)),
            _ => None,
        };
        let stress_total_su = match (speed_calc.stress_per_machine_su, assigned_machines) {
            (Some(su), Some(a)) => Some(su * f64::from(a)),
            _ => None,
        };

        node_map.insert(
            node.id.clone(),
            NodeCalculation {
                node_id: node.id.clone(),
                component_id: comp_id,
                in_cycle: false,
                requested_crafts_per_min: req_crafts,
                effective_crafts_per_min: effective_crafts,
                crafts_per_machine_per_min: crafts_per_machine,
                speed_source: speed_calc.source,
                required_machines_exact: req_machines_exact,
                required_machines_ceil: req_machines_ceil,
                assigned_machines,
                utilization_ratio,
                stress_total_su,
                is_bottleneck,
                deficit_detected,
                inputs: inputs_calc,
                outputs: outputs_calc,
                warnings: node_warnings,
            },
        );
    }

    // Nodes in cycles
    for cid in &cycle_nodes {
        node_map.insert(
            cid.clone(),
            NodeCalculation {
                node_id: cid.clone(),
                component_id: comp_id,
                in_cycle: true,
                requested_crafts_per_min: 0.0,
                effective_crafts_per_min: 0.0,
                crafts_per_machine_per_min: None,
                speed_source: SpeedSource::Unknown,
                required_machines_exact: None,
                required_machines_ceil: None,
                assigned_machines: None,
                utilization_ratio: None,
                stress_total_su: None,
                is_bottleneck: false,
                deficit_detected: false,
                inputs: Vec::new(),
                outputs: Vec::new(),
                warnings: Vec::new(),
            },
        );
    }

    for e in edges {
        let demand = *edge_demand.get(&e.id).unwrap_or(&0.0);
        let supply = *edge_supply.get(&e.id).unwrap_or(&0.0);
        let flow = *edge_actual.get(&e.id).unwrap_or(&0.0);
        let in_cycle = cycle_nodes.contains(&e.from_node_id) || cycle_nodes.contains(&e.to_node_id);
        let is_unit_mismatch = unit_bad_edges.contains(&e.id);
        let is_resource_mismatch = res_bad_edges.contains(&e.id);
        let deficit = (demand - flow).max(0.0);
        edge_map.insert(
            e.id.clone(),
            EdgeCalculation {
                edge_id: e.id.clone(),
                resource_id: e.resource_id.clone(),
                resource_kind: e.resource_kind,
                unit: FlowUnit::from_kind(e.resource_kind),
                flow_per_min: flow,
                demand_per_min: demand,
                supply_per_min: supply,
                deficit_per_min: deficit,
                is_unit_mismatch,
                is_resource_mismatch,
                is_bottleneck: deficit > 1e-6,
                in_cycle,
            },
        );
    }

    CachedComponent {
        node_map,
        edge_map,
        warnings,
        cycles,
    }
}

fn get_or_insert_bal<'m>(
    bal_map: &'m mut HashMap<(String, ResourceKind), ResourceBalanceRow>,
    rid: &str,
    kind: ResourceKind,
) -> &'m mut ResourceBalanceRow {
    bal_map
        .entry((rid.to_owned(), kind))
        .or_insert_with(|| ResourceBalanceRow {
            resource_id: rid.to_owned(),
            kind,
            unit: FlowUnit::from_kind(kind),
            total_source_supply_per_min: 0.0,
            total_consumed_per_min: 0.0,
            total_produced_main_per_min: 0.0,
            total_produced_byproduct_per_min: 0.0,
            total_target_demand_per_min: 0.0,
            net_balance_per_min: 0.0,
            has_probabilistic_supply: false,
        })
}

pub fn solve_factory_project(
    project: &FactoryProject,
    catalog: &SolverCatalog<'_>,
    cache: Option<&mut IncrementalSolverCache>,
) -> SolverResult {
    let t0 = Instant::now();
    let n = project.nodes.len();
    let mut uf = UnionFind::new(n);
    let idx_by_id: HashMap<&str, usize> = project
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id.as_str(), i))
        .collect();

    for e in &project.edges {
        if let (Some(&ia), Some(&ib)) = (
            idx_by_id.get(e.from_node_id.as_str()),
            idx_by_id.get(e.to_node_id.as_str()),
        ) {
            uf.union(ia, ib);
        }
    }

    let mut comp_nodes: HashMap<usize, Vec<&FactoryNode>> = HashMap::new();
    for (i, node) in project.nodes.iter().enumerate() {
        let root = uf.find(i);
        comp_nodes.entry(root).or_default().push(node);
    }
    let mut comp_edges: HashMap<usize, Vec<&FactoryEdge>> = HashMap::new();
    for e in &project.edges {
        if let Some(&ia) = idx_by_id.get(e.from_node_id.as_str()) {
            let root = uf.find(ia);
            comp_edges.entry(root).or_default().push(e);
        }
    }

    let mut node_map: HashMap<String, NodeCalculation> = HashMap::new();
    let mut edge_map: HashMap<String, EdgeCalculation> = HashMap::new();
    let mut warnings: Vec<SolverWarning> = Vec::new();
    let mut cycles: Vec<Vec<String>> = Vec::new();
    let mut recomputed = 0usize;
    let mut cached_hits = 0usize;

    let mut local_cache;
    let cache_ref = match cache {
        Some(c) => c,
        None => {
            local_cache = IncrementalSolverCache::default();
            &mut local_cache
        }
    };

    let mut sorted_roots: Vec<usize> = comp_nodes.keys().copied().collect();
    sorted_roots.sort_unstable();

    for (comp_idx, root) in sorted_roots.iter().enumerate() {
        let nodes = &comp_nodes[root];
        let edges = comp_edges.get(root).map(Vec::as_slice).unwrap_or(&[]);
        let sig = component_signature(nodes, edges, catalog);

        if let Some(hit) = cache_ref.get(&sig) {
            cached_hits += 1;
            node_map.extend(hit.node_map.clone());
            edge_map.extend(hit.edge_map.clone());
            warnings.extend(hit.warnings.clone());
            cycles.extend(hit.cycles.clone());
        } else {
            recomputed += 1;
            let solved = solve_component(comp_idx, nodes, edges, catalog);
            node_map.extend(solved.node_map.clone());
            edge_map.extend(solved.edge_map.clone());
            warnings.extend(solved.warnings.clone());
            cycles.extend(solved.cycles.clone());
            cache_ref.set(sig, solved);
        }
    }

    // Global resource balance aggregation (strict separation of item vs fluid)
    let mut bal_map: HashMap<(String, ResourceKind), ResourceBalanceRow> = HashMap::new();

    for node in &project.nodes {
        let Some(calc) = node_map.get(&node.id) else {
            continue;
        };
        if node.kind == NodeKind::Source {
            for o in &calc.outputs {
                if !o.resource_id.is_empty() {
                    get_or_insert_bal(&mut bal_map, &o.resource_id, o.kind)
                        .total_source_supply_per_min += o.supplied_per_min;
                }
            }
        } else if node.kind == NodeKind::Target {
            for i in &calc.inputs {
                if !i.resource_id.is_empty() {
                    get_or_insert_bal(&mut bal_map, &i.resource_id, i.kind)
                        .total_target_demand_per_min += i.required_per_min;
                }
            }
        } else if matches!(node.kind, NodeKind::Recipe | NodeKind::Machine) {
            for i in &calc.inputs {
                get_or_insert_bal(&mut bal_map, &i.resource_id, i.kind).total_consumed_per_min +=
                    i.required_per_min;
            }
            for o in &calc.outputs {
                let row = get_or_insert_bal(&mut bal_map, &o.resource_id, o.kind);
                if o.is_byproduct {
                    row.total_produced_byproduct_per_min += o.produced_expected_per_min;
                } else {
                    row.total_produced_main_per_min += o.produced_expected_per_min;
                }
                if o.probabilistic {
                    row.has_probabilistic_supply = true;
                }
            }
        }
    }

    let mut balances: Vec<ResourceBalanceRow> = bal_map
        .into_values()
        .map(|mut b| {
            b.net_balance_per_min = b.total_source_supply_per_min
                + b.total_produced_main_per_min
                + b.total_produced_byproduct_per_min
                - b.total_consumed_per_min
                - b.total_target_demand_per_min;
            b
        })
        .collect();
    balances.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));

    let mut total_stress_su = 0.0;
    let mut total_machines_rounded = 0u32;
    for calc in node_map.values() {
        if let Some(su) = calc.stress_total_su {
            total_stress_su += su;
        }
        if let Some(m) = calc.assigned_machines {
            total_machines_rounded = total_machines_rounded.saturating_add(m);
        }
    }

    let stats = SolverStats {
        total_nodes: project.nodes.len(),
        total_edges: project.edges.len(),
        component_count: sorted_roots.len(),
        recomputed_components: recomputed,
        cached_components: cached_hits,
        solve_time_ms: t0.elapsed().as_secs_f64() * 1000.0,
        total_stress_su,
        total_machines_rounded,
        cycle_count: cycles.len(),
    };

    SolverResult {
        node_map,
        edge_map,
        balances,
        warnings,
        cycles,
        stats,
    }
}
