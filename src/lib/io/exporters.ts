import type { FactoryProject, Locale, ResourceDef, SolverResult } from '../types/atlas';

function escXml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;');
}

function escCsv(val: string | number | boolean | null | undefined): string {
  const s = val === null || val === undefined ? '' : String(val);
  if (/[",\r\n;]/.test(s)) {
    return `"${s.replace(/"/g, '""')}"`;
  }
  return s;
}

/**
 * Generates an offline, self-contained SVG schematic diagram of the production chain.
 */
export function exportProjectToSvg(
  project: FactoryProject,
  solver: SolverResult,
  resourcesById: Map<string, ResourceDef>,
  locale: Locale
): string {
  const nodes = project.nodes;
  const padding = 64;
  const nodeW = 224;
  const nodeH = 112;

  let minX = 0;
  let minY = 0;
  let maxX = 800;
  let maxY = 500;
  if (nodes.length > 0) {
    minX = Math.min(...nodes.map((n) => n.x)) - padding;
    minY = Math.min(...nodes.map((n) => n.y)) - padding - 36;
    maxX = Math.max(...nodes.map((n) => n.x + nodeW)) + padding;
    maxY = Math.max(...nodes.map((n) => n.y + nodeH)) + padding;
  }

  const width = Math.max(640, Math.round(maxX - minX));
  const height = Math.max(400, Math.round(maxY - minY));
  const ox = -minX;
  const oy = -minY;

  const nodeMap = new Map(nodes.map((n) => [n.id, n]));
  const parts: string[] = [];

  parts.push(
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">`
  );
  parts.push(`<rect width="100%" height="100%" fill="#0f172a"/>`);
  parts.push(
    `<defs><pattern id="grid" width="32" height="32" patternUnits="userSpaceOnUse"><path d="M 32 0 L 0 0 0 32" fill="none" stroke="#1e293b" stroke-width="1"/></pattern></defs>`
  );
  parts.push(`<rect width="100%" height="100%" fill="url(#grid)"/>`);
  parts.push(
    `<text x="20" y="28" fill="#f8fafc" font-family="Segoe UI, Arial, sans-serif" font-size="16" font-weight="700">${escXml(
      project.name
    )} — Factory Atlas (${escXml(project.profile.createVersion)})</text>`
  );
  parts.push(
    `<text x="20" y="46" fill="#94a3b8" font-family="Segoe UI, Arial, sans-serif" font-size="11">${
      locale === 'ru'
        ? `Узлов: ${solver.stats.totalNodes} | Связей: ${solver.stats.totalEdges} | Машин: ${solver.stats.totalMachinesRounded} | SU: ${solver.stats.totalStressSU}`
        : `Nodes: ${solver.stats.totalNodes} | Wires: ${solver.stats.totalEdges} | Machines: ${solver.stats.totalMachinesRounded} | SU: ${solver.stats.totalStressSU}`
    }</text>`
  );

  // Edges
  for (const edge of project.edges) {
    const a = nodeMap.get(edge.fromNodeId);
    const b = nodeMap.get(edge.toNodeId);
    if (!a || !b) continue;

    const x1 = Math.round(a.x + ox + nodeW);
    const y1 = Math.round(a.y + oy + nodeH / 2);
    const x2 = Math.round(b.x + ox);
    const y2 = Math.round(b.y + oy + nodeH / 2);
    const dx = Math.max(48, Math.abs(x2 - x1) * 0.45);
    const ec = solver.edgeMap[edge.id];
    const stroke = ec?.isUnitMismatch || ec?.inCycle
      ? '#ef4444'
      : ec?.isBottleneck
        ? '#f97316'
        : edge.resourceKind === 'fluid'
          ? '#38bdf8'
          : '#f59e0b';

    parts.push(
      `<path d="M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}" fill="none" stroke="${stroke}" stroke-width="2.5"${
        edge.isByproductPort ? ' stroke-dasharray="6 4"' : ''
      }/>`
    );

    const mx = Math.round((x1 + x2) / 2);
    const my = Math.round((y1 + y2) / 2);
    const res = resourcesById.get(edge.resourceId);
    const resName = res ? (locale === 'ru' ? res.nameRu : res.nameEn) : edge.resourceId;
    const unitTxt = edge.resourceKind === 'fluid' ? (locale === 'ru' ? 'мБ/м' : 'mB/m') : (locale === 'ru' ? 'шт/м' : '/m');
    const label = `${resName}: ${(ec?.flowPerMin ?? 0).toFixed(1)} ${unitTxt}`;
    parts.push(
      `<rect x="${mx - 74}" y="${my - 11}" width="148" height="20" rx="5" fill="#1e293b" stroke="${stroke}" stroke-width="1"/>`
    );
    parts.push(
      `<text x="${mx}" y="${my + 3}" text-anchor="middle" fill="#e2e8f0" font-family="Segoe UI, Arial, sans-serif" font-size="10">${escXml(
        label
      )}</text>`
    );
  }

  // Nodes
  const kindColors: Record<string, string> = {
    source: '#10b981',
    machine: '#f59e0b',
    recipe: '#d97706',
    byproduct: '#a855f7',
    target: '#38bdf8',
    note: '#64748b',
  };

  for (const n of nodes) {
    const x = Math.round(n.x + ox);
    const y = Math.round(n.y + oy);
    const nc = solver.nodeMap[n.id];
    const border = nc?.inCycle ? '#ef4444' : nc?.isBottleneck ? '#f97316' : (kindColors[n.kind] ?? '#475569');
    parts.push(
      `<rect x="${x}" y="${y}" width="${nodeW}" height="${nodeH}" rx="10" fill="#1e293b" stroke="${border}" stroke-width="2"/>`
    );
    parts.push(
      `<rect x="${x}" y="${y}" width="${nodeW}" height="24" rx="10" fill="#0f172a"/>`
    );
    parts.push(
      `<text x="${x + 10}" y="${y + 16}" fill="${border}" font-family="Segoe UI, Arial, sans-serif" font-size="10" font-weight="700">${escXml(
        n.kind.toUpperCase()
      )}</text>`
    );
    parts.push(
      `<text x="${x + 10}" y="${y + 44}" fill="#f8fafc" font-family="Segoe UI, Arial, sans-serif" font-size="12" font-weight="600">${escXml(
        n.title.slice(0, 28)
      )}</text>`
    );

    let sub = '';
    if (n.kind === 'source') {
      sub = `${n.sourceUnlimited ? '∞' : n.sourceRatePerMin ?? 0} ${n.resourceKind === 'fluid' ? 'mB/min' : 'items/min'}`;
    } else if (n.kind === 'target') {
      const got = nc?.inputs[0]?.suppliedPerMin ?? 0;
      sub = `${got.toFixed(1)} / ${(n.targetRatePerMin ?? 0).toFixed(1)}`;
    } else if (n.kind === 'note') {
      sub = (n.noteText ?? '').slice(0, 32);
    } else if (nc) {
      const machTxt = nc.requiredMachinesExact !== null ? `${nc.requiredMachinesExact.toFixed(2)} (${nc.requiredMachinesCeil})` : '?';
      sub = `${locale === 'ru' ? 'Машин' : 'Machines'}: ${machTxt}${nc.stressTotalSU !== null ? ` · ${nc.stressTotalSU} SU` : ''}`;
    }

    parts.push(
      `<text x="${x + 10}" y="${y + 66}" fill="#cbd5e1" font-family="Segoe UI, Arial, sans-serif" font-size="11">${escXml(
        sub
      )}</text>`
    );

    if (nc?.isBottleneck || nc?.inCycle) {
      parts.push(
        `<text x="${x + 10}" y="${y + 90}" fill="#fca5a5" font-family="Segoe UI, Arial, sans-serif" font-size="10">${escXml(
          nc.inCycle
            ? locale === 'ru'
              ? '⚠ Цикл'
              : '⚠ Cycle'
            : locale === 'ru'
              ? '⚠ Узкое место / Дефицит'
              : '⚠ Bottleneck / Deficit'
        )}</text>`
      );
    }
  }

  parts.push(`</svg>`);
  return parts.join('\n');
}

/**
 * Exports full BOM, resource balances, and machine equipment list as UTF-8 CSV (with BOM for Windows Excel).
 */
export function exportBalancesToCsv(
  project: FactoryProject,
  solver: SolverResult,
  resourcesById: Map<string, ResourceDef>,
  locale: Locale
): string {
  const lines: string[] = [];
  lines.push(
    [
      'Section',
      'ID',
      'Name',
      'Kind',
      'Unit',
      'SourceSupplyPerMin',
      'ConsumedPerMin',
      'MainProducedPerMin',
      'ByproductProducedPerMin',
      'TargetDemandPerMin',
      'NetBalancePerMin',
      'Probabilistic',
    ].join(',')
  );

  for (const b of solver.balances) {
    const res = resourcesById.get(b.resourceId);
    const name = res ? (locale === 'ru' ? res.nameRu : res.nameEn) : b.resourceId;
    lines.push(
      [
        'BALANCE',
        escCsv(b.resourceId),
        escCsv(name),
        escCsv(b.kind),
        escCsv(b.unit),
        b.totalSourceSupplyPerMin.toFixed(3),
        b.totalConsumedPerMin.toFixed(3),
        b.totalProducedMainPerMin.toFixed(3),
        b.totalProducedByproductPerMin.toFixed(3),
        b.totalTargetDemandPerMin.toFixed(3),
        b.netBalancePerMin.toFixed(3),
        b.hasProbabilisticSupply ? 'yes' : 'no',
      ].join(',')
    );
  }

  lines.push('');
  lines.push(
    [
      'Section',
      'NodeID',
      'Title',
      'MachineID',
      'RecipeID',
      'RPM',
      'SpeedSource',
      'CraftsPerMin',
      'MachinesExact',
      'MachinesCeil',
      'StressSU',
      'Bottleneck',
    ].join(',')
  );

  for (const n of project.nodes) {
    if (n.kind !== 'recipe' && n.kind !== 'machine') continue;
    const nc = solver.nodeMap[n.id];
    lines.push(
      [
        'MACHINE_NODE',
        escCsv(n.id),
        escCsv(n.title),
        escCsv(n.machineId ?? ''),
        escCsv(n.recipeId ?? ''),
        n.rpm ?? '',
        escCsv(nc?.speedSource ?? 'unknown'),
        nc ? nc.effectiveCraftsPerMin.toFixed(3) : '',
        nc?.requiredMachinesExact !== null && nc?.requiredMachinesExact !== undefined
          ? nc.requiredMachinesExact.toFixed(3)
          : '',
        nc?.requiredMachinesCeil ?? '',
        nc?.stressTotalSU ?? '',
        nc?.isBottleneck ? 'yes' : 'no',
      ].join(',')
    );
  }

  // UTF-8 BOM ensures Excel on Windows opens Cyrillic without garbling
  return '\uFEFF' + lines.join('\r\n');
}

/**
 * Rasterizes the exported SVG schematic to a PNG Blob in browser environments.
 */
export async function renderSvgToPngBlob(svgText: string, scale = 2): Promise<Blob> {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svgText, 'image/svg+xml');
  const svgEl = doc.documentElement;
  const w = Number(svgEl.getAttribute('width') || 1000);
  const h = Number(svgEl.getAttribute('height') || 600);

  const blob = new Blob([svgText], { type: 'image/svg+xml;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  try {
    const img = new Image();
    img.width = w;
    img.height = h;
    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => reject(new Error('Failed to render SVG image for PNG export'));
      img.src = url;
    });
    const canvas = document.createElement('canvas');
    canvas.width = Math.min(8192, Math.round(w * scale));
    canvas.height = Math.min(8192, Math.round(h * scale));
    const ctx = canvas.getContext('2d');
    if (!ctx) throw new Error('Canvas 2D context unavailable');
    ctx.scale(canvas.width / w, canvas.height / h);
    ctx.drawImage(img, 0, 0);
    return await new Promise<Blob>((resolve, reject) => {
      canvas.toBlob((out) => (out ? resolve(out) : reject(new Error('Canvas toBlob failed'))), 'image/png');
    });
  } finally {
    URL.revokeObjectURL(url);
  }
}
