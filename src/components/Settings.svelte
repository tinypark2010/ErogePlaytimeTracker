<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check } from '@tauri-apps/plugin-updater';
  import { api } from '../lib/api';
  import type { Settings, Theme } from '../lib/types';
  export let ontheme: (theme: Theme) => void = () => {};
  export let ondirty: (dirty: boolean) => void = () => {};
  export let onsaved: (settings: Settings) => void = () => {};
  let settings: Settings = {
      autostart: false,
      auto_check_updates: true,
      skipped_update_version: null,
      reconciliation_seconds: 3,
      close_to_tray: true,
      theme: 'dark',
      screenshot_hotkey: '',
    },
    message = '',
    error = '',
    hotkeyError = '',
    recordingHotkey = false,
    checkingHotkey = false,
    hotkeyStatus = '',
    currentVersion = '',
    updateStatus = '',
    updateError = '',
    checkingUpdate = false,
    installingUpdate = false,
    updateDownloaded = 0,
    updateContentLength: number | undefined;
  let availableUpdate: Awaited<ReturnType<typeof check>> = null;
  let previewUpdateAvailable = false;
  $: availableVersion = availableUpdate?.version ?? '0.1.6';
  let savedSettings: Settings | null = null,
    lastDirty = false;
  $: {
    const dirty =
      recordingHotkey ||
      (savedSettings !== null && JSON.stringify(settings) !== JSON.stringify(savedSettings));
    if (dirty !== lastDirty) {
      lastDirty = dirty;
      ondirty(dirty);
    }
  }
  onMount(async () => {
    try {
      [settings, currentVersion] = await Promise.all([api.settings(), getVersion()]);
      savedSettings = { ...settings };
      ontheme(settings.theme);
      if (settings.screenshot_hotkey) {
        try {
          await api.validateScreenshotHotkey(settings.screenshot_hotkey);
        } catch (e) {
          hotkeyError = String(e);
        }
      }
    } catch (e) {
      error = String(e);
    }
  });
  onDestroy(() => {
    if (recordingHotkey) api.resumeScreenshotHotkey().catch(() => {});
  });
  function previewTheme() {
    ontheme(settings.theme);
  }
  function selectTheme(theme: Theme) {
    settings.theme = theme;
    ontheme(theme);
  }
  async function save() {
    try {
      await api.updateSettings(settings);
      settings = await api.settings();
      savedSettings = { ...settings };
      onsaved(settings);
      ondirty(false);
      message = '保存しました';
      error = '';
      hotkeyError = '';
    } catch (e) {
      const saveError = String(e);
      if (saveError.includes('キー') || saveError.includes('ホット')) {
        hotkeyError = saveError;
      } else {
        error = saveError;
      }
    }
  }
  function hotkeyFromEvent(event: KeyboardEvent) {
    if (['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) return '';
    const modifiers = [
      event.ctrlKey ? 'Ctrl' : '',
      event.altKey ? 'Alt' : '',
      event.shiftKey ? 'Shift' : '',
      event.metaKey ? 'Win' : '',
    ].filter(Boolean);
    let key = event.key;
    if (key.length === 1) key = key.toUpperCase();
    if (key === 'PrintScreen') key = 'PrintScreen';
    return [...modifiers, key].join('+');
  }
  async function recordHotkey(event: KeyboardEvent) {
    if (!recordingHotkey) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === 'Escape') {
      recordingHotkey = false;
      hotkeyStatus = '';
      await api.resumeScreenshotHotkey();
      return;
    }
    const candidate = hotkeyFromEvent(event);
    if (!candidate) return;
    checkingHotkey = true;
    error = '';
    hotkeyError = '';
    hotkeyStatus = `${candidate} を確認中…`;
    try {
      await api.validateScreenshotHotkey(candidate);
      settings.screenshot_hotkey = candidate;
      recordingHotkey = false;
      hotkeyStatus = '';
    } catch (e) {
      hotkeyStatus = '';
      hotkeyError = String(e);
    } finally {
      await api.resumeScreenshotHotkey().catch((e) => (hotkeyError = String(e)));
      checkingHotkey = false;
    }
  }
  async function startHotkeyRecording() {
    hotkeyStatus = '';
    error = '';
    hotkeyError = '';
    try {
      await api.suspendScreenshotHotkey();
      recordingHotkey = true;
    } catch (e) {
      hotkeyError = String(e);
    }
  }
  async function clearHotkey() {
    if (recordingHotkey) await api.resumeScreenshotHotkey();
    settings.screenshot_hotkey = '';
    recordingHotkey = false;
    hotkeyStatus = '';
    error = '';
    hotkeyError = '';
  }
  async function checkForUpdate() {
    if (checkingUpdate || installingUpdate) return;
    checkingUpdate = true;
    availableUpdate = null;
    previewUpdateAvailable = false;
    updateError = '';
    updateStatus = '更新を確認しています…';
    try {
      if (import.meta.env.DEV && import.meta.env.VITE_MOCK_UPDATE === 'true') {
        previewUpdateAvailable = true;
        updateStatus = `バージョン ${availableVersion} を利用できます。`;
        return;
      }
      availableUpdate = await check();
      updateStatus = availableUpdate
        ? `バージョン ${availableUpdate.version} を利用できます。`
        : '現在のバージョンが最新です。';
    } catch (e) {
      updateStatus = '';
      updateError = `更新を確認できませんでした: ${String(e)}`;
    } finally {
      checkingUpdate = false;
    }
  }
  async function installAvailableUpdate() {
    if (previewUpdateAvailable) {
      updateError = 'モック表示のため、実際の更新は行いません。';
      return;
    }
    if (!availableUpdate || installingUpdate) return;
    installingUpdate = true;
    updateError = '';
    updateDownloaded = 0;
    updateContentLength = undefined;
    try {
      await availableUpdate.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          updateContentLength = event.data.contentLength ?? undefined;
        } else if (event.event === 'Progress') {
          updateDownloaded += event.data.chunkLength;
        }
      });
      await relaunch();
    } catch (e) {
      updateError = `更新できませんでした: ${String(e)}`;
      installingUpdate = false;
    }
  }
  function updateProgressText() {
    if (!updateContentLength) return '更新をダウンロードしています…';
    const percent = Math.min(100, Math.round((updateDownloaded / updateContentLength) * 100));
    return `更新をダウンロードしています… ${percent}%`;
  }
