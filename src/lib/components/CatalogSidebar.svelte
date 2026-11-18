<script lang="ts">
  import { t } from '../i18n/translations';
  import type {
    AssetManifestEntry,
    Locale,
    MachineDef,
    NodeKind,
    RecipeDef,
    ResourceDef,
  } from '../types/atlas';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    locale: Locale;
    recipes: RecipeDef[];
    machines: MachineDef[];
    resources: ResourceDef[];
    assetsById: Map<string, AssetManifestEntry>;
    onAddNodeKind: (kind: NodeKind) => void;
    onAddRecipeNode: (recipe: RecipeDef) => void;
    onAddResourceNode: (resource: ResourceDef, role: 'source' | 'target' | 'byproduct') => void;
    onAddMachineNode: (machine: MachineDef) => void;
    onOpenCreateResource: () => void;
    onOpenCreateMachine: () => void;
    onOpenManualRecipe: () => void;
  }

  let {
    locale,
    recipes,
    machines,
    resources,
    assetsById,
    onAddNodeKind,
    onAddRecipeNode,
    onAddResourceNode,
    onAddMachineNode,
    onOpenCreateResource,
    onOpenCreateMachine,
    onOpenManualRecipe,
  }: Props = $props();

  let search = $state('');
  let activeTab = $state<'resources' | 'machines' | 'recipes'>('resources');
  const tr = $derived(t(locale));

  const query = $derived(search.trim().toLowerCase());

  const filteredRecipes = $derived(
    recipes.filter((r) => {
      if (!query) return true;
      return (
        r.nameRu.toLowerCase().includes(query) ||
        r.nameEn.toLowerCase().includes(query) ||
        r.id.toLowerCase().includes(query) ||
        r.inputs.some((i) => i.resourceId.toLowerCase().includes(query)) ||
        r.outputs.some((o) => o.resourceId.toLowerCase().includes(query))
      );
    })
  );

  const filteredResources = $derived(
    resources.filter((r) => {
      if (!query) return true;
      return (
        r.nameRu.toLowerCase().includes(query) ||
        r.nameEn.toLowerCase().includes(query) ||
        r.id.toLowerCase().includes(query)
      );
    })
  );

  const filteredMachines = $derived(
    machines.filter((m) => {
      if (!query) return true;
      return (
        m.nameRu.toLowerCase().includes(query) ||
        m.nameEn.toLowerCase().includes(query) ||
        m.id.toLowerCase().includes(query)
      );
    })
  );

  const nodeKinds: NodeKind[] = ['source', 'recipe', 'machine', 'byproduct', 'target', 'note'];
</script>

