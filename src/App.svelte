<script lang="ts">
  import { onMount } from 'svelte';
  import {
    assetIdToOverrideRelPaths,
    buildAssetManifest,
    loadBrowserOverrides,
    removeBrowserOverride,
    saveBrowserOverride,
    type LocalOriginalMeta,
    type TextureOverrideMap,
  } from './lib/assets/assetManager';
  import CatalogSidebar from './lib/components/CatalogSidebar.svelte';
  import DiagnosticsPanel from './lib/components/DiagnosticsPanel.svelte';
  import FactoryCanvas from './lib/components/FactoryCanvas.svelte';
  import InspectorPanel from './lib/components/InspectorPanel.svelte';
  import JarImportModal from './lib/components/JarImportModal.svelte';
  import ProfileModal from './lib/components/ProfileModal.svelte';
  import ResourceCreatorModal from './lib/components/ResourceCreatorModal.svelte';
  import SchematicViewer from './lib/components/SchematicViewer.svelte';
  import TextureManagerModal from './lib/components/TextureManagerModal.svelte';
  import TopBar from './lib/components/TopBar.svelte';
  import { createEmptyProject } from './lib/data/createCatalog';
  import { IncrementalSolverCache, solveFactoryProject } from './lib/engine/solver';
  import { t } from './lib/i18n/translations';
  import {
    exportBalancesToCsv,
    exportProjectToSvg,
    renderSvgToPngBlob,
  } from './lib/io/exporters';
  import { parseAndMigrateProject, serializeProject } from './lib/io/projectFormat';
  import { performRedo, performUndo, pushHistory, type UndoStack } from './lib/state/history';
  import {
    desktopOpenProjectDialog,
    desktopReadFileText,
    desktopSaveExportDialog,
    desktopSaveProjectDialog,
    getDesktopRuntimeContext,
    isTauriDesktop,
    loadDesktopAutosave,
    loadDesktopPngOverrides,
    removeDesktopOverridePng,
    saveDesktopAutosave,
    saveDesktopOverridePng,
    type DesktopRuntimeContext,
  } from './lib/storage/desktopBridge';
  import type {
    FactoryEdge,
    FactoryNode,
    FactoryProject,
    Locale,
    MachineDef,
    ModpackProfile,
    NodeKind,
    RecipeDef,
    ResourceDef,
    ResourceKind,
    ResourceVisualAssignment,
    ThemeMode,
    VisualEntrySummary,
  } from './lib/types/atlas';

  const AUTOSAVE_KEY = 'factory_atlas_workspace_v3';

  let locale = $state<Locale>('ru');
  let theme = $state<ThemeMode>('dark');
  const tr = $derived(t(locale));

  // First launch and new projects ALWAYS start completely empty (no starter chains or catalogs)
  const initialProject = createEmptyProject('Новый проект');
  let projects = $state<FactoryProject[]>([initialProject]);
  let activeProjectId = $state<string>(initialProject.id);
  let historyByProject = $state<Record<string, UndoStack>>({});
  let dirtyProjectIds = $state<Set<string>>(new Set());

  let selectedNodeIds = $state<string[]>([]);
  let selectedEdgeId = $state<string | null>(null);
  let focusNodeId = $state<string | null>(null);

  let toastMsg = $state<string | null>(null);
  let toastTimer: ReturnType<typeof setTimeout> | null = null;

  function showToast(msg: string) {
    toastMsg = msg;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => {
      toastMsg = null;
    }, 3800);
  }

  // Modals
  let showProfileModal = $state(false);
  let showImportModal = $state(false);
  let importInitialTab = $state<'jar' | 'manual'>('jar');
  let showSchematicModal = $state(false);
  let showTextureModal = $state(false);
  let showCreatorModal = $state(false);
  let creatorInitialMode = $state<'resource' | 'machine'>('resource');
  let editingVisualTarget = $state<{
    type: 'resource' | 'machine';
    id: string;
    name: string;
    visual?: ResourceVisualAssignment;
  } | null>(null);

  // Asset pipeline & local visual reference index (NOT added as project items!)
  let visualEntries = $state<VisualEntrySummary[]>([]);
  let localOriginals = $state<LocalOriginalMeta[]>([]);
  let diskOverrides = $state<string[]>([]);
  let browserOverrides = $state<TextureOverrideMap>(loadBrowserOverrides());
  let runtimeCtx = $state<DesktopRuntimeContext>({
    runtime: 'browser-preview',
    mode: 'web',
    dataDir: 'browser:localStorage',
    projectsDir: '',
    overridesDir: 'assets-overrides/',
    initialOpenFilePath: null,
  });
  let autosaveReady = $state(false);

  const runtimeBadge = $derived(
    runtimeCtx.runtime === 'tauri-desktop'
      ? runtimeCtx.mode === 'portable'
        ? locale === 'ru'
          ? 'Портативная (./data)'
          : 'Portable (./data)'
        : locale === 'ru'
          ? 'Установлена (AppData)'
          : 'Installed (AppData)'
      : locale === 'ru'
        ? 'Веб-предпросмотр'
        : 'Web Preview'
  );
  const runtimeTooltip = $derived(
    `${runtimeCtx.mode.toUpperCase()}: ${runtimeCtx.dataDir}`
  );

  let fileInputEl: HTMLInputElement | undefined = $state();
  let canvasRef: FactoryCanvas | undefined = $state();
  const solverCache = new IncrementalSolverCache();

  const activeProject = $derived(
    projects.find((p) => p.id === activeProjectId) ?? projects[0]
  );

  // Catalog maps strictly from active project's own resources, machines, and recipes
  const allRecipes = $derived(activeProject.customRecipes ?? []);
  const recipesById = $derived(new Map(allRecipes.map((r) => [r.id, r])));

  const allMachines = $derived.by(() => {
    const map = new Map<string, MachineDef>((activeProject.customMachines ?? []).map((m) => [m.id, m]));
    for (const r of allRecipes) {
      if (r.machineId && !map.has(r.machineId)) {
        map.set(r.machineId, {
          id: r.machineId,
          nameRu: r.machineId.split(':').pop() ?? r.machineId,
          nameEn: r.machineId.split(':').pop() ?? r.machineId,
          modId: r.machineId.split(':')[0] || 'custom',
          modVersion: r.source.modVersion || 'custom',
          stressImpactSU: null,
          defaultRpm: null,
          maxRpm: 256,
          supportsRpm: false,
          speedFormulaId: 'unknown',
          speedFormulaNoteRu: 'Скорость машины задаётся пользователем.',
          speedFormulaNoteEn: 'Machine speed configured by user.',
          sourceRef: r.source.origin,
        });
      }
    }
    return Array.from(map.values());
  });
  const machinesById = $derived(new Map(allMachines.map((m) => [m.id, m])));

  const allResources = $derived.by(() => {
    const map = new Map<string, ResourceDef>((activeProject.customResources ?? []).map((r) => [r.id, r]));
    for (const r of allRecipes) {
      for (const ing of r.inputs) {
        if (!map.has(ing.resourceId)) {
          map.set(ing.resourceId, {
            id: ing.resourceId,
            kind: ing.kind,
            nameRu: ing.resourceId.split(':').pop() ?? ing.resourceId,
            nameEn: ing.resourceId.split(':').pop() ?? ing.resourceId,
            modId: ing.resourceId.split(':')[0] || 'custom',
            category: ing.kind === 'fluid' ? 'fluid' : 'intermediate',
            color: ing.kind === 'fluid' ? '#38bdf8' : '#f59e0b',
          });
        }
      }
      for (const out of r.outputs) {
        if (!map.has(out.resourceId)) {
          map.set(out.resourceId, {
            id: out.resourceId,
            kind: out.kind,
            nameRu: out.resourceId.split(':').pop() ?? out.resourceId,
            nameEn: out.resourceId.split(':').pop() ?? out.resourceId,
            modId: out.resourceId.split(':')[0] || 'custom',
            category: out.kind === 'fluid' ? 'fluid' : 'product',
            color: out.kind === 'fluid' ? '#38bdf8' : '#f59e0b',
          });
        }
      }
    }
    return Array.from(map.values());
  });
  const resourcesById = $derived(new Map(allResources.map((r) => [r.id, r])));

  const assetManifest = $derived(
    buildAssetManifest({
      localOriginals,
      visualEntries,
      userResources: allResources,
      userMachines: allMachines,
      diskOverrides,
      browserOverrides,
    })
  );
  const assetsById = $derived(new Map(assetManifest.map((a) => [a.id, a])));

  const solverResult = $derived(
    solveFactoryProject(
      activeProject,
      { recipesById, machinesById, resourcesById },
      solverCache
    )
  );

  const selectedNode = $derived(
    selectedNodeIds.length === 1
      ? activeProject.nodes.find((n) => n.id === selectedNodeIds[0]) ?? null
      : null
  );
  const selectedEdge = $derived(
    selectedEdgeId ? activeProject.edges.find((e) => e.id === selectedEdgeId) ?? null : null
  );

  const activeUndo = $derived(historyByProject[activeProject.id] ?? { past: [], future: [] });
  const canUndo = $derived(activeUndo.past.length > 0);
  const canRedo = $derived(activeUndo.future.length > 0);

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.setAttribute('data-theme', theme);
    }
  });

  // Autosave (Tauri desktop writes to isolated dataDir/autosave.json; browser uses localStorage)
  let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    const payload = JSON.stringify({
      activeProjectId,
      projects,
      locale,
      theme,
    });
    if (!autosaveReady) return;
    if (autosaveTimer) clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => {
      if (isTauriDesktop()) {
        void saveDesktopAutosave(payload);
      } else {
        try {
          localStorage.setItem(AUTOSAVE_KEY, payload);
        } catch {
          // Ignore storage quota errors gracefully
        }
      }
    }, 350);
  });

  function applyAutosaveJson(saved: string) {
    const parsed = JSON.parse(saved) as {
      activeProjectId?: string;
      projects?: FactoryProject[];
      locale?: Locale;
      theme?: ThemeMode;
    };
    if (Array.isArray(parsed.projects) && parsed.projects.length > 0) {
      projects = parsed.projects;
      activeProjectId = parsed.activeProjectId || parsed.projects[0].id;
      if (parsed.locale === 'ru' || parsed.locale === 'en') locale = parsed.locale;
      if (parsed.theme === 'dark' || parsed.theme === 'light') theme = parsed.theme;
      showToast(t(locale).autosaveRestored);
    }
  }

  onMount(async () => {
    runtimeCtx = await getDesktopRuntimeContext();

    // Restore autosaved workspace (from isolated disk dataDir in Desktop, or localStorage in Browser)
    try {
      const saved = isTauriDesktop()
        ? await loadDesktopAutosave()
        : localStorage.getItem(AUTOSAVE_KEY);
      if (saved) applyAutosaveJson(saved);
    } catch {
      // Ignore corrupt autosave
    }

    // If launched by opening a .factoryatlas file via Windows file association
    if (isTauriDesktop() && runtimeCtx.initialOpenFilePath) {
      try {
        const opened = await desktopReadFileText(runtimeCtx.initialOpenFilePath);
        if (opened) openProjectFromText(opened.text);
      } catch {
        // Ignore broken initial file cleanly
      }
    }

    autosaveReady = true;

    if (isTauriDesktop()) {
      // Desktop (Installed / Portable) reads PNG overrides strictly from local disk without any HTTP server
      try {
        const knownIds = [
          ...allResources.map((r) => r.id),
          ...allMachines.map((m) => m.id),
        ];
        const diskPngs = await loadDesktopPngOverrides(knownIds);
        if (Object.keys(diskPngs).length > 0) {
          browserOverrides = { ...browserOverrides, ...diskPngs };
        }
      } catch {
        // Fallback SVGs still render cleanly
      }
    } else {
      // Dev / browser-preview server endpoints (never called in Tauri Desktop builds)
      try {
        const [visRes, origRes, ovrRes] = await Promise.all([
          fetch('/api/visual-index'),
          fetch('/api/local-originals-index'),
          fetch('/api/overrides-list'),
        ]);
        if (visRes.ok) {
          const data = (await visRes.json()) as { entries?: VisualEntrySummary[] };
          if (Array.isArray(data.entries)) visualEntries = data.entries;
        }
        if (origRes.ok) {
          const data = (await origRes.json()) as { copied?: LocalOriginalMeta[] };
          if (Array.isArray(data.copied)) localOriginals = data.copied;
        }
        if (ovrRes.ok) {
          const data = (await ovrRes.json()) as { files?: string[] };
          if (Array.isArray(data.files)) diskOverrides = data.files;
        }
      } catch {
        // Pure static mode falls back cleanly to built-in SVGs
      }
    }
  });

  function mutateActiveProject(mutator: (draft: FactoryProject) => void, recordHistory = true) {
    const idx = projects.findIndex((p) => p.id === activeProject.id);
    if (idx === -1) return;
    const prev = projects[idx];
    if (recordHistory) {
      historyByProject = {
        ...historyByProject,
        [prev.id]: pushHistory(activeUndo, prev),
      };
    }
    const next = structuredClone(prev);
    mutator(next);
    next.updatedAt = new Date().toISOString();
    projects = projects.map((p, i) => (i === idx ? next : p));
    dirtyProjectIds = new Set([...dirtyProjectIds, prev.id]);
  }

  function handleUndo() {
    const res = performUndo(activeUndo, activeProject);
    if (!res) return;
    historyByProject = { ...historyByProject, [activeProject.id]: res.stack };
    projects = projects.map((p) => (p.id === activeProject.id ? res.next : p));
  }

  function handleRedo() {
    const res = performRedo(activeUndo, activeProject);
    if (!res) return;
    historyByProject = { ...historyByProject, [activeProject.id]: res.stack };
    projects = projects.map((p) => (p.id === activeProject.id ? res.next : p));
  }

  function nextFreePosition(): { x: number; y: number } {
    const count = activeProject.nodes.length;
    return {
      x: 140 + ((count * 50) % 450),
      y: 120 + ((count * 40) % 300),
    };
  }

  function addNodeKind(kind: NodeKind) {
    const pos = nextFreePosition();
    const id = `n-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
    const firstRes = allResources[0];
    const node: FactoryNode = {
      id,
      kind,
      title: tr.nodeKinds[kind],
      x: pos.x,
      y: pos.y,
      resourceId:
        kind === 'source' || kind === 'target' || kind === 'byproduct'
          ? firstRes?.id
          : undefined,
      resourceKind:
        kind === 'source' || kind === 'target' || kind === 'byproduct'
          ? firstRes?.kind ?? 'item'
          : undefined,
      sourceRatePerMin: kind === 'source' ? 60 : undefined,
      targetRatePerMin: kind === 'target' ? 60 : undefined,
      speedMode: kind === 'recipe' || kind === 'machine' ? 'formula' : undefined,
      rpm: kind === 'recipe' || kind === 'machine' ? 64 : undefined,
      noteText: kind === 'note' ? (locale === 'ru' ? 'Пояснение к участку схемы...' : 'Section note...') : undefined,
    };
    mutateActiveProject((draft) => {
      draft.nodes.push(node);
    });
    selectedNodeIds = [id];
    selectedEdgeId = null;
  }

  function addRecipeNode(recipe: RecipeDef) {
    const pos = nextFreePosition();
    const id = `n-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
    const mach = machinesById.get(recipe.machineId);
    const node: FactoryNode = {
      id,
      kind: 'recipe',
      title: locale === 'ru' ? recipe.nameRu : recipe.nameEn,
      x: pos.x,
      y: pos.y,
      recipeId: recipe.id,
      machineId: recipe.machineId,
      rpm: mach?.defaultRpm ?? 64,
      speedMode:
        mach?.speedFormulaId === 'crushing_wheel' || mach?.speedFormulaId === 'encased_fan'
          ? 'assumption'
          : mach?.speedFormulaId === 'unknown'
            ? 'unknown'
            : 'formula',
      stackSizeAssumption:
        mach?.speedFormulaId === 'crushing_wheel' || mach?.speedFormulaId === 'encased_fan'
          ? 16
          : undefined,
    };
    mutateActiveProject((draft) => {
      draft.nodes.push(node);
    });
    selectedNodeIds = [id];
    selectedEdgeId = null;
  }

  function addResourceNode(res: ResourceDef, role: 'source' | 'target' | 'byproduct') {
    const pos = nextFreePosition();
    const id = `n-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
    const resName = locale === 'ru' ? res.nameRu : res.nameEn;
    const node: FactoryNode = {
      id,
      kind: role,
      title:
        role === 'source'
          ? `${locale === 'ru' ? 'Источник' : 'Source'}: ${resName}`
          : role === 'target'
            ? `${locale === 'ru' ? 'Цель' : 'Target'}: ${resName}`
            : `${locale === 'ru' ? 'Побочный' : 'Byproduct'}: ${resName}`,
      x: pos.x,
      y: pos.y,
      resourceId: res.id,
      resourceKind: res.kind,
      sourceRatePerMin: role === 'source' ? (res.kind === 'fluid' ? 1000 : 60) : undefined,
      targetRatePerMin: role === 'target' ? (res.kind === 'fluid' ? 500 : 30) : undefined,
    };
    mutateActiveProject((draft) => {
      draft.nodes.push(node);
    });
    selectedNodeIds = [id];
    selectedEdgeId = null;
  }

  function addMachineNode(mach: MachineDef) {
    const pos = nextFreePosition();
    const id = `n-${Date.now()}-${Math.floor(Math.random() * 1000)}`;
    const node: FactoryNode = {
      id,
      kind: 'machine',
      title: locale === 'ru' ? mach.nameRu : mach.nameEn,
      x: pos.x,
      y: pos.y,
      machineId: mach.id,
      rpm: mach.defaultRpm ?? 64,
      speedMode: mach.speedFormulaId === 'unknown' ? 'unknown' : 'formula',
    };
    mutateActiveProject((draft) => {
      draft.nodes.push(node);
    });
    selectedNodeIds = [id];
    selectedEdgeId = null;
  }

  function handleMoveNodes(moves: Array<{ id: string; x: number; y: number }>, commitHistory: boolean) {
    const moveMap = new Map(moves.map((m) => [m.id, m]));
    mutateActiveProject((draft) => {
      for (const n of draft.nodes) {
        const m = moveMap.get(n.id);
        if (m) {
          n.x = m.x;
          n.y = m.y;
        }
      }
    }, commitHistory);
  }

  function handleCreateEdge(params: {
    fromNodeId: string;
    toNodeId: string;
    resourceId: string;
    resourceKind: ResourceKind;
    isByproductPort?: boolean;
  }) {
    const dup = activeProject.edges.some(
      (e) =>
        e.fromNodeId === params.fromNodeId &&
        e.toNodeId === params.toNodeId &&
        e.resourceId === params.resourceId
    );
    if (dup) return;
    const edge: FactoryEdge = {
      id: `e-${Date.now()}-${Math.floor(Math.random() * 1000)}`,
      fromNodeId: params.fromNodeId,
      toNodeId: params.toNodeId,
      resourceId: params.resourceId,
      resourceKind: params.resourceKind,
      rateMode: 'auto',
      isByproductPort: params.isByproductPort,
    };
    mutateActiveProject((draft) => {
      draft.edges.push(edge);
    });
    selectedEdgeId = edge.id;
    selectedNodeIds = [];
  }

  function handleDeleteSelected() {
    if (selectedEdgeId) {
      const targetId = selectedEdgeId;
      mutateActiveProject((draft) => {
        draft.edges = draft.edges.filter((e) => e.id !== targetId);
      });
      selectedEdgeId = null;
      return;
    }
    if (selectedNodeIds.length > 0) {
      const kill = new Set(selectedNodeIds);
      mutateActiveProject((draft) => {
        draft.nodes = draft.nodes.filter((n) => !kill.has(n.id));
        draft.edges = draft.edges.filter((e) => !kill.has(e.fromNodeId) && !kill.has(e.toNodeId));
      });
      selectedNodeIds = [];
    }
  }

  function handleDuplicateSelected() {
    if (selectedNodeIds.length === 0) return;
    const idMap = new Map<string, string>();
    const newIds: string[] = [];
    mutateActiveProject((draft) => {
      const copies: FactoryNode[] = [];
      for (const nid of selectedNodeIds) {
        const orig = draft.nodes.find((n) => n.id === nid);
        if (!orig) continue;
        const copyId = `n-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
        idMap.set(orig.id, copyId);
        newIds.push(copyId);
        copies.push({
          ...structuredClone(orig),
          id: copyId,
          title: `${orig.title} (копия)`,
          x: orig.x + 36,
          y: orig.y + 36,
        });
      }
      draft.nodes.push(...copies);
      for (const e of [...draft.edges]) {
        const nf = idMap.get(e.fromNodeId);
        const nt = idMap.get(e.toNodeId);
        if (nf && nt) {
          draft.edges.push({
            ...structuredClone(e),
            id: `e-${Date.now()}-${Math.floor(Math.random() * 10000)}`,
            fromNodeId: nf,
            toNodeId: nt,
          });
        }
      }
    });
    selectedNodeIds = newIds;
  }

  function createBlankProject() {
    const proj = createEmptyProject(
      locale === 'ru' ? `Фабрика #${projects.length + 1}` : `Factory #${projects.length + 1}`
    );
    projects = [...projects, proj];
    activeProjectId = proj.id;
    selectedNodeIds = [];
    selectedEdgeId = null;
  }

  function closeProjectTab(id: string) {
    if (projects.length <= 1) return;
    const next = projects.filter((p) => p.id !== id);
    projects = next;
    if (activeProjectId === id) {
      activeProjectId = next[0].id;
    }
  }

  function openCreateResourceModal() {
    editingVisualTarget = null;
    creatorInitialMode = 'resource';
    showCreatorModal = true;
  }

  function openCreateMachineModal() {
    editingVisualTarget = null;
    creatorInitialMode = 'machine';
    showCreatorModal = true;
  }

  function openEditVisualModal(type: 'resource' | 'machine', id: string) {
    if (type === 'resource') {
      const r = resourcesById.get(id);
      if (!r) return;
      editingVisualTarget = {
        type: 'resource',
        id: r.id,
        name: locale === 'ru' ? r.nameRu : r.nameEn,
        visual: r.visual,
      };
    } else {
      const m = machinesById.get(id);
      if (!m) return;
      editingVisualTarget = {
        type: 'machine',
        id: m.id,
        name: locale === 'ru' ? m.nameRu : m.nameEn,
        visual: m.visual,
      };
    }
    showCreatorModal = true;
  }

  function triggerDownload(blob: Blob, filename: string) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  function openProjectFromText(text: string) {
    const res = parseAndMigrateProject(text);
    if (!res.ok) {
      showToast(locale === 'ru' ? res.errorRu : res.errorEn);
      return;
    }
    const opened = res.project;
    if (projects.some((p) => p.id === opened.id)) {
      opened.id = `${opened.id}-${Date.now()}`;
    }
    projects = [...projects, opened];
    activeProjectId = opened.id;
    selectedNodeIds = [];
    selectedEdgeId = null;
    if (res.migratedFromVersion) {
      showToast(
        locale === 'ru'
          ? `Проект мигрирован со схемы v${res.migratedFromVersion} до v2.`
          : `Project migrated from schema v${res.migratedFromVersion} to v2.`
      );
    }
    setTimeout(() => canvasRef?.fitView(), 40);
  }

  async function handleSaveProjectFile() {
    const json = serializeProject(activeProject);
    const safeName = activeProject.name.replace(/[^\p{L}\p{N}._-]+/gu, '_') || 'factory-project';
    const fileName = `${safeName}.factoryatlas`;
    if (isTauriDesktop()) {
      try {
        const savedPath = await desktopSaveProjectDialog(fileName, json);
        if (!savedPath) return;
      } catch (err) {
        showToast((err as Error).message);
        return;
      }
    } else {
      triggerDownload(new Blob([json], { type: 'application/json;charset=utf-8' }), fileName);
    }
    const nextDirty = new Set(dirtyProjectIds);
    nextDirty.delete(activeProject.id);
    dirtyProjectIds = nextDirty;
    showToast(tr.savedSuccess);
  }

  async function handleOpenProjectRequest() {
    if (isTauriDesktop()) {
      try {
        const opened = await desktopOpenProjectDialog();
        if (opened) openProjectFromText(opened.text);
      } catch (err) {
        showToast((err as Error).message);
      }
      return;
    }
    fileInputEl?.click();
  }

  async function handleOpenProjectPicked(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const text = await file.text();
      openProjectFromText(text);
    } catch (err) {
      showToast((err as Error).message);
    } finally {
      if (fileInputEl) fileInputEl.value = '';
    }
  }

  async function handleExportSvg() {
    const svg = exportProjectToSvg(activeProject, solverResult, resourcesById, locale);
    const fileName = `${activeProject.name}.svg`;
    if (isTauriDesktop()) {
      await desktopSaveExportDialog({
        defaultName: fileName,
        filterName: 'SVG Vector Image',
        extensions: ['svg'],
        bytes: new TextEncoder().encode(svg),
      });
      return;
    }
    triggerDownload(new Blob([svg], { type: 'image/svg+xml;charset=utf-8' }), fileName);
  }

  async function handleExportPng() {
    const svg = exportProjectToSvg(activeProject, solverResult, resourcesById, locale);
    try {
      const png = await renderSvgToPngBlob(svg, 2);
      const fileName = `${activeProject.name}.png`;
      if (isTauriDesktop()) {
        const buf = new Uint8Array(await png.arrayBuffer());
        await desktopSaveExportDialog({
          defaultName: fileName,
          filterName: 'PNG Image',
          extensions: ['png'],
          bytes: buf,
        });
        return;
      }
      triggerDownload(png, fileName);
    } catch (err) {
      showToast((err as Error).message);
    }
  }

  async function handleExportCsv() {
    const csv = exportBalancesToCsv(activeProject, solverResult, resourcesById, locale);
    const fileName = `${activeProject.name}-bom.csv`;
    if (isTauriDesktop()) {
      await desktopSaveExportDialog({
        defaultName: fileName,
        filterName: 'CSV Table',
        extensions: ['csv'],
        bytes: new TextEncoder().encode(csv),
      });
      return;
    }
    triggerDownload(new Blob([csv], { type: 'text/csv;charset=utf-8' }), fileName);
  }

  async function handleUploadOverride(assetId: string, dataUrl: string, fileName: string) {
    browserOverrides = saveBrowserOverride(assetId, dataUrl, fileName);
    const [flatRel] = assetIdToOverrideRelPaths(assetId);
    if (isTauriDesktop()) {
      try {
        await saveDesktopOverridePng(assetId, dataUrl);
      } catch (err) {
        showToast((err as Error).message);
      }
    } else {
      try {
        const resp = await fetch('/api/assets/override', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ fileName: flatRel, dataUrl }),
        });
        if (resp.ok && !diskOverrides.includes(flatRel)) {
          diskOverrides = [...diskOverrides, flatRel];
        }
      } catch {
        // Static mode: browser override still works cleanly!
      }
    }
    showToast(
      locale === 'ru'
        ? `Перерисовка для ${assetId} применена`
        : `Custom texture override applied for ${assetId}`
    );
  }

  async function handleResetOverride(assetId: string) {
    browserOverrides = removeBrowserOverride(assetId);
    if (isTauriDesktop()) {
      await removeDesktopOverridePng(assetId);
    }
  }

  function handleGlobalKeydown(e: KeyboardEvent) {
    const tag = (document.activeElement?.tagName || '').toLowerCase();
    const editingText = tag === 'input' || tag === 'textarea' || tag === 'select';
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
      if (editingText) return;
      e.preventDefault();
      if (e.shiftKey) handleRedo();
      else handleUndo();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
      if (editingText) return;
      e.preventDefault();
      handleRedo();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'd') {
      if (editingText) return;
      e.preventDefault();
      handleDuplicateSelected();
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && !editingText) {
      if (selectedNodeIds.length > 0 || selectedEdgeId) {
        e.preventDefault();
        handleDeleteSelected();
      }
    }
  }
