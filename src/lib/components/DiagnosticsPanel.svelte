<script lang="ts">
  import { t } from '../i18n/translations';
  import type {
    AssetManifestEntry,
    FactoryNode,
    Locale,
    ResourceDef,
    SolverResult,
  } from '../types/atlas';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    locale: Locale;
    solver: SolverResult;
    nodes: FactoryNode[];
    resourcesById: Map<string, ResourceDef>;
    assetsById: Map<string, AssetManifestEntry>;
    onFocusNode: (nodeId: string) => void;
  }

  let { locale, solver, nodes, resourcesById, assetsById, onFocusNode }: Props = $props();
  const tr = $derived(t(locale));

  let collapsed = $state(false);
  let activeTab = $state<'balances' | 'machines' | 'warnings'>('balances');

  const machineNodes = $derived(
    nodes.filter((n) => n.kind === 'recipe' || n.kind === 'machine')
  );

  function resName(id: string): string {
    const r = resourcesById.get(id);
    return r ? (locale === 'ru' ? r.nameRu : r.nameEn) : id;
  }
</script>

<section class="diagnostics-dock" class:collapsed aria-label={tr.diagnosticsTitle}>
  <div class="dock-header">
    <div class="dock-tabs" role="tablist">
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'balances'}
        class:active={activeTab === 'balances'}
        onclick={() => {
          activeTab = 'balances';
          collapsed = false;
        }}
      >
        {tr.tabBalances} ({solver.balances.length})
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'machines'}
        class:active={activeTab === 'machines'}
        onclick={() => {
          activeTab = 'machines';
          collapsed = false;
        }}
      >
        {tr.tabMachines} ({solver.stats.totalMachinesRounded} · {solver.stats.totalStressSU} SU)
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === 'warnings'}
        class:active={activeTab === 'warnings'}
        onclick={() => {
          activeTab = 'warnings';
          collapsed = false;
        }}
      >
        {tr.tabWarnings}
        {#if solver.warnings.length > 0}
          <span class="warn-count">{solver.warnings.length}</span>
        {/if}
      </button>
    </div>

    <div class="dock-stats">
      <span>
        {locale === 'ru' ? 'Расчёт' : 'Solve'}: {solver.stats.solveTimeMs.toFixed(2)} ms
        ({solver.stats.cachedComponents}/{solver.stats.componentCount} cached)
      </span>
      <button type="button" class="btn-ghost btn-xs" onclick={() => (collapsed = !collapsed)}>
        {collapsed ? '▲' : '▼'}
      </button>
    </div>
  </div>

  {#if !collapsed}
    <div class="dock-body">
      {#if activeTab === 'balances'}
        {#if solver.balances.length === 0}
          <div class="empty-msg">
            {locale === 'ru' ? 'Добавьте и соедините узлы для расчёта баланса.' : 'Add and connect nodes to compute resource balances.'}
          </div>
        {:else}
          <table class="diag-table">
            <thead>
              <tr>
                <th>{locale === 'ru' ? 'Ресурс / Жидкость' : 'Resource / Fluid'}</th>
                <th>{locale === 'ru' ? 'Ед.' : 'Unit'}</th>
                <th>{locale === 'ru' ? 'Из источников' : 'Source'}</th>
                <th>{locale === 'ru' ? 'Расход' : 'Consumed'}</th>
                <th>{locale === 'ru' ? 'Выпуск (осн.)' : 'Main Out'}</th>
                <th>{locale === 'ru' ? 'Побочный' : 'Byproduct'}</th>
                <th>{locale === 'ru' ? 'Цель' : 'Target'}</th>
                <th>{locale === 'ru' ? 'Итоговый баланс' : 'Net Balance'}</th>
              </tr>
            </thead>
            <tbody>
              {#each solver.balances as row (row.kind + ':' + row.resourceId)}
                <tr>
                  <td class="res-cell">
                    <PixelSprite asset={assetsById.get(row.resourceId)} size={18} />
                    <span>{resName(row.resourceId)}</span>
                    {#if row.hasProbabilisticSupply}
                      <span class="prob-pill" title={tr.probabilisticLabel}>≈</span>
                    {/if}
                  </td>
                  <td>
                    <span class={row.kind === 'fluid' ? 'u-fluid' : 'u-item'}>
                      {row.kind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin}
                    </span>
                  </td>
                  <td>{row.totalSourceSupplyPerMin.toFixed(2)}</td>
                  <td>{row.totalConsumedPerMin.toFixed(2)}</td>
                  <td>{row.totalProducedMainPerMin.toFixed(2)}</td>
                  <td>{row.totalProducedByproductPerMin.toFixed(2)}</td>
                  <td>{row.totalTargetDemandPerMin.toFixed(2)}</td>
                  <td>
                    {#if row.netBalancePerMin < -1e-4}
                      <span class="bal-deficit">{tr.deficitLabel}: {row.netBalancePerMin.toFixed(2)}</span>
                    {:else if row.netBalancePerMin > 1e-4}
                      <span class="bal-surplus">+{row.netBalancePerMin.toFixed(2)}</span>
                    {:else}
                      <span class="bal-even">0.00</span>
                    {/if}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}
      {:else if activeTab === 'machines'}
        <table class="diag-table">
          <thead>
            <tr>
              <th>{locale === 'ru' ? 'Узел' : 'Node'}</th>
              <th>RPM</th>
              <th>{locale === 'ru' ? 'Опер./мин на 1 маш.' : 'Ops/min per mach'}</th>
              <th>{locale === 'ru' ? 'Требуется (опер./мин)' : 'Requested ops/min'}</th>
              <th>{tr.machinesExactLabel}</th>
              <th>{tr.machinesCeilLabel}</th>
              <th>{tr.stressLabel}</th>
              <th>{locale === 'ru' ? 'Статус' : 'Status'}</th>
            </tr>
          </thead>
          <tbody>
            {#each machineNodes as node (node.id)}
              {@const nc = solver.nodeMap[node.id]}
              <tr class="click-row" onclick={() => onFocusNode(node.id)}>
                <td>{node.title}</td>
                <td>{node.rpm ?? '—'}</td>
                <td>{nc?.craftsPerMachinePerMin !== null && nc?.craftsPerMachinePerMin !== undefined ? nc.craftsPerMachinePerMin.toFixed(2) : '?'}</td>
                <td>{nc ? nc.requestedCraftsPerMin.toFixed(2) : '0.00'}</td>
                <td>{nc?.requiredMachinesExact !== null && nc?.requiredMachinesExact !== undefined ? nc.requiredMachinesExact.toFixed(2) : '?'}</td>
                <td>{nc?.requiredMachinesCeil ?? '?'}</td>
                <td>{nc?.stressTotalSU ?? '—'}</td>
                <td>
                  {#if nc?.inCycle}
                    <span class="bal-deficit">{locale === 'ru' ? 'Цикл' : 'Cycle'}</span>
                  {:else if nc?.isBottleneck}
                    <span class="bal-deficit">{tr.bottleneckLabel}</span>
                  {:else if nc?.speedSource === 'unknown'}
                    <span class="bal-warn">{tr.unknownSpeedLabel}</span>
                  {:else if nc?.speedSource === 'assumption'}
                    <span class="u-fluid">{tr.assumptionLabel}</span>
                  {:else}
                    <span class="bal-surplus">OK</span>
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      {:else}
        {#if solver.warnings.length === 0}
          <div class="empty-msg ok-msg">✓ {tr.noWarnings}</div>
        {:else}
          <div class="warnings-list">
            {#each solver.warnings as w (w.id)}
              <button
                type="button"
                class={`warn-row sev-${w.severity}`}
                onclick={() => w.nodeId && onFocusNode(w.nodeId)}
              >
                <span class="warn-code">{w.code}</span>
                <span class="warn-text">{locale === 'ru' ? w.messageRu : w.messageEn}</span>
              </button>
            {/each}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
</section>

<style>
  .diagnostics-dock {
    background: var(--bg-panel);
    border-top: 1px solid var(--border-subtle);
    height: 215px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .diagnostics-dock.collapsed {
    height: 36px;
  }

  .dock-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 10px;
    height: 36px;
    min-height: 36px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
  }

  .dock-tabs {
    display: flex;
    gap: 6px;
    height: 100%;
  }

  .dock-tabs button {
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 600;
    padding: 0 10px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .dock-tabs button.active {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }

  .warn-count {
    background: rgba(249, 115, 22, 0.25);
    color: #fdba74;
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 999px;
  }

  .dock-stats {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 11px;
    color: var(--text-muted);
  }

  .dock-body {
    flex: 1;
    overflow: auto;
    padding: 6px 12px;
  }

  .diag-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .diag-table th {
    text-align: left;
    font-size: 11px;
    color: var(--text-muted);
    font-weight: 600;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border-subtle);
    position: sticky;
    top: 0;
    background: var(--bg-panel);
  }
  .diag-table td {
    padding: 5px 8px;
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-primary);
  }
  .click-row {
    cursor: pointer;
  }
  .click-row:hover td {
    background: var(--bg-elevated);
  }

  .res-cell {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .prob-pill {
    font-size: 10px;
    padding: 0 5px;
    border-radius: 4px;
    background: rgba(168, 85, 247, 0.2);
    color: #c084fc;
    font-weight: 700;
  }

  .u-fluid { color: #38bdf8; font-weight: 600; }
  .u-item { color: #f59e0b; font-weight: 600; }
  .bal-deficit { color: #fca5a5; font-weight: 700; }
  .bal-warn { color: #fdba74; font-weight: 700; }
  .bal-surplus { color: #34d399; font-weight: 700; }
  .bal-even { color: var(--text-muted); }

  .empty-msg {
    padding: 20px;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .ok-msg {
    color: #34d399;
    font-weight: 600;
  }

  .warnings-list {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .warn-row {
    display: flex;
    align-items: center;
    gap: 10px;
    text-align: left;
    padding: 7px 10px;
    border-radius: 7px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    color: var(--text-primary);
    cursor: pointer;
    font-size: 12px;
  }
  .warn-row.sev-error { border-left: 3px solid #ef4444; }
  .warn-row.sev-warning { border-left: 3px solid #f97316; }
  .warn-row.sev-info { border-left: 3px solid #38bdf8; }
  .warn-code {
    font-size: 10px;
    font-weight: 700;
    color: var(--text-muted);
    min-width: 118px;
  }
</style>