<aside class="catalog-sidebar" aria-label={tr.catalogTitle}>
  <div class="sidebar-header">
    <h2>{tr.catalogTitle}</h2>
    <input
      type="search"
      class="search-input"
      bind:value={search}
      placeholder={tr.searchPlaceholder}
      aria-label={tr.searchPlaceholder}
    />
  </div>

  <section class="sidebar-block">
    <div class="section-label">{tr.nodeTypesSection}</div>
    <div class="kind-grid">
      {#each nodeKinds as kind}
        <button
          type="button"
          class={`kind-pill kind-${kind}`}
          onclick={() => onAddNodeKind(kind)}
          title={locale === 'ru' ? `Добавить узел: ${tr.nodeKinds[kind]}` : `Add node: ${tr.nodeKinds[kind]}`}
        >
          + {tr.nodeKinds[kind]}
        </button>
      {/each}
    </div>
  </section>

  <section class="sidebar-block">
    <div class="section-label">{locale === 'ru' ? 'Создать вручную' : 'Create Manually'}</div>
    <div class="preset-row">
      <button type="button" class="btn-primary btn-xs" onclick={onOpenCreateResource}>
        {locale === 'ru' ? '+ Ресурс' : '+ Resource'}
      </button>
      <button type="button" class="btn-secondary btn-xs" onclick={onOpenCreateMachine}>
        {locale === 'ru' ? '+ Машина' : '+ Machine'}
      </button>
      <button type="button" class="btn-secondary btn-xs" onclick={onOpenManualRecipe}>
        {locale === 'ru' ? '+ Рецепт' : '+ Recipe'}
      </button>
    </div>
  </section>

  <div class="catalog-tabs" role="tablist">
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'recipes'}
      class:active={activeTab === 'recipes'}
      onclick={() => (activeTab = 'recipes')}
    >
      {locale === 'ru' ? `Рецепты (${filteredRecipes.length})` : `Recipes (${filteredRecipes.length})`}
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'resources'}
      class:active={activeTab === 'resources'}
      onclick={() => (activeTab = 'resources')}
    >
      {locale === 'ru' ? `Ресурсы (${filteredResources.length})` : `Items (${filteredResources.length})`}
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === 'machines'}
      class:active={activeTab === 'machines'}
      onclick={() => (activeTab = 'machines')}
    >
      {locale === 'ru' ? `Машины (${filteredMachines.length})` : `Machines (${filteredMachines.length})`}
    </button>
  </div>

  <div class="catalog-scroll">
    {#if activeTab === 'recipes'}
      <div class="manual-recipe-bar">
        <button type="button" class="btn-secondary btn-xs full-w" onclick={onOpenManualRecipe}>
          {tr.addRecipeManual}
        </button>
      </div>
      {#if filteredRecipes.length === 0}
        <div class="empty-tab-msg">
          {locale === 'ru'
            ? 'В проекте пока нет рецептов. Создайте рецепт вручную или импортируйте .jar/.json при желании.'
            : 'No recipes in this project yet. Create one manually or import .jar/.json if needed.'}
        </div>
      {/if}
      {#each filteredRecipes.slice(0, 120) as recipe (recipe.id)}
        {@const mainOut = recipe.outputs[0]?.resourceId}
        {@const spriteAsset = (mainOut && assetsById.get(mainOut)) || assetsById.get(recipe.machineId)}
        <button
          type="button"
          class="catalog-item"
          onclick={() => onAddRecipeNode(recipe)}
          title={`${recipe.id} (${recipe.source.modId} ${recipe.source.modVersion})`}
        >
          <PixelSprite asset={spriteAsset} size={26} />
          <div class="ci-body">
            <div class="ci-title">{locale === 'ru' ? recipe.nameRu : recipe.nameEn}</div>
            <div class="ci-meta">
              <span>{recipe.machineId.split(':')[1] ?? recipe.machineId}</span>
              <span>·</span>
              <span>{recipe.source.modId} {recipe.source.modVersion}</span>
              {#if recipe.outputs.some((o) => o.probabilityKind !== 'guaranteed')}
                <span class="badge-prob">%</span>
              {/if}
            </div>
          </div>
          <span class="ci-add">+</span>
        </button>
      {/each}
      {#if filteredRecipes.length > 120}
        <div class="trunc-hint">
          {locale === 'ru'
            ? `Показаны первые 120 из ${filteredRecipes.length}. Уточните поиск.`
            : `Showing first 120 of ${filteredRecipes.length}. Narrow your search.`}
        </div>
      {/if}
    {:else if activeTab === 'resources'}
      <div class="manual-recipe-bar">
        <button type="button" class="btn-secondary btn-xs full-w" onclick={onOpenCreateResource}>
          {locale === 'ru' ? '+ Создать ресурс (предмет / жидкость)' : '+ Create Resource (Item / Fluid)'}
        </button>
      </div>
      {#if filteredResources.length === 0}
        <div class="empty-tab-msg">
          {locale === 'ru'
            ? 'Список ресурсов пуст. Создайте первый предмет или жидкость вручную — 3D-модель и текстура подберутся по названию или ID.'
            : 'No resources yet. Create your first item or fluid manually — 3D model and texture can be matched by name or ID.'}
        </div>
      {/if}
      {#each filteredResources as res (res.id)}
        <div class="resource-row">
          <PixelSprite
            asset={assetsById.get(res.id)}
            fallbackUrl={res.kind === 'fluid' ? '/assets-fallback/fluid-fallback.svg' : '/assets-fallback/item-fallback.svg'}
            size={24}
          />
          <div class="ci-body">
            <div class="ci-title">{locale === 'ru' ? res.nameRu : res.nameEn}</div>
            <div class="ci-meta">
              <span class={res.kind === 'fluid' ? 'unit-fluid' : 'unit-item'}>
                {res.kind === 'fluid' ? tr.unitFluidMin : tr.unitItemsMin}
              </span>
              <span>· {res.id}</span>
            </div>
          </div>
          <div class="res-quick-btns">
            <button
              type="button"
              class="mini-btn"
              title={locale === 'ru' ? 'Добавить как источник' : 'Add as Source'}
              onclick={() => onAddResourceNode(res, 'source')}
            >
              {locale === 'ru' ? 'Ист.' : 'Src'}
            </button>
            <button
              type="button"
              class="mini-btn"
              title={locale === 'ru' ? 'Добавить как цель' : 'Add as Target'}
              onclick={() => onAddResourceNode(res, 'target')}
            >
              {locale === 'ru' ? 'Цель' : 'Tgt'}
            </button>
          </div>
        </div>
      {/each}
    {:else}
      <div class="manual-recipe-bar">
        <button type="button" class="btn-secondary btn-xs full-w" onclick={onOpenCreateMachine}>
          {locale === 'ru' ? '+ Создать машину / станок' : '+ Create Machine'}
        </button>
      </div>
      {#if filteredMachines.length === 0}
        <div class="empty-tab-msg">
          {locale === 'ru'
            ? 'Список машин пуст. Добавьте станок вручную и выберите расчёт скорости (Create / ручной / неизвестно).'
            : 'No machines yet. Add a machine manually and set its speed model.'}
        </div>
      {/if}
      {#each filteredMachines as mach (mach.id)}
        <button
          type="button"
          class="catalog-item"
          onclick={() => onAddMachineNode(mach)}
        >
          <PixelSprite
            asset={assetsById.get(mach.id)}
            fallbackUrl="/assets-fallback/machine-fallback.svg"
            size={26}
          />
          <div class="ci-body">
            <div class="ci-title">{locale === 'ru' ? mach.nameRu : mach.nameEn}</div>
            <div class="ci-meta">
              <span>{mach.stressImpactSU !== null ? `${mach.stressImpactSU} SU/RPM` : 'SU: —'}</span>
              <span>· {mach.modId} {mach.modVersion}</span>
            </div>
          </div>
          <span class="ci-add">+</span>
        </button>
      {/each}
    {/if}
  </div>
</aside>

<style>
  .catalog-sidebar {
    width: 296px;
    min-width: 260px;
    background: var(--bg-panel);
    border-right: 1px solid var(--border-subtle);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 10px 12px 8px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .sidebar-header h2 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 700;
    color: var(--text-primary);
  }

  .search-input {
    width: 100%;
    padding: 7px 10px;
    border-radius: 7px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12.5px;
    box-sizing: border-box;
  }

  .sidebar-block {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border-subtle);
  }

  .section-label {
    font-size: 10.5px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-muted);
    margin-bottom: 6px;
    font-weight: 600;
  }

  .kind-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px;
  }

  .kind-pill {
    font-size: 11px;
    padding: 5px 7px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
    color: var(--text-primary);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .kind-pill:hover {
    border-color: var(--accent);
  }

  .preset-row {
    display: flex;
    gap: 6px;
  }

  .catalog-tabs {
    display: flex;
    border-bottom: 1px solid var(--border-subtle);
    background: var(--bg-elevated);
  }

  .catalog-tabs button {
    flex: 1;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 11.5px;
    font-weight: 600;
    padding: 8px 4px;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }

  .catalog-tabs button.active {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
    background: var(--bg-panel);
  }

  .catalog-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .manual-recipe-bar {
    margin-bottom: 4px;
  }
  .full-w {
    width: 100%;
  }

  .catalog-item,
  .resource-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border-radius: 7px;
    border: 1px solid transparent;
    background: var(--bg-elevated);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
    width: 100%;
  }

  .catalog-item:hover {
    border-color: var(--accent);
  }

  .ci-body {
    flex: 1;
    min-width: 0;
  }

  .ci-title {
    font-size: 12px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .ci-meta {
    font-size: 10.5px;
    color: var(--text-muted);
    display: flex;
    gap: 4px;
    align-items: center;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .badge-prob {
    background: rgba(168, 85, 247, 0.2);
    color: #c084fc;
    padding: 0 4px;
    border-radius: 4px;
    font-weight: 700;
  }

  .ci-add {
    font-weight: 700;
    color: var(--accent-text);
    font-size: 14px;
    padding: 0 4px;
  }

  .res-quick-btns {
    display: flex;
    gap: 4px;
  }

  .mini-btn {
    font-size: 10.5px;
    padding: 3px 6px;
    border-radius: 5px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-panel);
    color: var(--text-primary);
    cursor: pointer;
  }
  .mini-btn:hover {
    border-color: var(--accent);
  }

  .unit-fluid {
    color: #38bdf8;
    font-weight: 600;
  }
  .unit-item {
    color: #f59e0b;
    font-weight: 600;
  }

  .trunc-hint,
  .empty-tab-msg {
    font-size: 11.5px;
    color: var(--text-muted);
    text-align: center;
    padding: 14px 10px;
    line-height: 1.4;
  }
</style>