</script>

<svelte:window
  onkeydown={handleGlobalKeydown}
  onbeforeunload={(e) => {
    if (dirtyProjectIds.size > 0) {
      e.preventDefault();
      e.returnValue = '';
    }
  }}
/>

<div class="app-shell">
  <TopBar
    {projects}
    {activeProjectId}
    {locale}
    {theme}
    {runtimeBadge}
    {runtimeTooltip}
    {canUndo}
    {canRedo}
    {dirtyProjectIds}
    onSelectProject={(id) => {
      activeProjectId = id;
      selectedNodeIds = [];
      selectedEdgeId = null;
    }}
    onNewProject={createBlankProject}
    onCloseProject={closeProjectTab}
    onOpenProjectFile={handleOpenProjectRequest}
    onSaveProjectFile={handleSaveProjectFile}
    onUndo={handleUndo}
    onRedo={handleRedo}
    onOpenProfile={() => (showProfileModal = true)}
    onOpenImport={() => {
      importInitialTab = 'jar';
      showImportModal = true;
    }}
    onOpenSchematic={() => (showSchematicModal = true)}
    onOpenTextures={() => (showTextureModal = true)}
    onExportSvg={handleExportSvg}
    onExportPng={handleExportPng}
    onExportCsv={handleExportCsv}
    onToggleLocale={() => (locale = locale === 'ru' ? 'en' : 'ru')}
    onToggleTheme={() => (theme = theme === 'dark' ? 'light' : 'dark')}
  />

  <input
    bind:this={fileInputEl}
    type="file"
    accept=".factoryatlas,.json"
    onchange={handleOpenProjectPicked}
    hidden
  />

  <div class="main-workspace">
    <CatalogSidebar
      {locale}
      recipes={allRecipes}
      machines={allMachines}
      resources={allResources}
      {assetsById}
      onAddNodeKind={addNodeKind}
      onAddRecipeNode={addRecipeNode}
      onAddResourceNode={addResourceNode}
      onAddMachineNode={addMachineNode}
      onOpenCreateResource={openCreateResourceModal}
      onOpenCreateMachine={openCreateMachineModal}
      onOpenManualRecipe={() => {
        importInitialTab = 'manual';
        showImportModal = true;
      }}
    />

    <div class="center-column">
      <FactoryCanvas
        bind:this={canvasRef}
        nodes={activeProject.nodes}
        edges={activeProject.edges}
        solver={solverResult}
        {locale}
        {selectedNodeIds}
        {selectedEdgeId}
        {recipesById}
        {resourcesById}
        {assetsById}
        {focusNodeId}
        onSelectNodes={(ids) => (selectedNodeIds = ids)}
        onSelectEdge={(id) => (selectedEdgeId = id)}
        onMoveNodes={handleMoveNodes}
        onCreateEdge={handleCreateEdge}
        onDeleteSelected={handleDeleteSelected}
        onDuplicateSelected={handleDuplicateSelected}
        onOpenCreateResource={openCreateResourceModal}
        onOpenCreateMachine={openCreateMachineModal}
        onOpenManualRecipe={() => {
          importInitialTab = 'manual';
          showImportModal = true;
        }}
        onOpenProjectFile={handleOpenProjectRequest}
        onOpenSchematic={() => (showSchematicModal = true)}
      />

      <DiagnosticsPanel
        {locale}
        solver={solverResult}
        nodes={activeProject.nodes}
        {resourcesById}
        {assetsById}
        onFocusNode={(nid) => {
          selectedNodeIds = [nid];
          selectedEdgeId = null;
          focusNodeId = nid;
          setTimeout(() => {
            if (focusNodeId === nid) focusNodeId = null;
          }, 150);
        }}
      />
    </div>

    <InspectorPanel
      {locale}
      {selectedNode}
      {selectedEdge}
      solver={solverResult}
      recipes={allRecipes}
      machines={allMachines}
      resources={allResources}
      {recipesById}
      {machinesById}
      {resourcesById}
      {assetsById}
      onUpdateNode={(nodeId, patch) => {
        mutateActiveProject((draft) => {
          const n = draft.nodes.find((x) => x.id === nodeId);
          if (n) Object.assign(n, patch);
        });
      }}
      onUpdateEdge={(edgeId, patch) => {
        mutateActiveProject((draft) => {
          const e = draft.edges.find((x) => x.id === edgeId);
          if (e) Object.assign(e, patch);
        });
      }}
      onDeleteEdge={(edgeId) => {
        mutateActiveProject((draft) => {
          draft.edges = draft.edges.filter((e) => e.id !== edgeId);
        });
        selectedEdgeId = null;
      }}
      onDeleteNode={(nodeId) => {
        mutateActiveProject((draft) => {
          draft.nodes = draft.nodes.filter((n) => n.id !== nodeId);
          draft.edges = draft.edges.filter((e) => e.fromNodeId !== nodeId && e.toNodeId !== nodeId);
        });
        selectedNodeIds = [];
      }}
      onEditVisual={openEditVisualModal}
    />
  </div>

  {#if toastMsg}
    <div class="toast-banner" role="status">{toastMsg}</div>
  {/if}

  {#if showProfileModal}
    <ProfileModal
      {locale}
      profile={activeProject.profile}
      onSave={(next: ModpackProfile) => {
        mutateActiveProject((draft) => {
          draft.profile = next;
        });
      }}
      onClose={() => (showProfileModal = false)}
    />
  {/if}

  {#if showImportModal}
    <JarImportModal
      {locale}
      machines={allMachines}
      resources={allResources}
      initialTab={importInitialTab}
      onAddRecipes={(newRecipes, extractedTextures) => {
        mutateActiveProject((draft) => {
          const existing = new Set(draft.customRecipes.map((r) => r.id));
          for (const r of newRecipes) {
            if (!existing.has(r.id)) draft.customRecipes.push(r);
          }
        });
        if (extractedTextures) {
          for (const [aid, bytes] of Object.entries(extractedTextures).slice(0, 60)) {
            let binary = '';
            for (let i = 0; i < bytes.length; i++) binary += String.fromCharCode(bytes[i]);
            browserOverrides = saveBrowserOverride(
              aid,
              `data:image/png;base64,${btoa(binary)}`,
              `${aid}.png`
            );
          }
        }
        showToast(
          locale === 'ru'
            ? `Добавлено рецептов: ${newRecipes.length}`
            : `Added recipes: ${newRecipes.length}`
        );
      }}
      onClose={() => (showImportModal = false)}
    />
  {/if}

  {#if showCreatorModal}
    <ResourceCreatorModal
      {locale}
      initialMode={creatorInitialMode}
      {visualEntries}
      {editingVisualTarget}
      onSaveResource={(res, addRole) => {
        mutateActiveProject((draft) => {
          if (!draft.customResources) draft.customResources = [];
          const idx = draft.customResources.findIndex((x) => x.id === res.id);
          if (idx >= 0) draft.customResources[idx] = res;
          else draft.customResources.push(res);
        });
        if (addRole) addResourceNode(res, addRole);
        showToast(
          locale === 'ru'
            ? `Ресурс «${res.nameRu}» сохранён в проект`
            : `Saved resource "${res.nameEn}"`
        );
      }}
      onSaveMachine={(mach, addToCanvas) => {
        mutateActiveProject((draft) => {
          if (!draft.customMachines) draft.customMachines = [];
          const idx = draft.customMachines.findIndex((x) => x.id === mach.id);
          if (idx >= 0) draft.customMachines[idx] = mach;
          else draft.customMachines.push(mach);
        });
        if (addToCanvas) addMachineNode(mach);
        showToast(
          locale === 'ru'
            ? `Машина «${mach.nameRu}» сохранена в проект`
            : `Saved machine "${mach.nameEn}"`
        );
      }}
      onUpdateVisualOnly={(type, id, visual) => {
        mutateActiveProject((draft) => {
          if (type === 'resource') {
            if (!draft.customResources) draft.customResources = [];
            const existing = draft.customResources.find((x) => x.id === id);
            if (existing) {
              existing.visual = visual;
            } else {
              const r = resourcesById.get(id);
              if (r) draft.customResources.push({ ...r, visual });
            }
          } else {
            if (!draft.customMachines) draft.customMachines = [];
            const existing = draft.customMachines.find((x) => x.id === id);
            if (existing) {
              existing.visual = visual;
            } else {
              const m = machinesById.get(id);
              if (m) draft.customMachines.push({ ...m, visual });
            }
          }
        });
        showToast(
          locale === 'ru'
            ? `Визуальный ресурс для ${id} обновлён`
            : `Updated visual asset for ${id}`
        );
      }}
      onClose={() => {
        showCreatorModal = false;
        editingVisualTarget = null;
      }}
    />
  {/if}

  {#if showSchematicModal}
    <SchematicViewer {locale} {assetsById} onClose={() => (showSchematicModal = false)} />
  {/if}

  {#if showTextureModal}
    <TextureManagerModal
      {locale}
      manifest={assetManifest}
      onUploadOverride={handleUploadOverride}
      onResetOverride={handleResetOverride}
      onClose={() => (showTextureModal = false)}
    />
  {/if}
</div>

<style>
  .app-shell {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .main-workspace {
    flex: 1;
    display: flex;
    overflow: hidden;
    min-height: 0;
  }

  .center-column {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .toast-banner {
    position: fixed;
    right: 18px;
    bottom: 230px;
    background: var(--bg-panel);
    border: 1px solid var(--accent);
    color: var(--text-primary);
    padding: 9px 14px;
    border-radius: 9px;
    font-size: 12.5px;
    font-weight: 600;
    box-shadow: 0 12px 28px rgba(0, 0, 0, 0.45);
    z-index: 120;
  }
</style>
