<script lang="ts">
  import { computeMachineThroughput } from '../engine/machineSpeeds';
  import { t } from '../i18n/translations';
  import type {
    AssetManifestEntry,
    FactoryEdge,
    FactoryNode,
    Locale,
    MachineDef,
    MachineSpeedMode,
    RecipeDef,
    ResourceDef,
    SolverResult,
  } from '../types/atlas';
  import ModelViewer3D from './ModelViewer3D.svelte';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    locale: Locale;
    selectedNode: FactoryNode | null;
    selectedEdge: FactoryEdge | null;
    solver: SolverResult;
    recipes: RecipeDef[];
    machines: MachineDef[];
    resources: ResourceDef[];
    recipesById: Map<string, RecipeDef>;
    machinesById: Map<string, MachineDef>;
    resourcesById: Map<string, ResourceDef>;
    assetsById: Map<string, AssetManifestEntry>;
    onUpdateNode: (nodeId: string, patch: Partial<FactoryNode>) => void;
    onUpdateEdge: (edgeId: string, patch: Partial<FactoryEdge>) => void;
    onDeleteEdge: (edgeId: string) => void;
    onDeleteNode: (nodeId: string) => void;
    onEditVisual: (type: 'resource' | 'machine', id: string) => void;
  }

  let {
    locale,
    selectedNode,
    selectedEdge,
    solver,
    recipes,
    machines,
    resources,
    recipesById,
    machinesById,
    resourcesById,
    assetsById,
    onUpdateNode,
    onUpdateEdge,
    onDeleteEdge,
    onDeleteNode,
    onEditVisual,
  }: Props = $props();

  const tr = $derived(t(locale));

  const nodeCalc = $derived(selectedNode ? solver.nodeMap[selectedNode.id] : undefined);
  const edgeCalc = $derived(selectedEdge ? solver.edgeMap[selectedEdge.id] : undefined);
  const selectedRecipe = $derived(
    selectedNode?.recipeId ? recipesById.get(selectedNode.recipeId) : undefined
  );
  const selectedMachine = $derived.by(() => {
    const mid = selectedNode?.machineId ?? selectedRecipe?.machineId;
    return mid ? machinesById.get(mid) : undefined;
  });
  const selectedResource = $derived(
    selectedNode?.resourceId ? resourcesById.get(selectedNode.resourceId) : undefined
  );
  const activeVisualTarget = $derived.by(() => {
    if (!selectedNode) return null;
    if (selectedResource) {
      return {
        type: 'resource' as const,
        id: selectedResource.id,
        visual: selectedResource.visual,
        fallback:
          selectedResource.kind === 'fluid'
            ? '/assets-fallback/fluid-fallback.svg'
            : '/assets-fallback/item-fallback.svg',
      };
    }
    if (selectedMachine) {
      return {
        type: 'machine' as const,
        id: selectedMachine.id,
        visual: selectedMachine.visual,
        fallback: '/assets-fallback/machine-fallback.svg',
      };
    }
    return null;
  });
  const speedPreview = $derived.by(() => {
    if (!selectedNode || (selectedNode.kind !== 'recipe' && selectedNode.kind !== 'machine')) {
      return null;
    }
    return computeMachineThroughput({
      machine: selectedMachine,
      recipe: selectedRecipe,
      rpm: selectedNode.rpm,
      speedMode: selectedNode.speedMode,
      manualCraftsPerMin: selectedNode.manualCraftsPerMin,
      stackSizeAssumption: selectedNode.stackSizeAssumption,
    });
  });

  function resName(id: string): string {
    const r = resourcesById.get(id);
    return r ? (locale === 'ru' ? r.nameRu : r.nameEn) : id;
  }
</script>

