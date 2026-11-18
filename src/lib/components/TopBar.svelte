<script lang="ts">
  import { t } from '../i18n/translations';
  import type { FactoryProject, Locale, ThemeMode } from '../types/atlas';

  interface Props {
    projects: FactoryProject[];
    activeProjectId: string;
    locale: Locale;
    theme: ThemeMode;
    runtimeBadge?: string;
    runtimeTooltip?: string;
    canUndo: boolean;
    canRedo: boolean;
    dirtyProjectIds: Set<string>;
    onSelectProject: (id: string) => void;
    onNewProject: () => void;
    onCloseProject: (id: string) => void;
    onOpenProjectFile: () => void;
    onSaveProjectFile: () => void;
    onUndo: () => void;
    onRedo: () => void;
    onOpenProfile: () => void;
    onOpenImport: () => void;
    onOpenSchematic: () => void;
    onOpenTextures: () => void;
    onExportSvg: () => void;
    onExportPng: () => void;
    onExportCsv: () => void;
    onToggleLocale: () => void;
    onToggleTheme: () => void;
  }

  let {
    projects,
    activeProjectId,
    locale,
    theme,
    runtimeBadge,
    runtimeTooltip,
    canUndo,
    canRedo,
    dirtyProjectIds,
    onSelectProject,
    onNewProject,
    onCloseProject,
    onOpenProjectFile,
    onSaveProjectFile,
    onUndo,
    onRedo,
    onOpenProfile,
    onOpenImport,
    onOpenSchematic,
    onOpenTextures,
    onExportSvg,
    onExportPng,
    onExportCsv,
    onToggleLocale,
    onToggleTheme,
  }: Props = $props();

  let exportOpen = $state(false);
  const tr = $derived(t(locale));
</script>

