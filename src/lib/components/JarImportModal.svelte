<script lang="ts">
  import { untrack } from 'svelte';
  import { importModArchive, parseSingleRecipeJson } from '../io/jarImporter';
  import type {
    JarImportReport,
    Locale,
    MachineDef,
    RecipeDef,
    ResourceDef,
    ResourceKind,
  } from '../types/atlas';

  interface Props {
    locale: Locale;
    machines: MachineDef[];
    resources: ResourceDef[];
    initialTab?: 'jar' | 'manual';
    onAddRecipes: (recipes: RecipeDef[], extractedTextures?: Record<string, Uint8Array>) => void;
    onClose: () => void;
  }

  let {
    locale,
    machines,
    resources,
    initialTab = 'jar',
    onAddRecipes,
    onClose,
  }: Props = $props();

  let tab = $state<'jar' | 'manual'>(untrack(() => initialTab));
  let extractTextures = $state(false);
  let convertDroplets = $state(false);
  let busy = $state(false);
  let progressMsg = $state('');
  let report: JarImportReport | null = $state(null);
  let pendingTextures: Record<string, Uint8Array> | undefined = $state(undefined);
  let confirmedImport = $state(false);
  let errorMsg: string | null = $state(null);

  // Manual recipe builder state (starts blank unless user already created resources/machines)
  let manualName = $state('');
  let manualMachineId = $state(untrack(() => machines[0]?.id ?? ''));
  let manualTicks = $state<number | ''>(100);
  let manualInRes = $state(untrack(() => resources[0]?.id ?? ''));
  let manualInKind = $state<ResourceKind>('item');
  let manualInAmount = $state(1);
  let manualOutRes = $state('');
  let manualOutKind = $state<ResourceKind>('item');
  let manualOutAmount = $state(1);
  let manualOutChance = $state(1);

  async function handleArchivePick(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    errorMsg = null;
    report = null;
    pendingTextures = undefined;
    confirmedImport = false;
    busy = true;
    progressMsg = locale === 'ru' ? `Чтение «${file.name}»...` : `Reading "${file.name}"...`;

    try {
      await new Promise((r) => setTimeout(r, 15));
      const bytes = new Uint8Array(await file.arrayBuffer());

      if (file.name.toLowerCase().endsWith('.json')) {
        const txt = new TextDecoder().decode(bytes);
        const parsed = parseSingleRecipeJson(JSON.parse(txt), `data/custom/recipe/${file.name}`, {
          modId: 'custom',
          modVersion: 'manual-json',
          minecraftVersion: '1.21.1',
          loader: 'neoforge',
          convertDroplets,
        });
        if (!parsed.ok) {
          throw new Error(locale === 'ru' ? parsed.reasonRu : parsed.reasonEn);
        }
        report = {
          fileName: file.name,
          detectedModId: 'custom-json',
          detectedModName: file.name,
          detectedModVersion: '1.0',
          detectedLoader: 'neoforge',
          detectedMinecraftVersion: '1.21.1',
          archiveSizeBytes: bytes.byteLength,
          entriesScanned: 1,
          importedCount: 1,
          skippedCount: 0,
          extractedTexturesCount: 0,
          recipes: [parsed.recipe],
          skippedDetails: [],
          warnings: [],
        };
      } else {
        progressMsg =
          locale === 'ru'
            ? 'Проверка архива (лимиты, path traversal) и разбор рецептов...'
            : 'Validating archive safety & parsing recipes...';
        await new Promise((r) => setTimeout(r, 15));
        const res = importModArchive(bytes, file.name, {
          extractTextures,
          convertFabricDroplets: convertDroplets ? true : undefined,
        });
        report = res.report;
        pendingTextures = res.extractedTextures;
      }
    } catch (err) {
      errorMsg = (err as Error).message;
    } finally {
      busy = false;
      progressMsg = '';
    }
  }

  function confirmApplyImported() {
    if (!report) return;
    onAddRecipes(report.recipes, pendingTextures);
    confirmedImport = true;
  }

  function handleSaveManual(e: Event) {
    e.preventDefault();
    const machFinal = manualMachineId.trim() || 'custom:machine';
    const recipe: RecipeDef = {
      id: `custom:manual/${Date.now()}`,
      type: 'custom:manual',
      machineId: machFinal,
      nameRu: manualName.trim() || 'Пользовательский рецепт',
      nameEn: manualName.trim() || 'Custom Recipe',
      inputs: [{ resourceId: manualInRes.trim(), kind: manualInKind, amount: Math.max(0.001, manualInAmount) }],
      outputs: [
        {
          resourceId: manualOutRes.trim(),
          kind: manualOutKind,
          amount: Math.max(0.001, manualOutAmount),
          chance: Math.min(1, Math.max(0.01, manualOutChance)),
          probabilityKind: manualOutChance < 1 ? 'bernoulli' : 'guaranteed',
        },
      ],
      processingTimeTicks: manualTicks === '' ? null : Math.max(1, Number(manualTicks)),
      heatRequirement: 'none',
      source: {
        origin: 'manual',
        modId: 'custom',
        modVersion: 'user',
        minecraftVersion: 'custom',
        loader: 'neoforge',
      },
    };
    onAddRecipes([recipe]);
    onClose();
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Import Recipes">
  <div class="modal-card">
    <div class="modal-header">
      <div>
        <h3>{locale === 'ru' ? 'Импорт рецептов и аддонов' : 'Import Recipes & Addons'}</h3>
        <div class="modal-sub">
          {locale === 'ru'
            ? 'Локальные .jar / .zip / .json читаются только как архивы данных без запуска Java и без отправки в сеть.'
            : 'Local .jar / .zip / .json files are parsed purely as data archives — Java code is never executed.'}
        </div>
      </div>
      <button type="button" class="btn-ghost" onclick={onClose}>✕</button>
    </div>

    <div class="tab-bar">
      <button type="button" class:active={tab === 'jar'} onclick={() => (tab = 'jar')}>
        {locale === 'ru' ? 'Импорт из .jar / .zip / .json' : 'Import from .jar / .zip / .json'}
      </button>
      <button type="button" class:active={tab === 'manual'} onclick={() => (tab = 'manual')}>
        {locale === 'ru' ? 'Добавить рецепт вручную' : 'Add Manual Recipe'}
      </button>
    </div>

    <div class="modal-body">
      {#if tab === 'jar'}
        <div class="import-options">
          <label class="chk">
            <input type="checkbox" bind:checked={extractTextures} />
            <span>
              {locale === 'ru'
                ? 'Также извлечь PNG-текстуры предметов/блоков в локальную сессию'
                : 'Also extract item/block PNG textures into local session'}
            </span>
          </label>
          <label class="chk">
            <input type="checkbox" bind:checked={convertDroplets} />
            <span>
              {locale === 'ru'
                ? 'Принудительно пересчитывать Fabric droplets (1 мБ = 81 droplet)'
                : 'Force Fabric droplets conversion (1 mB = 81 droplets)'}
            </span>
          </label>
        </div>

        <div class="pick-zone">
          <label class="btn-primary file-btn">
            {locale === 'ru' ? 'Выбрать локальный .jar / .zip / .json' : 'Select local .jar / .zip / .json'}
            <input type="file" accept=".jar,.zip,.json" onchange={handleArchivePick} hidden />
          </label>
          {#if busy}
            <div class="progress-box">
              <div class="progress-bar"></div>
              <span>{progressMsg}</span>
            </div>
          {/if}
        </div>

        {#if errorMsg}
          <div class="err-box">{errorMsg}</div>
        {/if}

        {#if report}
          <div class="report-card">
            <div class="report-grid">
              <div><span>Файл:</span> <strong>{report.fileName}</strong></div>
              <div><span>Mod ID:</span> <strong>{report.detectedModId} ({report.detectedModVersion})</strong></div>
              <div><span>Загрузчик / MC:</span> <strong>{report.detectedLoader} · {report.detectedMinecraftVersion}</strong></div>
              <div><span>Проверено записей:</span> <strong>{report.entriesScanned}</strong></div>
              <div class="ok-c"><span>Импортировано рецептов:</span> <strong>{report.importedCount}</strong></div>
              <div class="warn-c"><span>Пропущено / Неподдерж.:</span> <strong>{report.skippedCount}</strong></div>
            </div>

            <div style="margin-top: 10px; display: flex; gap: 8px; align-items: center;">
              <button
                type="button"
                class="btn-primary"
                disabled={confirmedImport || (report.recipes.length === 0 && !pendingTextures)}
                onclick={confirmApplyImported}
              >
                {confirmedImport
                  ? locale === 'ru'
                    ? 'Импорт добавлен в проект ✓'
                    : 'Added to project ✓'
                  : locale === 'ru'
                    ? `Подтвердить и импортировать в проект (${report.recipes.length})`
                    : `Confirm & import to project (${report.recipes.length})`}
              </button>
            </div>

            {#if report.warnings.length > 0}
              <div class="warn-list">
                {#each report.warnings as w}
                  <div>⚠ {w}</div>
                {/each}
              </div>
            {/if}

            {#if report.skippedDetails.length > 0}
              <div class="skipped-title">
                {locale === 'ru' ? 'Причины пропуска (первые 30):' : 'Skipped reasons (first 30):'}
              </div>
              <div class="skipped-list">
                {#each report.skippedDetails.slice(0, 30) as s}
                  <div class="skip-row">
                    <code>{s.path}</code>
                    <span>{locale === 'ru' ? s.messageRu : s.messageEn}</span>
                  </div>
                {/each}
              </div>
            {/if}
          </div>
        {/if}
      {:else}
        <form class="manual-form" onsubmit={handleSaveManual}>
          <datalist id="manual-resources-list">
            {#each resources as r (r.id)}
              <option value={r.id}>{locale === 'ru' ? r.nameRu : r.nameEn}</option>
            {/each}
          </datalist>
          <datalist id="manual-machines-list">
            {#each machines as m (m.id)}
              <option value={m.id}>{locale === 'ru' ? m.nameRu : m.nameEn}</option>
            {/each}
          </datalist>

          <div class="form-row">
            <label>
              <span>{locale === 'ru' ? 'Название рецепта' : 'Recipe name'}</span>
              <input
                class="inp"
                type="text"
                bind:value={manualName}
                placeholder={locale === 'ru' ? 'Например: Прессовка медного листа' : 'e.g. Copper Sheet Pressing'}
                required
              />
            </label>
            <label>
              <span>{locale === 'ru' ? 'Машина (ID или из списка)' : 'Machine (ID or list)'}</span>
              <input
                class="inp"
                type="text"
                list="manual-machines-list"
                bind:value={manualMachineId}
                placeholder="custom:press"
                required
              />
            </label>
          </div>

          <div class="form-row-3">
            <label>
              <span>{locale === 'ru' ? 'Входной ресурс ID' : 'Input Resource ID'}</span>
              <input
                class="inp"
                type="text"
                list="manual-resources-list"
                bind:value={manualInRes}
                placeholder="custom:copper_ingot"
                required
              />
            </label>
            <label>
              <span>{locale === 'ru' ? 'Тип' : 'Kind'}</span>
              <select class="inp" bind:value={manualInKind}>
                <option value="item">{locale === 'ru' ? 'Предмет (шт)' : 'Item'}</option>
                <option value="fluid">{locale === 'ru' ? 'Жидкость (мБ)' : 'Fluid (mB)'}</option>
              </select>
            </label>
            <label>
              <span>{locale === 'ru' ? 'Кол-во' : 'Amount'}</span>
              <input class="inp" type="number" step="0.1" min="0.01" bind:value={manualInAmount} required />
            </label>
          </div>

          <div class="form-row-4">
            <label>
              <span>{locale === 'ru' ? 'Выходной ресурс ID' : 'Output Resource ID'}</span>
              <input
                class="inp"
                type="text"
                list="manual-resources-list"
                bind:value={manualOutRes}
                placeholder="custom:copper_sheet"
                required
              />
            </label>
            <label>
              <span>{locale === 'ru' ? 'Тип' : 'Kind'}</span>
              <select class="inp" bind:value={manualOutKind}>
                <option value="item">{locale === 'ru' ? 'Предмет' : 'Item'}</option>
                <option value="fluid">{locale === 'ru' ? 'Жидкость (мБ)' : 'Fluid (mB)'}</option>
              </select>
            </label>
            <label>
              <span>{locale === 'ru' ? 'Кол-во' : 'Amount'}</span>
              <input class="inp" type="number" step="0.1" min="0.01" bind:value={manualOutAmount} required />
            </label>
            <label>
              <span>{locale === 'ru' ? 'Шанс (0..1)' : 'Chance (0..1)'}</span>
              <input class="inp" type="number" step="0.05" min="0.01" max="1" bind:value={manualOutChance} required />
            </label>
          </div>

          <div class="form-row">
            <label>
              <span>{locale === 'ru' ? 'Длительность (тики, 20 = 1с)' : 'Duration (ticks)'}</span>
              <input class="inp" type="number" min="1" bind:value={manualTicks} />
            </label>
          </div>

          <div class="form-actions">
            <button type="button" class="btn-secondary" onclick={onClose}>
              {locale === 'ru' ? 'Отмена' : 'Cancel'}
            </button>
            <button type="submit" class="btn-primary">
              {locale === 'ru' ? 'Добавить в каталог' : 'Add to catalog'}
            </button>
          </div>
        </form>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(2, 6, 23, 0.72);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    padding: 20px;
  }
  .modal-card {
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: 12px;
    width: min(860px, 95vw);
    max-height: 88vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .modal-header h3 { margin: 0 0 4px; font-size: 16px; }
  .modal-sub { font-size: 12px; color: var(--text-muted); }

  .tab-bar {
    display: flex;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    padding: 0 18px;
    gap: 12px;
  }
  .tab-bar button {
    border: none;
    background: transparent;
    color: var(--text-muted);
    padding: 10px 4px;
    font-size: 13px;
    font-weight: 600;
    border-bottom: 2px solid transparent;
    cursor: pointer;
  }
  .tab-bar button.active {
    color: var(--accent-text);
    border-bottom-color: var(--accent);
  }

  .modal-body {
    padding: 16px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .import-options { display: flex; flex-direction: column; gap: 6px; font-size: 12.5px; }
  .chk { display: flex; align-items: center; gap: 8px; cursor: pointer; }
  .pick-zone { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; }
  .file-btn { cursor: pointer; }

  .progress-box { display: flex; align-items: center; gap: 10px; font-size: 12px; color: var(--text-secondary); }
  .progress-bar {
    width: 90px;
    height: 6px;
    border-radius: 999px;
    background: linear-gradient(90deg, var(--accent), #38bdf8);
  }

  .err-box {
    padding: 10px 12px;
    border-radius: 8px;
    background: rgba(239, 68, 68, 0.16);
    border: 1px solid #ef4444;
    color: #fecaca;
    font-size: 12.5px;
  }

  .report-card {
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 8px;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .report-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
    font-size: 12.5px;
  }
  .report-grid span { color: var(--text-muted); }
  .ok-c strong { color: #34d399; }
  .warn-c strong { color: #fdba74; }
  .warn-list { font-size: 12px; color: #fdba74; }

  .skipped-title { font-size: 12px; font-weight: 700; }
  .skipped-list {
    max-height: 200px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11.5px;
  }
  .skip-row {
    display: flex;
    flex-direction: column;
    background: var(--bg-panel);
    padding: 5px 8px;
    border-radius: 5px;
  }
  .skip-row code { font-size: 10.5px; color: var(--text-muted); }

  .manual-form { display: flex; flex-direction: column; gap: 12px; }
  .form-row { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .form-row-3 { display: grid; grid-template-columns: 2fr 1fr 0.8fr; gap: 10px; }
  .form-row-4 { display: grid; grid-template-columns: 2fr 1fr 0.8fr 0.8fr; gap: 10px; }
  .manual-form label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--text-muted); }
  .inp {
    padding: 7px 9px;
    border-radius: 7px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12.5px;
  }
  .form-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 4px; }
</style>
