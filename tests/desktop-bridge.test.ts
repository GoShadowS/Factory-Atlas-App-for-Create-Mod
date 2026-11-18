import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  getDesktopRuntimeContext,
  isTauriDesktop,
  loadDesktopAutosave,
  loadDesktopPngOverrides,
  saveDesktopAutosave,
  saveDesktopOverridePng,
} from '../src/lib/storage/desktopBridge';

describe('Desktop IPC Bridge (Installed vs Portable vs Web Preview)', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('defaults cleanly to browser-preview mode outside Tauri desktop', async () => {
    expect(isTauriDesktop()).toBe(false);
    const ctx = await getDesktopRuntimeContext();
    expect(ctx.runtime).toBe('browser-preview');
    expect(ctx.mode).toBe('web');
  });

  it('routes autosave and PNG overrides via Tauri IPC when portable mode is active', async () => {
    const invokeMock = vi.fn(async (cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_runtime_context') {
        return {
          runtime: 'tauri-desktop',
          mode: 'portable',
          dataDir: 'D:\\Apps\\FactoryAtlas\\data',
          projectsDir: 'D:\\Apps\\FactoryAtlas\\data\\projects',
          overridesDir: 'D:\\Apps\\FactoryAtlas\\data\\assets-overrides',
          initialOpenFilePath: null,
        };
      }
      if (cmd === 'desktop_load_autosave') return '{"projects":[]}';
      if (cmd === 'desktop_list_overrides') {
        return [{ relPath: 'create__cogwheel.png', dataUrl: 'data:image/png;base64,iVBORw0KGgo=' }];
      }
      return { ok: true, cmd, args };
    });

    vi.stubGlobal('window', {
      __TAURI_INTERNALS__: { invoke: invokeMock },
    });

    expect(isTauriDesktop()).toBe(true);
    const ctx = await getDesktopRuntimeContext();
    expect(ctx.mode).toBe('portable');
    expect(ctx.dataDir).toContain('data');

    expect(await loadDesktopAutosave()).toBe('{"projects":[]}');
    await saveDesktopAutosave('{"active":"p1"}');
    expect(invokeMock).toHaveBeenCalledWith('desktop_save_autosave', { payload: '{"active":"p1"}' });

    const ovr = await loadDesktopPngOverrides(['create:cogwheel']);
    expect(ovr['create:cogwheel']?.dataUrl.startsWith('data:image/png')).toBe(true);

    await saveDesktopOverridePng('create:brass_ingot', 'data:image/png;base64,iVBORw0KGgo=');
    expect(invokeMock).toHaveBeenCalledWith('desktop_save_override_png', {
      relPath: 'create__brass_ingot.png',
      dataUrl: 'data:image/png;base64,iVBORw0KGgo=',
    });
  });
});