<header class="topbar">
  <div class="brand-cluster">
    <img
      src="/assets-fallback/factory-atlas-icon.svg"
      alt="Factory Atlas"
      class="brand-icon"
      width="28"
      height="28"
    />
    <div class="brand-text">
      <div class="brand-title">
        <span>{tr.appName}</span>
        <span class="offline-pill">{tr.offlineBadge}</span>
        {#if runtimeBadge}
          <span class="mode-pill" title={runtimeTooltip ?? ''}>{runtimeBadge}</span>
        {/if}
      </div>
      <div class="brand-sub">{tr.appSubtitle}</div>
    </div>
  </div>

  <nav class="project-tabs" aria-label="Open projects">
    {#each projects as p (p.id)}
      <div class="tab-chip" class:active={p.id === activeProjectId}>
        <button
          type="button"
          class="tab-select"
          onclick={() => onSelectProject(p.id)}
          title={p.name}
        >
          <span class="tab-name">{p.name}</span>
          {#if dirtyProjectIds.has(p.id)}
            <span class="dirty-dot" title="Есть несохранённые изменения">●</span>
          {/if}
        </button>
        {#if projects.length > 1}
          <button
            type="button"
            class="tab-close"
            onclick={() => onCloseProject(p.id)}
            aria-label={`Close ${p.name}`}
          >
            ×
          </button>
        {/if}
      </div>
    {/each}
    <button type="button" class="btn-ghost btn-sm" onclick={onNewProject} title={tr.newProject}>
      +
    </button>
  </nav>

  <div class="actions-cluster">
    <div class="btn-group">
      <button type="button" class="btn-secondary" onclick={onOpenProjectFile}>{tr.openProject}</button>
      <button type="button" class="btn-primary" onclick={onSaveProjectFile}>{tr.saveProject}</button>
    </div>

    <div class="btn-group">
      <button
        type="button"
        class="btn-secondary"
        disabled={!canUndo}
        onclick={onUndo}
        title={`${tr.undo} (Ctrl+Z)`}
      >
        ↶
      </button>
      <button
        type="button"
        class="btn-secondary"
        disabled={!canRedo}
        onclick={onRedo}
        title={`${tr.redo} (Ctrl+Y)`}
      >
        ↷
      </button>
    </div>

    <button type="button" class="btn-secondary" onclick={onOpenProfile}>{tr.profileBtn}</button>
    <button type="button" class="btn-secondary" onclick={onOpenImport}>{tr.importBtn}</button>
    <button type="button" class="btn-secondary" onclick={onOpenSchematic}>{tr.schematicBtn}</button>
    <button type="button" class="btn-secondary" onclick={onOpenTextures}>{tr.texturesBtn}</button>

    <div class="export-dropdown">
      <button
        type="button"
        class="btn-secondary"
        onclick={() => (exportOpen = !exportOpen)}
        aria-expanded={exportOpen}
      >
        {tr.exportMenu} ▾
      </button>
      {#if exportOpen}
        <div class="dropdown-menu" role="menu">
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              exportOpen = false;
              onExportPng();
            }}
          >
            {tr.exportPng}
          </button>
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              exportOpen = false;
              onExportSvg();
            }}
          >
            {tr.exportSvg}
          </button>
          <button
            type="button"
            role="menuitem"
            onclick={() => {
              exportOpen = false;
              onExportCsv();
            }}
          >
            {tr.exportCsv}
          </button>
        </div>
      {/if}
    </div>

    <button type="button" class="btn-ghost" onclick={onToggleLocale} title="Switch RU/EN">
      {locale.toUpperCase()}
    </button>
    <button
      type="button"
      class="btn-ghost"
      onclick={onToggleTheme}
      title={theme === 'dark' ? tr.themeLight : tr.themeDark}
    >
      {theme === 'dark' ? '☀' : '☾'}
    </button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 14px;
    background: var(--bg-panel);
    border-bottom: 1px solid var(--border-subtle);
    min-height: 54px;
    flex-wrap: wrap;
    z-index: 30;
  }

  .brand-cluster {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-right: 6px;
  }

  .brand-icon {
    border-radius: 6px;
    flex-shrink: 0;
  }

  .brand-title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    font-size: 14px;
    color: var(--text-primary);
  }

  .offline-pill {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
    border: 1px solid var(--border-accent);
  }

  .mode-pill {
    font-size: 10px;
    font-weight: 600;
    padding: 1px 6px;
    border-radius: 999px;
    background: var(--bg-elevated);
    color: var(--text-secondary);
    border: 1px solid var(--border-subtle);
  }

  .brand-sub {
    font-size: 11px;
    color: var(--text-muted);
  }

  .project-tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    flex: 1;
    min-width: 180px;
    overflow-x: auto;
    padding-bottom: 1px;
  }

  .tab-chip {
    display: inline-flex;
    align-items: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border-subtle);
    border-radius: 7px;
    max-width: 210px;
    transition: border-color 0.12s;
  }

  .tab-chip.active {
    background: var(--bg-canvas);
    border-color: var(--accent);
  }

  .tab-select {
    background: transparent;
    border: none;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 500;
    padding: 5px 8px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    max-width: 182px;
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dirty-dot {
    color: var(--accent);
    font-size: 9px;
  }

  .tab-close {
    background: transparent;
    border: none;
    color: var(--text-muted);
    padding: 4px 6px 4px 2px;
    cursor: pointer;
    font-size: 14px;
    line-height: 1;
  }
  .tab-close:hover {
    color: var(--danger);
  }

  .actions-cluster {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .btn-group {
    display: inline-flex;
    gap: 4px;
  }

  .export-dropdown {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    right: 0;
    top: calc(100% + 4px);
    min-width: 210px;
    background: var(--bg-panel);
    border: 1px solid var(--border-strong);
    border-radius: 8px;
    box-shadow: 0 12px 30px rgba(0, 0, 0, 0.35);
    padding: 4px;
    display: flex;
    flex-direction: column;
    z-index: 50;
  }

  .dropdown-menu button {
    background: transparent;
    border: none;
    color: var(--text-primary);
    text-align: left;
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 12.5px;
    cursor: pointer;
  }

  .dropdown-menu button:hover {
    background: var(--bg-elevated);
  }
</style>
