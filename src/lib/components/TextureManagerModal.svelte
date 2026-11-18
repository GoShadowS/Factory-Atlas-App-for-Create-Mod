<script lang="ts">
  import { assetIdToOverrideRelPaths } from '../assets/assetManager';
  import type { AssetManifestEntry, Locale } from '../types/atlas';
  import PixelSprite from './PixelSprite.svelte';

  interface Props {
    locale: Locale;
    manifest: AssetManifestEntry[];
    onUploadOverride: (assetId: string, dataUrl: string, fileName: string) => void;
    onResetOverride: (assetId: string) => void;
    onClose: () => void;
  }

  let { locale, manifest, onUploadOverride, onResetOverride, onClose }: Props = $props();
  let query = $state('');
  let statusFilter = $state<'all' | 'temporary-original' | 'user-override' | 'fallback'>('all');

  const counts = $derived.by(() => {
    let orig = 0;
    let ovr = 0;
    let fb = 0;
    for (const m of manifest) {
      if (m.status === 'temporary-original') orig++;
      else if (m.status === 'user-override') ovr++;
      else fb++;
    }
    return { orig, ovr, fb };
  });

  const filtered = $derived(
    manifest.filter((m) => {
      if (statusFilter !== 'all' && m.status !== statusFilter) return false;
      const q = query.trim().toLowerCase();
      if (!q) return true;
      return (
        m.id.toLowerCase().includes(q) ||
        m.nameRu.toLowerCase().includes(q) ||
        m.nameEn.toLowerCase().includes(q) ||
        m.modId.toLowerCase().includes(q)
      );
    })
  );

  function handleFilePick(assetId: string, e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    if (!file.name.toLowerCase().endsWith('.png')) return;
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === 'string') {
        onUploadOverride(assetId, reader.result, file.name);
      }
    };
    reader.readAsDataURL(file);
  }

  function statusLabel(status: AssetManifestEntry['status']): string {
    if (status === 'user-override') {
      return locale === 'ru' ? 'Пользовательская перерисовка' : 'User Override';
    }
    if (status === 'temporary-original') {
      return locale === 'ru' ? 'Временный оригинал' : 'Temporary Original';
    }
    return locale === 'ru' ? 'Заглушка (нет оригинала)' : 'Fallback (No Upstream)';
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Texture Manager">
  <div class="modal-card modal-xl">
    <div class="modal-header">
      <div>
        <h3>{locale === 'ru' ? 'Менеджер текстур и переопределений' : 'Texture Manifest & Overrides'}</h3>
        <div class="modal-sub">
          {locale === 'ru'
            ? 'Локальные оригиналы подключены из assets-local/upstream-originals/ (не попадают в Git и публичный релиз). Свои PNG-перерисовки можно загрузить ниже или положить в assets-overrides/.'
            : 'Local originals are loaded from assets-local/upstream-originals/ (excluded from Git/public release). Drop custom PNGs in assets-overrides/ or upload below.'}
        </div>
      </div>
      <button type="button" class="btn-ghost" onclick={onClose}>✕</button>
    </div>

    <div class="stats-bar">
      <span class="pill pill-orig">
        {locale === 'ru' ? 'Временные оригиналы' : 'Temp Originals'}: <strong>{counts.orig}</strong>
      </span>
      <span class="pill pill-ovr">
        {locale === 'ru' ? 'Перерисовки' : 'User Overrides'}: <strong>{counts.ovr}</strong>
      </span>
      <span class="pill pill-fb">
        {locale === 'ru' ? 'Заглушки' : 'Fallbacks'}: <strong>{counts.fb}</strong>
      </span>

      <div class="filter-controls">
        <input
          type="search"
          class="search-inp"
          bind:value={query}
          placeholder={locale === 'ru' ? 'Поиск ID или мода...' : 'Search ID or mod...'}
        />
        <select class="search-inp" bind:value={statusFilter}>
          <option value="all">{locale === 'ru' ? 'Все статусы' : 'All statuses'}</option>
          <option value="temporary-original">{locale === 'ru' ? 'Временный оригинал' : 'Temp original'}</option>
          <option value="user-override">{locale === 'ru' ? 'Перерисовки' : 'User override'}</option>
          <option value="fallback">{locale === 'ru' ? 'Заглушки' : 'Fallback'}</option>
        </select>
      </div>
    </div>

    <div class="table-wrap">
      <table class="asset-table">
        <thead>
          <tr>
            <th>{locale === 'ru' ? 'Вид' : 'Sprite'}</th>
            <th>{locale === 'ru' ? 'ID и Название' : 'ID & Name'}</th>
            <th>{locale === 'ru' ? 'Источник / Версия / Лицензия' : 'Source / Version / License'}</th>
            <th>{locale === 'ru' ? 'Размер / Файл замены' : 'Size / Override path'}</th>
            <th>{locale === 'ru' ? 'Статус' : 'Status'}</th>
            <th>{locale === 'ru' ? 'Действие' : 'Action'}</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as entry (entry.id)}
            {@const [flatRel] = assetIdToOverrideRelPaths(entry.id)}
            <tr>
              <td><PixelSprite asset={entry} size={32} /></td>
              <td>
                <div class="id-main">{locale === 'ru' ? entry.nameRu : entry.nameEn}</div>
                <div class="id-code">{entry.id}</div>
              </td>
              <td>
                <div class="src-line">{entry.sourceRepo} ({entry.modVersion})</div>
                <div class="src-sub">{entry.license}</div>
                {#if entry.missingUpstreamReason}
                  <div class="miss-hint">{entry.missingUpstreamReason}</div>
                {/if}
              </td>
              <td>
                <div>{entry.recommendedSize}</div>
                <div class="id-code">assets-overrides/{flatRel}</div>
              </td>
              <td>
                <span class={`badge status-${entry.status}`}>{statusLabel(entry.status)}</span>
              </td>
              <td>
                <div class="act-row">
                  <label class="btn-secondary btn-xs file-lbl">
                    {locale === 'ru' ? 'Заменить PNG' : 'Replace PNG'}
                    <input type="file" accept=".png,image/png" onchange={(e) => handleFilePick(entry.id, e)} hidden />
                  </label>
                  {#if entry.status === 'user-override'}
                    <button type="button" class="btn-ghost btn-xs" onclick={() => onResetOverride(entry.id)}>
                      {locale === 'ru' ? 'Сброс' : 'Reset'}
                    </button>
                  {/if}
                </div>
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
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
    width: min(1100px, 96vw);
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border-subtle);
  }
  .modal-header h3 { margin: 0 0 4px; font-size: 16px; }
  .modal-sub { font-size: 12px; color: var(--text-muted); max-width: 860px; }

  .stats-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 18px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    flex-wrap: wrap;
  }
  .pill { font-size: 12px; padding: 4px 10px; border-radius: 999px; border: 1px solid var(--border-subtle); }
  .pill-orig { background: rgba(245, 158, 11, 0.14); color: #fbbf24; }
  .pill-ovr { background: rgba(52, 211, 153, 0.14); color: #34d399; }
  .pill-fb { background: rgba(148, 163, 184, 0.14); color: #cbd5e1; }

  .filter-controls {
    margin-left: auto;
    display: flex;
    gap: 8px;
  }
  .search-inp {
    padding: 6px 10px;
    border-radius: 6px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12px;
  }

  .table-wrap {
    flex: 1;
    overflow: auto;
    padding: 8px 18px 16px;
  }
  .asset-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 12px;
  }
  .asset-table th {
    text-align: left;
    font-size: 11px;
    color: var(--text-muted);
    padding: 8px 6px;
    border-bottom: 1px solid var(--border-subtle);
    position: sticky;
    top: 0;
    background: var(--bg-panel);
  }
  .asset-table td {
    padding: 8px 6px;
    border-bottom: 1px solid var(--border-subtle);
    vertical-align: middle;
  }

  .id-main { font-weight: 600; }
  .id-code { font-family: monospace; font-size: 11px; color: var(--text-muted); }
  .src-line { font-size: 11.5px; }
  .src-sub { font-size: 11px; color: var(--text-muted); }
  .miss-hint { font-size: 10.5px; color: #fdba74; margin-top: 2px; }

  .badge {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 6px;
    font-weight: 600;
    white-space: nowrap;
  }
  .status-temporary-original { background: rgba(245, 158, 11, 0.18); color: #fbbf24; }
  .status-user-override { background: rgba(52, 211, 153, 0.18); color: #34d399; }
  .status-fallback { background: rgba(148, 163, 184, 0.18); color: #cbd5e1; }

  .act-row { display: flex; gap: 6px; align-items: center; }
  .file-lbl { cursor: pointer; display: inline-flex; }
</style>
