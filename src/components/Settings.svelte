<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Settings, Theme } from '../lib/types';
  export let ontheme: (theme: Theme) => void = () => {};
  let settings: Settings = {
      autostart: false,
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
    hotkeyStatus = '';
  onMount(async () => {
    try {
      settings = await api.settings();
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
  <button class="primary" disabled={recordingHotkey || checkingHotkey} onclick={save}>保存</button
  >{#if message}<p>
      {message}
    </p>{/if}{#if error}<p class="error">{error}</p>{/if}
</section>