<aside class="inspector-panel" aria-label={tr.inspectorTitle}>
  <div class="inspector-head">
    <h2>{tr.inspectorTitle}</h2>
  </div>

  <div class="inspector-body">
    {#if selectedNode}
      <div class="field-group">
        <label class="field-label" for="insp-title">
          {locale === 'ru' ? 'Название узла' : 'Node title'}
        </label>
        <input
          id="insp-title"
          type="text"
          class="text-input"
          value={selectedNode.title}
          oninput={(e) => onUpdateNode(selectedNode.id, { title: (e.target as HTMLInputElement).value })}
        />
      </div>

      {#if selectedNode.kind === 'source' || selectedNode.kind === 'target' || selectedNode.kind === 'byproduct'}
        <div class="field-group">
          <label class="field-label" for="insp-res">
            {locale === 'ru' ? 'Ресурс / Жидкость' : 'Resource / Fluid'}
          </label>
          <select
            id="insp-res"
            class="text-input"
            value={selectedNode.resourceId ?? ''}
            onchange={(e) => {
              const id = (e.target as HTMLSelectElement).value;
              const r = resourcesById.get(id);
              onUpdateNode(selectedNode.id, {
                resourceId: id,
                resourceKind: r?.kind ?? 'item',
              });
            }}
          >
            <option value="">— {locale === 'ru' ? 'Выберите' : 'Select'} —</option>
            {#each resources as r (r.id)}
              <option value={r.id}>
                {locale === 'ru' ? r.nameRu : r.nameEn} ({r.kind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})
              </option>
            {/each}
          </select>
        </div>

        <div class="unit-badge-row">
          <span class={selectedNode.resourceKind === 'fluid' ? 'badge-fluid' : 'badge-item'}>
            {selectedNode.resourceKind === 'fluid'
              ? `${locale === 'ru' ? 'Единица' : 'Unit'}: ${tr.unitFluidMin}`
              : `${locale === 'ru' ? 'Единица' : 'Unit'}: ${tr.unitItemsMin}`}
          </span>
        </div>
      {/if}

      {#if selectedNode.kind === 'source'}
        <div class="field-group">
          <label class="checkbox-row">
            <input
              type="checkbox"
              checked={Boolean(selectedNode.sourceUnlimited)}
              onchange={(e) =>
                onUpdateNode(selectedNode.id, {
                  sourceUnlimited: (e.target as HTMLInputElement).checked,
                })}
            />
            <span>{locale === 'ru' ? 'Безлимитный источник (расчёт по спросу)' : 'Unlimited source (demand-driven)'}</span>
          </label>
        </div>
        {#if !selectedNode.sourceUnlimited}
          <div class="field-group">
            <label class="field-label" for="insp-src-rate">
              {locale === 'ru'
                ? `Доступная подача (${selectedNode.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})`
                : `Supply rate (${selectedNode.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})`}
            </label>
            <input
              id="insp-src-rate"
              type="number"
              min="0"
              step="0.5"
              class="text-input"
              value={selectedNode.sourceRatePerMin ?? 0}
              oninput={(e) =>
                onUpdateNode(selectedNode.id, {
                  sourceRatePerMin: Math.max(0, Number((e.target as HTMLInputElement).value) || 0),
                })}
            />
          </div>
        {/if}
      {/if}

      {#if selectedNode.kind === 'target'}
        <div class="field-group">
          <label class="field-label" for="insp-tgt-rate">
            {locale === 'ru'
              ? `Целевой темп (${selectedNode.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})`
              : `Target rate (${selectedNode.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})`}
          </label>
          <input
            id="insp-tgt-rate"
            type="number"
            min="0"
            step="1"
            class="text-input"
            value={selectedNode.targetRatePerMin ?? 0}
            oninput={(e) =>
              onUpdateNode(selectedNode.id, {
                targetRatePerMin: Math.max(0, Number((e.target as HTMLInputElement).value) || 0),
              })}
          />
        </div>
      {/if}

      {#if selectedNode.kind === 'recipe' || selectedNode.kind === 'machine'}
        <div class="field-group">
          <label class="field-label" for="insp-recipe">
            {locale === 'ru' ? 'Рецепт' : 'Recipe'}
          </label>
          <select
            id="insp-recipe"
            class="text-input"
            value={selectedNode.recipeId ?? ''}
            onchange={(e) => {
              const rid = (e.target as HTMLSelectElement).value;
              const rec = recipesById.get(rid);
              onUpdateNode(selectedNode.id, {
                recipeId: rid || undefined,
                machineId: rec?.machineId ?? selectedNode.machineId,
              });
            }}
          >
            <option value="">— {locale === 'ru' ? 'Без рецепта' : 'No recipe'} —</option>
            {#each recipes as r (r.id)}
              <option value={r.id}>{locale === 'ru' ? r.nameRu : r.nameEn}</option>
            {/each}
          </select>
        </div>

        <div class="field-group">
          <label class="field-label" for="insp-machine">
            {locale === 'ru' ? 'Машина' : 'Machine'}
          </label>
          <select
            id="insp-machine"
            class="text-input"
            value={selectedNode.machineId ?? selectedRecipe?.machineId ?? ''}
            onchange={(e) =>
              onUpdateNode(selectedNode.id, { machineId: (e.target as HTMLSelectElement).value })}
          >
            <option value="">— {locale === 'ru' ? 'Не выбрана' : 'None'} —</option>
            {#each machines as m (m.id)}
              <option value={m.id}>{locale === 'ru' ? m.nameRu : m.nameEn}</option>
            {/each}
          </select>
        </div>

        <div class="field-group">
          <label class="field-label" for="insp-spd-mode">
            {locale === 'ru' ? 'Режим расчёта скорости' : 'Speed calculation mode'}
          </label>
          <select
            id="insp-spd-mode"
            class="text-input"
            value={selectedNode.speedMode ?? 'formula'}
            onchange={(e) =>
              onUpdateNode(selectedNode.id, {
                speedMode: (e.target as HTMLSelectElement).value as MachineSpeedMode,
              })}
          >
            <option value="formula">{locale === 'ru' ? 'Формула Create (RPM)' : 'Create Formula (RPM)'}</option>
            <option value="assumption">{locale === 'ru' ? 'Допущение (пачка/лента)' : 'Assumption (batch/belt)'}</option>
            <option value="manual">{locale === 'ru' ? 'Ручной темп (опер./мин)' : 'Manual rate (ops/min)'}</option>
            <option value="unknown">{locale === 'ru' ? 'Неизвестно (предупреждение)' : 'Unknown (warn)'}</option>
          </select>
        </div>

        {#if selectedNode.speedMode === 'manual'}
          <div class="field-group">
            <label class="field-label" for="insp-manual-rate">
              {locale === 'ru' ? 'Операций/мин на 1 машину' : 'Ops/min per machine'}
            </label>
            <input
              id="insp-manual-rate"
              type="number"
              min="0.1"
              step="0.5"
              class="text-input"
              value={selectedNode.manualCraftsPerMin ?? 30}
              oninput={(e) =>
                onUpdateNode(selectedNode.id, {
                  manualCraftsPerMin: Math.max(0.01, Number((e.target as HTMLInputElement).value) || 0),
                })}
            />
          </div>
        {:else if selectedMachine?.supportsRpm}
          <div class="field-group">
            <label class="field-label" for="insp-rpm">
              {locale === 'ru' ? `Скорость (RPM, макс. ${selectedMachine.maxRpm})` : `Speed (RPM, max ${selectedMachine.maxRpm})`}
            </label>
            <input
              id="insp-rpm"
              type="number"
              min="1"
              max={selectedMachine.maxRpm}
              step="8"
              class="text-input"
              value={selectedNode.rpm ?? selectedMachine.defaultRpm ?? 64}
              oninput={(e) =>
                onUpdateNode(selectedNode.id, {
                  rpm: Math.min(selectedMachine.maxRpm, Math.max(0, Number((e.target as HTMLInputElement).value) || 0)),
                })}
            />
          </div>
        {/if}

        {#if selectedMachine && (selectedMachine.speedFormulaId === 'crushing_wheel' || selectedMachine.speedFormulaId === 'encased_fan')}
          <div class="field-group">
            <label class="field-label" for="insp-stack">
              {locale === 'ru' ? 'Допущение размера пачки (1–64)' : 'Batch size assumption (1–64)'}
            </label>
            <input
              id="insp-stack"
              type="number"
              min="1"
              max="64"
              class="text-input"
              value={selectedNode.stackSizeAssumption ?? 16}
              oninput={(e) =>
                onUpdateNode(selectedNode.id, {
                  stackSizeAssumption: Math.min(64, Math.max(1, Number((e.target as HTMLInputElement).value) || 1)),
                })}
            />
          </div>
        {/if}

        <div class="field-group">
          <label class="field-label" for="insp-cap">
            {locale === 'ru' ? 'Ограничение числа машин (пусто = авто)' : 'Machine count limit (empty = auto)'}
          </label>
          <input
            id="insp-cap"
            type="number"
            min="1"
            step="1"
            class="text-input"
            placeholder={locale === 'ru' ? 'Авто' : 'Auto'}
            value={selectedNode.machineCountOverride ?? ''}
            oninput={(e) => {
              const v = (e.target as HTMLInputElement).value.trim();
              onUpdateNode(selectedNode.id, {
                machineCountOverride: v ? Math.max(1, Math.floor(Number(v))) : undefined,
              });
            }}
          />
        </div>

        {#if speedPreview}
          <div class="info-card">
            <div class="info-card-title">{locale === 'ru' ? 'Расчёт темпа' : 'Speed calculation'}</div>
            <p>{locale === 'ru' ? speedPreview.explanationRu : speedPreview.explanationEn}</p>
            {#if selectedRecipe}
              <div class="provenance">
                {locale === 'ru' ? 'Источник:' : 'Source:'} {selectedRecipe.source.modId} {selectedRecipe.source.modVersion}
                ({selectedRecipe.source.minecraftVersion} · {selectedRecipe.source.loader})
              </div>
            {/if}
          </div>
        {/if}

        {#if nodeCalc}
          <div class="info-card">
            <div class="info-card-title">{locale === 'ru' ? 'Потоки узла' : 'Node flows'}</div>
            {#each nodeCalc.inputs as inp}
              <div class="flow-line">
                <PixelSprite asset={assetsById.get(inp.resourceId)} size={16} />
                <span class="fl-name">IN: {resName(inp.resourceId)}</span>
                <span class="fl-val">{inp.suppliedPerMin.toFixed(1)} / {inp.requiredPerMin.toFixed(1)}</span>
              </div>
            {/each}
            {#each nodeCalc.outputs as out}
              <div class="flow-line">
                <PixelSprite asset={assetsById.get(out.resourceId)} size={16} />
                <span class="fl-name">
                  OUT{out.isByproduct ? ' (+)' : ''}: {resName(out.resourceId)}
                </span>
                <span class="fl-val">
                  {out.probabilistic ? '≈ ' : ''}{out.producedExpectedPerMin.toFixed(2)}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      {/if}

      {#if selectedNode.kind === 'note'}
        <div class="field-group">
          <label class="field-label" for="insp-note">
            {locale === 'ru' ? 'Текст заметки' : 'Note text'}
          </label>
          <textarea
            id="insp-note"
            rows="6"
            class="text-input"
            value={selectedNode.noteText ?? ''}
            oninput={(e) => onUpdateNode(selectedNode.id, { noteText: (e.target as HTMLTextAreaElement).value })}
          ></textarea>
        </div>
      {/if}

      {#if activeVisualTarget}
        <div class="info-card" style="align-items: center; text-align: center;">
          <div class="info-card-title">{locale === 'ru' ? '3D-модель / текстура' : '3D Model / Texture'}</div>
          <ModelViewer3D
            {locale}
            visual={activeVisualTarget.visual}
            fallbackUrl={activeVisualTarget.fallback}
            size={156}
          />
          <button
            type="button"
            class="btn-secondary btn-xs full-w"
            style="margin-top: 6px;"
            onclick={() => onEditVisual(activeVisualTarget.type, activeVisualTarget.id)}
          >
            {locale === 'ru' ? 'Настроить модель / текстуру' : 'Configure model / texture'}
          </button>
        </div>
      {/if}

      <div class="danger-zone">
        <button type="button" class="btn-danger full-w" onclick={() => onDeleteNode(selectedNode.id)}>
          {locale === 'ru' ? 'Удалить узел' : 'Delete node'}
        </button>
      </div>
    {:else if selectedEdge}
      <div class="field-group">
        <label class="field-label" for="edge-res">
          {locale === 'ru' ? 'Передаваемый ресурс' : 'Carried resource'}
        </label>
        <select
          id="edge-res"
          class="text-input"
          value={selectedEdge.resourceId}
          onchange={(e) => {
            const id = (e.target as HTMLSelectElement).value;
            const r = resourcesById.get(id);
            onUpdateEdge(selectedEdge.id, { resourceId: id, resourceKind: r?.kind ?? 'item' });
          }}
        >
          {#each resources as r (r.id)}
            <option value={r.id}>
              {locale === 'ru' ? r.nameRu : r.nameEn} ({r.kind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})
            </option>
          {/each}
        </select>
      </div>

      <div class="field-group">
        <label class="field-label" for="edge-mode">
          {locale === 'ru' ? 'Режим потока' : 'Flow mode'}
        </label>
        <select
          id="edge-mode"
          class="text-input"
          value={selectedEdge.rateMode}
          onchange={(e) =>
            onUpdateEdge(selectedEdge.id, {
              rateMode: (e.target as HTMLSelectElement).value as 'auto' | 'manual',
            })}
        >
          <option value="auto">{locale === 'ru' ? 'Авторасчёт по спросу/подаче' : 'Auto demand/supply'}</option>
          <option value="manual">{locale === 'ru' ? 'Ручной лимит потока' : 'Manual rate cap'}</option>
        </select>
      </div>

      {#if selectedEdge.rateMode === 'manual'}
        <div class="field-group">
          <label class="field-label" for="edge-cap">
            {locale === 'ru' ? 'Пропускная способность' : 'Manual rate cap'} ({selectedEdge.resourceKind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin})
          </label>
          <input
            id="edge-cap"
            type="number"
            min="0"
            step="1"
            class="text-input"
            value={selectedEdge.manualRatePerMin ?? 60}
            oninput={(e) =>
              onUpdateEdge(selectedEdge.id, {
                manualRatePerMin: Math.max(0, Number((e.target as HTMLInputElement).value) || 0),
              })}
          />
        </div>
      {/if}

      {#if edgeCalc}
        <div class="info-card">
          <div class="info-card-title">{locale === 'ru' ? 'Параметры линии' : 'Line diagnostics'}</div>
          <div>{locale === 'ru' ? 'Поток' : 'Flow'}: <strong>{edgeCalc.flowPerMin.toFixed(2)} {edgeCalc.unit}</strong></div>
          <div>{locale === 'ru' ? 'Спрос' : 'Demand'}: {edgeCalc.demandPerMin.toFixed(2)} {edgeCalc.unit}</div>
          <div>{locale === 'ru' ? 'Подача' : 'Supply'}: {edgeCalc.supplyPerMin.toFixed(2)} {edgeCalc.unit}</div>
          {#if edgeCalc.deficitPerMin > 0}
            <div class="err-txt">{tr.deficitLabel}: {edgeCalc.deficitPerMin.toFixed(2)} {edgeCalc.unit}</div>
          {/if}
        </div>
      {/if}

      <div class="danger-zone">
        <button type="button" class="btn-danger full-w" onclick={() => onDeleteEdge(selectedEdge.id)}>
          {locale === 'ru' ? 'Разорвать соединение' : 'Delete wire'}
        </button>
      </div>
    {:else}
      <div class="empty-inspector">
        <p>{tr.emptyInspector}</p>
      </div>
    {/if}
  </div>
</aside>

<style>
  .inspector-panel {
    width: 300px;
    min-width: 260px;
    background: var(--bg-panel);
    border-left: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .inspector-head {
    padding: 10px 14px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .inspector-head h2 {
    margin: 0;
    font-size: 13px;
    font-weight: 700;
  }

  .inspector-body {
    flex: 1;
    overflow-y: auto;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .text-input {
    width: 100%;
    padding: 7px 9px;
    border-radius: 7px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12.5px;
    box-sizing: border-box;
    font-family: inherit;
  }

  .checkbox-row {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    cursor: pointer;
  }

  .unit-badge-row {
    display: flex;
    gap: 6px;
  }
  .badge-fluid {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(56, 189, 248, 0.16);
    color: #38bdf8;
    font-weight: 600;
  }
  .badge-item {
    font-size: 11px;
    padding: 2px 8px;
    border-radius: 999px;
    background: rgba(245, 158, 11, 0.16);
    color: #f59e0b;
    font-weight: 600;
  }

  .info-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 9px 10px;
    font-size: 11.5px;
    color: var(--text-secondary);
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .info-card p {
    margin: 0;
    line-height: 1.35;
  }
  .info-card-title {
    font-weight: 700;
    color: var(--text-primary);
    font-size: 11.5px;
  }
  .provenance {
    font-size: 10.5px;
    color: var(--text-muted);
  }

  .flow-line {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .fl-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fl-val {
    font-weight: 700;
    color: var(--text-primary);
  }

  .err-txt {
    color: #fca5a5;
    font-weight: 600;
  }

  .danger-zone {
    margin-top: auto;
    padding-top: 8px;
  }
  .full-w {
    width: 100%;
  }

  .empty-inspector {
    color: var(--text-muted);
    font-size: 12.5px;
    line-height: 1.45;
  }
</style>
