<script lang="ts">
  import { untrack } from 'svelte';
  import { matchVisualResources, type VisualMatchResult } from '../assets/visualMatcher';
  import { VERIFIED_SPEED_FORMULAS } from '../data/createCatalog';
  import type {
    Locale,
    MachineDef,
    ResourceDef,
    ResourceKind,
    ResourceVisualAssignment,
    VisualEntrySummary,
  } from '../types/atlas';
  import ModelViewer3D from './ModelViewer3D.svelte';

  interface Props {
    locale: Locale;
    initialMode?: 'resource' | 'machine';
    visualEntries: VisualEntrySummary[];
    editingVisualTarget?: {
      type: 'resource' | 'machine';
      id: string;
      name: string;
      visual?: ResourceVisualAssignment;
    } | null;
    onSaveResource: (res: ResourceDef, addToCanvasRole?: 'source' | 'target') => void;
    onSaveMachine: (mach: MachineDef, addToCanvas?: boolean) => void;
    onUpdateVisualOnly?: (type: 'resource' | 'machine', id: string, visual: ResourceVisualAssignment) => void;
    onClose: () => void;
  }

  let {
    locale,
    initialMode = 'resource',
    visualEntries,
    editingVisualTarget = null,
    onSaveResource,
    onSaveMachine,
    onUpdateVisualOnly,
    onClose,
  }: Props = $props();

  let mode = $state<'resource' | 'machine'>(untrack(() => editingVisualTarget?.type ?? initialMode));

  // Form fields
  let displayName = $state(untrack(() => editingVisualTarget?.name ?? ''));
  let customId = $state(untrack(() => editingVisualTarget?.id ?? ''));
  let resKind = $state<ResourceKind>('item');

  // Machine fields
  let formulaId = $state<MachineDef['speedFormulaId']>('unknown');
  let stressSU = $state<string>('');
  let defaultRpm = $state<number>(64);

  // Visual match state
  let searchQuery = $state(untrack(() => editingVisualTarget?.name ?? ''));
  let selectedVisual = $state<ResourceVisualAssignment | undefined>(
    untrack(() => editingVisualTarget?.visual)
  );
  let lastMatch = $state<VisualMatchResult | null>(null);
  let addCanvasChoice = $state<'none' | 'source' | 'target' | 'machine'>('none');

  function makeSlugId(name: string): string {
    const clean = name
      .toLowerCase()
      .trim()
      .replace(/[^a-z0-9_]+/g, '_')
      .replace(/^_+|_+$/g, '');
    return clean ? `custom:${clean}` : `custom:item_${Date.now()}`;
  }

  function applyVisualCandidate(entry: VisualEntrySummary, reason: VisualMatchResult['reason']) {
    selectedVisual = {
      visualId: entry.id,
      modelRef: entry.modelRef,
      modelPath: entry.modelPath,
      texturePath: entry.texturePath,
      modelSupport: entry.modelSupport,
      matchReason: reason,
      disabled: false,
    };
    if (!editingVisualTarget) {
      if (!displayName.trim()) {
        displayName = locale === 'ru' ? (entry.nameRu || entry.nameEn) : entry.nameEn;
      }
      if (!customId.trim() || customId.startsWith('custom:')) {
        customId = entry.id;
      }
      if (entry.kind === 'fluid') {
        resKind = 'fluid';
      }
    }
  }

  function runVisualLookup() {
    const q = (searchQuery || displayName || customId).trim();
    if (!q) return;
    const res = matchVisualResources(q, visualEntries, 16);
    lastMatch = res;
    if (res.status === 'unique' && res.best) {
      applyVisualCandidate(res.best, res.reason);
    } else if (res.status === 'not-found') {
      selectedVisual = {
        visualId: null,
        modelRef: null,
        modelPath: null,
        texturePath: null,
        modelSupport: 'none',
        matchReason: 'none',
        disabled: false,
      };
    }
  }

  function disableVisual() {
    selectedVisual = {
      visualId: null,
      modelRef: null,
      modelPath: null,
      texturePath: null,
      modelSupport: 'none',
      matchReason: 'manual',
      disabled: true,
    };
  }

  async function handleCustomPngUpload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result !== 'string') return;
      selectedVisual = {
        ...(selectedVisual ?? {
          visualId: null,
          modelRef: null,
          modelPath: null,
          texturePath: null,
          modelSupport: 'none',
        }),
        customTextureOverrideDataUrl: reader.result,
        disabled: false,
        matchReason: 'manual',
      };
    };
    reader.readAsDataURL(file);
  }

  async function handleCustomModelJsonUpload(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const txt = await file.text();
    selectedVisual = {
      ...(selectedVisual ?? {
        visualId: null,
        modelRef: 'custom:inline',
        modelPath: file.name,
        texturePath: null,
        modelSupport: '3d-json-elements',
      }),
      modelRef: selectedVisual?.modelRef || `custom:${file.name.replace(/\.json$/i, '')}`,
      customModelJson: txt,
      modelSupport: '3d-json-elements',
      disabled: false,
      matchReason: 'manual',
    };
  }

  function handleSubmit(e: Event) {
    e.preventDefault();
    if (editingVisualTarget && onUpdateVisualOnly) {
      onUpdateVisualOnly(
        editingVisualTarget.type,
        editingVisualTarget.id,
        selectedVisual ?? {
          visualId: null,
          modelRef: null,
          modelPath: null,
          texturePath: null,
          modelSupport: 'none',
          disabled: true,
        }
      );
      onClose();
      return;
    }

    const nameFinal = displayName.trim() || customId.trim() || 'Новый элемент';
    const idFinal = customId.trim() || makeSlugId(nameFinal);

    if (mode === 'resource') {
      const resource: ResourceDef = {
        id: idFinal,
        kind: resKind,
        nameRu: nameFinal,
        nameEn: nameFinal,
        modId: idFinal.includes(':') ? idFinal.split(':', 2)[0] : 'custom',
        category: resKind === 'fluid' ? 'fluid' : 'intermediate',
        color: resKind === 'fluid' ? '#38bdf8' : '#f59e0b',
        visual: selectedVisual,
        userCreated: true,
      };
      onSaveResource(
        resource,
        addCanvasChoice === 'source' || addCanvasChoice === 'target' ? addCanvasChoice : undefined
      );
    } else {
      const formulaMeta = VERIFIED_SPEED_FORMULAS.find((f) => f.id === formulaId) ?? VERIFIED_SPEED_FORMULAS[0];
      const suVal = stressSU.trim() === '' ? formulaMeta.defaultStressSU : Number(stressSU);
      const mach: MachineDef = {
        id: idFinal,
        nameRu: nameFinal,
        nameEn: nameFinal,
        modId: idFinal.includes(':') ? idFinal.split(':', 2)[0] : 'custom',
        modVersion: 'user',
        stressImpactSU: Number.isFinite(suVal) ? suVal : null,
        defaultRpm: formulaMeta.supportsRpm ? defaultRpm : null,
        maxRpm: 256,
        supportsRpm: formulaMeta.supportsRpm,
        speedFormulaId: formulaId,
        speedFormulaNoteRu: formulaMeta.noteRu,
        speedFormulaNoteEn: formulaMeta.noteEn,
        sourceRef: 'Создано пользователем вручную',
        visual: selectedVisual,
        userCreated: true,
      };
      onSaveMachine(mach, addCanvasChoice === 'machine');
    }
    onClose();
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Creator">
  <div class="modal-card modal-lg">
    <div class="modal-header">
      <div>
        <h3>
          {#if editingVisualTarget}
            {locale === 'ru' ? `Модель и текстура: ${editingVisualTarget.name}` : `Model & texture: ${editingVisualTarget.name}`}
          {:else if mode === 'resource'}
            {locale === 'ru' ? 'Создать ресурс (предмет / жидкость)' : 'Create Resource (Item / Fluid)'}
          {:else}
            {locale === 'ru' ? 'Создать машину / станок' : 'Create Machine'}
          {/if}
        </h3>
        <div class="modal-sub">
          {locale === 'ru'
            ? 'Создаёт только выбранный элемент в вашем проекте. Рецепты и цепочки не добавляются автоматически.'
            : 'Creates only this element in your project. No recipes or chains are auto-generated.'}
        </div>
      </div>
      <button type="button" class="btn-ghost" onclick={onClose}>✕</button>
    </div>

    {#if !editingVisualTarget}
      <div class="tabs-bar">
        <button type="button" class:active={mode === 'resource'} onclick={() => (mode = 'resource')}>
          {locale === 'ru' ? 'Ресурс (предмет / жидкость)' : 'Resource (item / fluid)'}
        </button>
        <button type="button" class:active={mode === 'machine'} onclick={() => (mode = 'machine')}>
          {locale === 'ru' ? 'Машина / станок' : 'Machine'}
        </button>
      </div>
    {/if}

    <form class="creator-grid" onsubmit={handleSubmit}>
      <div class="creator-form">
        {#if !editingVisualTarget}
          <label class="field-group">
            <span class="field-label">{locale === 'ru' ? 'Название (RU или EN)' : 'Name (RU or EN)'}</span>
            <input
              type="text"
              class="text-input"
              bind:value={displayName}
              placeholder={locale === 'ru' ? 'Например: Шестерня или Brass Ingot' : 'e.g. Cogwheel or Brass Ingot'}
              oninput={() => {
                if (!searchQuery || searchQuery === displayName.slice(0, -1)) {
                  searchQuery = displayName;
                }
              }}
              required
            />
          </label>

          <label class="field-group">
            <span class="field-label">{locale === 'ru' ? 'ID ресурса (опционально)' : 'Resource ID (optional)'}</span>
            <input
              type="text"
              class="text-input"
              bind:value={customId}
              placeholder={locale === 'ru' ? 'Например: create:cogwheel или custom:my_alloy' : 'e.g. create:cogwheel or custom:my_alloy'}
            />
          </label>

          {#if mode === 'resource'}
            <label class="field-group">
              <span class="field-label">{locale === 'ru' ? 'Единица измерения' : 'Unit / Kind'}</span>
              <select class="text-input" bind:value={resKind}>
                <option value="item">{locale === 'ru' ? 'Предмет (предм./мин)' : 'Item (items/min)'}</option>
                <option value="fluid">{locale === 'ru' ? 'Жидкость (мБ/мин)' : 'Fluid (mB/min)'}</option>
              </select>
            </label>
          {:else}
            <label class="field-group">
              <span class="field-label">{locale === 'ru' ? 'Модель расчёта скорости' : 'Speed calculation rule'}</span>
              <select
                class="text-input"
                bind:value={formulaId}
                onchange={() => {
                  const f = VERIFIED_SPEED_FORMULAS.find((x) => x.id === formulaId);
                  if (f && f.defaultStressSU !== null) stressSU = String(f.defaultStressSU);
                }}
              >
                {#each VERIFIED_SPEED_FORMULAS as f (f.id)}
                  <option value={f.id}>{locale === 'ru' ? f.nameRu : f.nameEn}</option>
                {/each}
              </select>
            </label>

            <div class="inline-2">
              <label class="field-group">
                <span class="field-label">{locale === 'ru' ? 'Нагрузка SU / RPM (пусто = не задано)' : 'Stress SU / RPM'}</span>
                <input type="number" step="0.5" min="0" class="text-input" bind:value={stressSU} placeholder="—" />
              </label>
              <label class="field-group">
                <span class="field-label">{locale === 'ru' ? 'RPM по умолчанию' : 'Default RPM'}</span>
                <input type="number" step="8" min="1" max="256" class="text-input" bind:value={defaultRpm} />
              </label>
            </div>
          {/if}

          <label class="field-group">
            <span class="field-label">{locale === 'ru' ? 'Добавить на холст после создания' : 'Add to canvas on save'}</span>
            <select class="text-input" bind:value={addCanvasChoice}>
              <option value="none">{locale === 'ru' ? 'Только сохранить в каталог проекта' : 'Save to project catalog only'}</option>
              {#if mode === 'resource'}
                <option value="source">{locale === 'ru' ? 'Сохранить и добавить узел «Источник»' : 'Save & add Source node'}</option>
                <option value="target">{locale === 'ru' ? 'Сохранить и добавить узел «Цель»' : 'Save & add Target node'}</option>
              {:else}
                <option value="machine">{locale === 'ru' ? 'Сохранить и добавить узел «Машина»' : 'Save & add Machine node'}</option>
              {/if}
            </select>
          </label>
        {/if}

        <div class="matcher-box">
          <div class="field-label">
            {locale === 'ru'
              ? `Поиск 3D-модели и текстуры (в базе: ${visualEntries.length} позиций)`
              : `Search 3D model & texture (${visualEntries.length} indexed)`}
          </div>
          <div class="search-row">
            <input
              type="search"
              class="text-input"
              bind:value={searchQuery}
              placeholder={locale === 'ru' ? 'ID, путь модели или RU/EN название...' : 'ID, path or RU/EN name...'}
              onkeydown={(e) => {
                if (e.key === 'Enter') {
                  e.preventDefault();
                  runVisualLookup();
                }
              }}
            />
            <button type="button" class="btn-primary" onclick={runVisualLookup}>
              {locale === 'ru' ? 'Найти' : 'Match'}
            </button>
          </div>

          {#if lastMatch?.status === 'ambiguous'}
            <div class="cand-hint warn">
              {locale === 'ru'
                ? `Найдено несколько совпадений (${lastMatch.candidates.length}). Выберите точный вариант:`
                : `Multiple matches (${lastMatch.candidates.length}). Pick the exact asset:`}
            </div>
            <div class="cand-grid">
              {#each lastMatch.candidates as c (c.id)}
                <button
                  type="button"
                  class="cand-card"
                  class:selected={selectedVisual?.visualId === c.id}
                  onclick={() => applyVisualCandidate(c, lastMatch!.reason)}
                >
                  {#if c.texturePath}
                    <img src={`/local-originals/${c.texturePath}`} alt={c.id} class="cand-img" />
                  {:else}
                    <img src="/assets-fallback/item-fallback.svg" alt={c.id} class="cand-img" />
                  {/if}
                  <div class="cand-text">
                    <div class="cand-name">{locale === 'ru' ? (c.nameRu || c.nameEn) : c.nameEn}</div>
                    <div class="cand-id">{c.id}</div>
                    <span class="cand-tag">{c.hasModel3D ? '3D JSON' : c.modelSupport}</span>
                  </div>
                </button>
              {/each}
            </div>
          {:else if lastMatch?.status === 'not-found'}
            <div class="cand-hint">
              {visualEntries.length === 0
                ? locale === 'ru'
                  ? 'Локальный индекс assets-local/visual-index.json отсутствует — используется нейтральная заглушка или ручная загрузка.'
                  : 'Local assets-local/visual-index.json not present — using neutral fallback or manual upload.'
                : locale === 'ru'
                  ? 'Точное совпадение не найдено — используется заглушка. Можно загрузить свой PNG / JSON ниже.'
                  : 'No exact match found — using fallback. You can upload custom PNG / JSON below.'}
            </div>
          {:else if lastMatch?.status === 'unique' && lastMatch.best}
            <div class="cand-hint ok">
              {locale === 'ru'
                ? `Найдено однозначно (${lastMatch.reason}): ${lastMatch.best.id}`
                : `Unambiguous match (${lastMatch.reason}): ${lastMatch.best.id}`}
            </div>
          {/if}

          <div class="override-actions">
            <label class="btn-secondary btn-xs file-btn">
              {locale === 'ru' ? 'Свой .png' : 'Custom .png'}
              <input type="file" accept=".png,image/png" onchange={handleCustomPngUpload} hidden />
            </label>
            <label class="btn-secondary btn-xs file-btn">
              {locale === 'ru' ? 'Своя 3D .json' : 'Custom 3D .json'}
              <input type="file" accept=".json,application/json" onchange={handleCustomModelJsonUpload} hidden />
            </label>
            <button type="button" class="btn-secondary btn-xs" onclick={disableVisual}>
              {locale === 'ru' ? 'Откл. визуал (заглушка)' : 'Use fallback'}
            </button>
          </div>
        </div>

        <div class="modal-footer">
          <button type="button" class="btn-secondary" onclick={onClose}>
            {locale === 'ru' ? 'Отмена' : 'Cancel'}
          </button>
          <button type="submit" class="btn-primary">
            {editingVisualTarget
              ? locale === 'ru' ? 'Применить модель/текстуру' : 'Apply Visual'
              : locale === 'ru' ? 'Сохранить в проект' : 'Save to Project'}
          </button>
        </div>
      </div>

      <div class="creator-preview">
        <div class="field-label">{locale === 'ru' ? 'Предпросмотр 3D / текстуры' : '3D / Texture Preview'}</div>
        <ModelViewer3D
          {locale}
          visual={selectedVisual}
          fallbackUrl={resKind === 'fluid' ? '/assets-fallback/fluid-fallback.svg' : '/assets-fallback/item-fallback.svg'}
          size={210}
        />
        {#if selectedVisual?.visualId}
          <div class="preview-meta">
            <div><strong>ID:</strong> <code>{selectedVisual.visualId}</code></div>
            {#if selectedVisual.modelPath}
              <div><strong>JSON:</strong> <code>{selectedVisual.modelPath}</code></div>
            {/if}
            {#if selectedVisual.texturePath}
              <div><strong>PNG:</strong> <code>{selectedVisual.texturePath}</code></div>
            {/if}
          </div>
        {/if}
      </div>
    </form>
  </div>
</div>

<style>
  .modal-lg {
    width: min(860px, 95vw);
    max-height: 90vh;
    overflow: auto;
  }

  .tabs-bar {
    display: flex;
    gap: 6px;
    padding: 8px 18px 0;
    border-bottom: 1px solid var(--border);
  }
  .tabs-bar button {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 8px 12px;
    font-size: 13px;
    font-weight: 600;
    cursor: pointer;
    border-bottom: 2px solid transparent;
  }
  .tabs-bar button.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .creator-grid {
    display: grid;
    grid-template-columns: 1fr 260px;
    gap: 18px;
    padding: 16px 18px;
  }

  .creator-form {
    display: flex;
    flex-direction: column;
    gap: 11px;
  }

  .creator-preview {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    border-left: 1px solid var(--border);
    padding-left: 14px;
  }

  .inline-2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
  }

  .matcher-box {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    background: rgba(15, 23, 42, 0.25);
  }

  .search-row {
    display: flex;
    gap: 6px;
  }
  .search-row input {
    flex: 1;
  }

  .cand-hint {
    font-size: 11.5px;
    color: var(--text-muted);
  }
  .cand-hint.warn {
    color: #fbbf24;
  }
  .cand-hint.ok {
    color: #34d399;
  }

  .cand-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(175px, 1fr));
    gap: 6px;
    max-height: 190px;
    overflow: auto;
  }

  .cand-card {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg-panel);
    color: var(--text-primary);
    text-align: left;
    cursor: pointer;
  }
  .cand-card:hover,
  .cand-card.selected {
    border-color: var(--accent);
    background: rgba(245, 158, 11, 0.12);
  }

  .cand-img {
    width: 28px;
    height: 28px;
    image-rendering: pixelated;
    object-fit: contain;
    flex-shrink: 0;
  }

  .cand-text {
    min-width: 0;
    flex: 1;
  }
  .cand-name {
    font-size: 11.5px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cand-id {
    font-size: 10px;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .cand-tag {
    font-size: 9.5px;
    color: var(--accent);
    font-weight: 700;
  }

  .override-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .preview-meta {
    width: 100%;
    font-size: 10.5px;
    color: var(--text-muted);
    word-break: break-all;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 6px;
  }

  @media (max-width: 760px) {
    .creator-grid {
      grid-template-columns: 1fr;
    }
    .creator-preview {
      border-left: none;
      border-top: 1px solid var(--border);
      padding-left: 0;
      padding-top: 12px;
    }
  }
</style>
