import { assetIdToOverrideRelPaths, type TextureOverrideMap } from '../assets/assetManager';

export interface DesktopRuntimeContext {
  runtime: 'tauri-desktop' | 'browser-preview';
  mode: 'installed' | 'portable' | 'web';
  dataDir: string;
  projectsDir: string;
  overridesDir: string;
  initialOpenFilePath: string | null;
}

export interface OpenedFileResult {
  path: string;
  fileName: string;
  text: string;
}

interface TauriInternalsLike {
  invoke: <T = unknown>(cmd: string, args?: Record<string, unknown>) => Promise<T>;
}

function getTauriInternals(): TauriInternalsLike | null {
  if (typeof window === 'undefined') return null;
  const w = window as unknown as { __TAURI_INTERNALS__?: TauriInternalsLike };
  return w.__TAURI_INTERNALS__ && typeof w.__TAURI_INTERNALS__.invoke === 'function'
    ? w.__TAURI_INTERNALS__
    : null;
}

export function isTauriDesktop(): boolean {
  return getTauriInternals() !== null;
}

export async function getDesktopRuntimeContext(): Promise<DesktopRuntimeContext> {
  const tauri = getTauriInternals();
  if (!tauri) {
    return {
      runtime: 'browser-preview',
      mode: 'web',
      dataDir: 'browser:localStorage',
      projectsDir: '',
      overridesDir: 'assets-overrides/',
      initialOpenFilePath: null,
    };
  }
  return await tauri.invoke<DesktopRuntimeContext>('get_runtime_context');
}

export async function loadDesktopAutosave(): Promise<string | null> {
  const tauri = getTauriInternals();
  if (!tauri) return null;
  return await tauri.invoke<string | null>('desktop_load_autosave');
}

export async function saveDesktopAutosave(payload: string): Promise<void> {
  const tauri = getTauriInternals();
  if (!tauri) return;
  await tauri.invoke('desktop_save_autosave', { payload });
}

export async function desktopSaveProjectDialog(
  defaultName: string,
  jsonText: string
): Promise<string | null> {
  const tauri = getTauriInternals();
  if (!tauri) return null;
  return await tauri.invoke<string | null>('desktop_save_project_with_dialog', {
    defaultName,
    jsonText,
  });
}

export async function desktopOpenProjectDialog(): Promise<OpenedFileResult | null> {
  const tauri = getTauriInternals();
  if (!tauri) return null;
  return await tauri.invoke<OpenedFileResult | null>('desktop_open_project_with_dialog');
}

export async function desktopReadFileText(path: string): Promise<OpenedFileResult | null> {
  const tauri = getTauriInternals();
  if (!tauri) return null;
  return await tauri.invoke<OpenedFileResult>('desktop_read_file_text', { path });
}

export async function desktopSaveExportDialog(params: {
  defaultName: string;
  filterName: string;
  extensions: string[];
  bytes: Uint8Array;
}): Promise<string | null> {
  const tauri = getTauriInternals();
  if (!tauri) return null;
  let binary = '';
  for (let i = 0; i < params.bytes.length; i++) {
    binary += String.fromCharCode(params.bytes[i]);
  }
  return await tauri.invoke<string | null>('desktop_save_export_with_dialog', {
    req: {
      defaultName: params.defaultName,
      filterName: params.filterName,
      extensions: params.extensions,
      base64Bytes: btoa(binary),
    },
  });
}

export async function loadDesktopPngOverrides(
  knownIds: string[]
): Promise<TextureOverrideMap> {
  const tauri = getTauriInternals();
  if (!tauri) return {};
  const list = await tauri.invoke<Array<{ relPath: string; dataUrl: string }>>(
    'desktop_list_overrides'
  );
  const byRel = new Map(list.map((i) => [i.relPath, i.dataUrl]));
  const result: TextureOverrideMap = {};
  for (const id of knownIds) {
    const [flatRel, nestedRel] = assetIdToOverrideRelPaths(id);
    const dataUrl = byRel.get(flatRel) ?? byRel.get(nestedRel);
    if (dataUrl) {
      result[id] = {
        dataUrl,
        fileName: flatRel,
        updatedAt: new Date().toISOString(),
      };
    }
  }
  return result;
}

export async function saveDesktopOverridePng(
  assetId: string,
  dataUrl: string
): Promise<void> {
  const tauri = getTauriInternals();
  if (!tauri) return;
  const [flatRel] = assetIdToOverrideRelPaths(assetId);
  await tauri.invoke('desktop_save_override_png', {
    relPath: flatRel,
    dataUrl,
  });
}

export async function removeDesktopOverridePng(assetId: string): Promise<void> {
  const tauri = getTauriInternals();
  if (!tauri) return;
  const [flatRel, nestedRel] = assetIdToOverrideRelPaths(assetId);
  await tauri.invoke('desktop_remove_override_png', { relPath: flatRel });
  await tauri.invoke('desktop_remove_override_png', { relPath: nestedRel });
}
