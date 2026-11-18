<script lang="ts">
  import { untrack } from 'svelte';
  import { PRESET_PROFILES } from '../data/createCatalog';
  import type { Locale, ModpackProfile } from '../types/atlas';

  interface Props {
    locale: Locale;
    profile: ModpackProfile;
    onSave: (next: ModpackProfile) => void;
    onClose: () => void;
  }

  let { locale, profile, onSave, onClose }: Props = $props();
  // Intentional snapshot copy for modal editing
  let draft = $state<ModpackProfile>(untrack(() => structuredClone(profile)));
  let newAddonId = $state('');
  let newAddonName = $state('');
  let newAddonVer = $state('');

  function applyPreset(id: string) {
    const found = PRESET_PROFILES.find((p) => p.id === id);
    if (found) draft = structuredClone(found);
  }

  function addAddon() {
    if (!newAddonId.trim() || !newAddonName.trim()) return;
    draft.addons = [
      ...draft.addons,
      {
        id: newAddonId.trim(),
        name: newAddonName.trim(),
        version: newAddonVer.trim() || 'custom',
        verifiedRecipes: false,
      },
    ];
    newAddonId = '';
    newAddonName = '';
    newAddonVer = '';
  }

  function removeAddon(id: string) {
    draft.addons = draft.addons.filter((a) => a.id !== id);
  }
</script>

<div class="modal-backdrop" role="dialog" aria-modal="true" aria-label="Modpack Profile">
  <div class="modal-card">
    <div class="modal-header">
      <div>
        <h3>{locale === 'ru' ? 'Профиль сборки Minecraft / Create' : 'Modpack Profile'}</h3>
        <div class="modal-sub">
          {locale === 'ru'
            ? 'Версии Create и загрузчиков не считаются совместимыми автоматически — профиль фиксируется в файле проекта.'
            : 'Create versions and mod loaders are tracked explicitly inside each project file.'}
        </div>
      </div>
      <button type="button" class="btn-ghost" onclick={onClose}>✕</button>
    </div>

    <div class="modal-body">
      <div class="preset-strip">
        <span class="lbl">{locale === 'ru' ? 'Пресеты:' : 'Presets:'}</span>
        {#each PRESET_PROFILES as preset (preset.id)}
          <button type="button" class="btn-secondary btn-xs" onclick={() => applyPreset(preset.id)}>
            {preset.name}
          </button>
        {/each}
      </div>

      <div class="form-grid">
        <label>
          <span>{locale === 'ru' ? 'Название профиля' : 'Profile Name'}</span>
          <input class="inp" type="text" bind:value={draft.name} />
        </label>
        <label>
          <span>{locale === 'ru' ? 'Версия Minecraft' : 'Minecraft Version'}</span>
          <input class="inp" type="text" bind:value={draft.minecraftVersion} />
        </label>
        <label>
          <span>{locale === 'ru' ? 'Загрузчик модов' : 'Mod Loader'}</span>
          <select class="inp" bind:value={draft.loader}>
            <option value="neoforge">NeoForge</option>
            <option value="forge">Forge</option>
            <option value="fabric">Fabric</option>
          </select>
        </label>
        <label>
          <span>{locale === 'ru' ? 'Версия Create' : 'Create Version'}</span>
          <input class="inp" type="text" bind:value={draft.createVersion} />
        </label>
      </div>

      {#if draft.loader === 'fabric'}
        <div class="info-note">
          {locale === 'ru'
            ? 'Примечание для Fabric (напр. 1.20.1): в JSON рецептах Create Fabric объёмы жидкостей указаны в droplets (81000 = 1000 мБ). При импорте .jar выберите пересчёт droplets.'
            : 'Fabric note: Create Fabric 1.20.1 recipe JSONs use droplets (81000 = 1000 mB). Enable droplet conversion when importing .jar files.'}
        </div>
      {/if}

      <div class="addons-sec">
        <div class="lbl">{locale === 'ru' ? 'Установленные аддоны' : 'Configured Addons'}</div>
        {#each draft.addons as addon (addon.id)}
          <div class="addon-row">
            <strong>{addon.name}</strong>
            <code>{addon.id}</code>
            <span>v{addon.version}</span>
            <span class="ver-pill">
              {addon.verifiedRecipes
                ? locale === 'ru' ? 'Рецепты проверены' : 'Verified'
                : locale === 'ru' ? 'Пользовательский' : 'Custom'}
            </span>
            <button type="button" class="btn-ghost btn-xs" onclick={() => removeAddon(addon.id)}>✕</button>
          </div>
        {/each}

        <div class="add-addon-row">
          <input class="inp" placeholder="modid" bind:value={newAddonId} />
          <input class="inp" placeholder={locale === 'ru' ? 'Название аддона' : 'Addon Name'} bind:value={newAddonName} />
          <input class="inp" placeholder={locale === 'ru' ? 'Версия' : 'Version'} bind:value={newAddonVer} />
          <button type="button" class="btn-secondary btn-xs" onclick={addAddon}>+</button>
        </div>
      </div>
    </div>

    <div class="modal-footer">
      <button type="button" class="btn-secondary" onclick={onClose}>{locale === 'ru' ? 'Отмена' : 'Cancel'}</button>
      <button
        type="button"
        class="btn-primary"
        onclick={() => {
          onSave(draft);
          onClose();
        }}
      >
        {locale === 'ru' ? 'Сохранить профиль' : 'Save Profile'}
      </button>
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
    width: min(760px, 95vw);
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

  .modal-body {
    padding: 16px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .preset-strip { display: flex; flex-wrap: wrap; align-items: center; gap: 6px; }
  .lbl { font-size: 11.5px; font-weight: 700; color: var(--text-muted); }
  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .form-grid label { display: flex; flex-direction: column; gap: 4px; font-size: 11.5px; color: var(--text-muted); }
  .inp {
    padding: 7px 9px;
    border-radius: 7px;
    border: 1px solid var(--border-subtle);
    background: var(--bg-input);
    color: var(--text-primary);
    font-size: 12.5px;
  }
  .info-note {
    font-size: 12px;
    background: rgba(56, 189, 248, 0.14);
    border: 1px solid rgba(56, 189, 248, 0.4);
    color: #7dd3fc;
    padding: 8px 10px;
    border-radius: 7px;
  }
  .addons-sec { display: flex; flex-direction: column; gap: 8px; }
  .addon-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 7px;
    background: var(--bg-elevated);
    font-size: 12.5px;
  }
  .addon-row code { color: var(--text-muted); font-size: 11px; }
  .ver-pill {
    margin-left: auto;
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 999px;
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .add-addon-row { display: grid; grid-template-columns: 1fr 1.4fr 0.8fr auto; gap: 6px; }

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 12px 18px;
    border-top: 1px solid var(--border-subtle);
  }
</style>