</script>

<svelte:window onkeydown={recordHotkey} />

<section class="panel form">
  <h1>設定</h1>
  <label
    >カラーテーマ<select bind:value={settings.theme} onchange={previewTheme}
      ><option value="dark">ダーク</option><option value="light">ライト</option><option value="pink"
        >ピンク</option
      ><option value="blue">ブルー</option></select
    ></label
  >
  <div class="theme-preview" role="group" aria-label="カラーテーマを選択">
    <button
      type="button"
      class="theme-dot theme-dark"
      class:selected={settings.theme === 'dark'}
      aria-label="ダークテーマを選択"
      aria-pressed={settings.theme === 'dark'}
      onclick={() => selectTheme('dark')}
    ></button><button
      type="button"
      class="theme-dot theme-light"
      class:selected={settings.theme === 'light'}
      aria-label="ライトテーマを選択"
      aria-pressed={settings.theme === 'light'}
      onclick={() => selectTheme('light')}
    ></button><button
      type="button"
      class="theme-dot theme-pink"
      class:selected={settings.theme === 'pink'}
      aria-label="ピンクテーマを選択"
      aria-pressed={settings.theme === 'pink'}
      onclick={() => selectTheme('pink')}
    ></button><button
      type="button"
      class="theme-dot theme-blue"
      class:selected={settings.theme === 'blue'}
      aria-label="ブルーテーマを選択"
      aria-pressed={settings.theme === 'blue'}
      onclick={() => selectTheme('blue')}
    ></button>
  </div>
  <label class="check"
    ><input type="checkbox" bind:checked={settings.autostart} /> Windowsログイン時に自動起動</label
  ><label class="check"
    ><input type="checkbox" bind:checked={settings.close_to_tray} /> ウィンドウを閉じたらトレイへ格納</label
  ><label
    >foreground照合間隔（秒）<input
      type="number"
      min="2"
      max="30"
      bind:value={settings.reconciliation_seconds}
    /></label
  >
  <div class="hotkey-setting">
    <span class="setting-label">スクリーンショットキー</span>
    <div
      class:recording={recordingHotkey}
      class:invalid={Boolean(hotkeyError)}
      class="hotkey-recorder"
    >
      <kbd
        >{recordingHotkey
          ? '設定したいキーを押してください…'
          : settings.screenshot_hotkey || '未設定'}</kbd
      >
      <button type="button" disabled={checkingHotkey} onclick={startHotkeyRecording}>
        {settings.screenshot_hotkey ? '変更' : 'キーを設定'}
      </button>
      {#if settings.screenshot_hotkey}<button type="button" onclick={clearHotkey}>解除</button>{/if}
    </div>
    <p class="hint">
      記録中は任意のキーまたはキーの組み合わせを押してください。Escでキャンセルします。
    </p>
    {#if hotkeyStatus}<p class="hotkey-status">{hotkeyStatus}</p>{/if}
    {#if hotkeyError}<p class="error">{hotkeyError}</p>{/if}
  </div>
  <div class="update-setting">
    <div>
      <span class="setting-label">アプリの更新</span>
      <p class="hint">現在のバージョン: {currentVersion || '確認中…'}</p>
      {#if settings.skipped_update_version}
        <p class="hint">
          バージョン {settings.skipped_update_version} の自動通知はスキップ中です。
        </p>
      {/if}
    </div>
    <div class="update-setting-actions">
      <button type="button" disabled={checkingUpdate || installingUpdate} onclick={checkForUpdate}>
        {checkingUpdate ? '確認中…' : '更新を確認'}
      </button>
      {#if availableUpdate || previewUpdateAvailable}
        <button
          type="button"
          class="primary"
          disabled={installingUpdate}
          onclick={installAvailableUpdate}>{installingUpdate ? '更新中…' : '更新して再起動'}</button
        >
      {/if}
      {#if updateStatus}<p class="update-check-status">{updateStatus}</p>{/if}
    </div>
    <label class="check update-auto-check">
      <input type="checkbox" bind:checked={settings.auto_check_updates} />
      起動時に更新を自動確認して通知する
    </label>
    {#if installingUpdate}
      <p>{updateProgressText()}</p>
      {#if updateContentLength}
        <progress value={updateDownloaded} max={updateContentLength}></progress>
      {/if}
    {/if}
    {#if updateError}<p class="error">{updateError}</p>{/if}
  </div>
  <button class="primary" disabled={recordingHotkey || checkingHotkey} onclick={save}>保存</button
  >{#if message}<p>
      {message}
    </p>{/if}{#if error}<p class="error">{error}</p>{/if}
</section>
