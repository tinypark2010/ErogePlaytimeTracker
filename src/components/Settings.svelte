<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { getVersion } from '@tauri-apps/api/app';
  import { open, save as saveFile } from '@tauri-apps/plugin-dialog';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check } from '@tauri-apps/plugin-updater';
  import { api } from '../lib/api';
  import { userErrorMessage } from '../lib/errors';
  import type { BackupImportPreview, Settings, Theme } from '../lib/types';
  export let ontheme: (theme: Theme) => void = () => {};
  export let ondirty: (dirty: boolean) => void = () => {};
  export let onsaved: (settings: Settings) => void = () => {};
  export let trackingActive = false;
  let settings: Settings = {
      autostart: false,
      auto_check_updates: true,
      skipped_update_version: null,
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
    updateContentLength: number | undefined,
    backupBusy = false,
    backupMessage = '',
    backupError = '',
    importConfirmed = false,
    destroyed = false;
  let importPreview: BackupImportPreview | null = null;
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
          hotkeyError = userErrorMessage(e, 'スクリーンショットキーを確認できませんでした。');
        }
      }
    } catch (e) {
      error = userErrorMessage(e, '設定を読み込めませんでした。');
    }
  });
  onDestroy(() => {
    destroyed = true;
    if (recordingHotkey) api.resumeScreenshotHotkey().catch(() => {});
    if (importPreview && !importConfirmed) {
      api.cancelBackupImport(importPreview.import_id).catch(() => {});
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
      settings = await api.settings();
      savedSettings = { ...settings };
      onsaved(settings);
      ondirty(false);
      message = '保存しました';
      error = '';
      hotkeyError = '';
    } catch (e) {
      const saveError = userErrorMessage(e, '設定を保存できませんでした。');
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
      hotkeyError = userErrorMessage(e, 'スクリーンショットキーを確認できませんでした。');
    } finally {
      await api
        .resumeScreenshotHotkey()
        .catch(
          (e) =>
            (hotkeyError = userErrorMessage(e, 'スクリーンショットキーを再登録できませんでした。')),
        );
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
      hotkeyError = userErrorMessage(e, 'スクリーンショットキーを解除できませんでした。');
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
      updateError = userErrorMessage(e, '更新を確認できませんでした。');
    } finally {
      checkingUpdate = false;
    }
  }
  async function installAvailableUpdate() {
    let gameIsRunning = trackingActive;
    try {
      gameIsRunning ||= (await api.status()).games.length > 0;
    } catch (e) {
      updateError = userErrorMessage(e, 'ゲームの起動状態を確認できませんでした。');
      return;
    }
    if (gameIsRunning) {
      updateError = '';
      updateStatus = '起動中のゲームを終了してから更新してください。';
      return;
    }
    if (previewUpdateAvailable) {
      updateError = '';
      updateStatus = 'モック表示のため、実際の更新は行いません。';
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
      updateError = userErrorMessage(e, '更新できませんでした。もう一度お試しください。');
      installingUpdate = false;
    }
  }
  function updateProgressText() {
    if (!updateContentLength) return '更新をダウンロードしています…';
    const percent = Math.min(100, Math.round((updateDownloaded / updateContentLength) * 100));
    return `更新をダウンロードしています… ${percent}%`;
  }
  function backupFilename() {
    const date = new Date();
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `eroge-playtime-tracker-${year}-${month}-${day}.eptbackup`;
  }
  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    const units = ['KB', 'MB', 'GB', 'TB'];
    let value = bytes / 1024;
    let unit = units[0];
    for (let index = 1; index < units.length && value >= 1024; index++) {
      value /= 1024;
      unit = units[index];
    }
    return `${value.toFixed(value >= 10 ? 1 : 2)} ${unit}`;
  }
  async function exportData() {
    if (backupBusy || trackingActive) return;
    let destination: string | null;
    try {
      destination = await saveFile({
        defaultPath: backupFilename(),
        filters: [{ name: 'Eroge Playtime Tracker バックアップ', extensions: ['eptbackup'] }],
      });
    } catch (e) {
      backupError = userErrorMessage(e, '保存先を選択できませんでした。');
      return;
    }
    if (!destination) return;
    backupBusy = true;
    backupMessage = 'バックアップを作成しています…';
    backupError = '';
    try {
      const result = await api.exportBackup(destination);
      if (destroyed) return;
      backupMessage = `バックアップを作成しました（${formatBytes(result.file_size)}）。`;
      if (result.missing_media_count > 0) {
        backupMessage += ` 見つからなかった画像 ${result.missing_media_count} 件は含まれていません。`;
      }
    } catch (e) {
      backupMessage = '';
      backupError = userErrorMessage(e, 'バックアップを作成できませんでした。');
    } finally {
      backupBusy = false;
    }
  }
  async function chooseImport() {
    if (backupBusy || trackingActive) return;
    let source: string | string[] | null;
    try {
      source = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'Eroge Playtime Tracker バックアップ', extensions: ['eptbackup'] }],
      });
    } catch (e) {
      backupError = userErrorMessage(e, 'バックアップファイルを選択できませんでした。');
      return;
    }
    if (!source || Array.isArray(source)) return;
    backupBusy = true;
    backupMessage = 'バックアップを検証しています…';
    backupError = '';
    try {
      const preview = await api.prepareBackupImport(source);
      if (destroyed) {
        await api.cancelBackupImport(preview.import_id).catch(() => {});
        return;
      }
      importPreview = preview;
      backupMessage = '';
    } catch (e) {
      backupMessage = '';
      backupError = userErrorMessage(e, 'バックアップを読み込めませんでした。');
    } finally {
      backupBusy = false;
    }
  }
  async function cancelImport() {
    if (!importPreview || backupBusy) return;
    const preview = importPreview;
    importPreview = null;
    try {
      await api.cancelBackupImport(preview.import_id);
    } catch (e) {
      backupError = userErrorMessage(e, 'インポートの準備データを削除できませんでした。');
    }
  }
  async function confirmImport() {
    if (!importPreview || backupBusy || trackingActive) return;
    backupBusy = true;
    backupError = '';
    try {
      await api.confirmBackupImport(importPreview.import_id);
      importConfirmed = true;
      backupMessage = 'インポートを適用するため再起動しています…';
      await relaunch();
    } catch (e) {
      importConfirmed = false;
      await api.cancelBackupImport(importPreview.import_id).catch(() => {});
      backupMessage = '';
      backupError = userErrorMessage(e, 'インポートを開始できませんでした。');
      importPreview = null;
      backupBusy = false;
    }
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
      <div class="update-setting-buttons">
        <button
          type="button"
          disabled={checkingUpdate || installingUpdate}
          onclick={checkForUpdate}
        >
          {checkingUpdate ? '確認中…' : '更新を確認'}
        </button>
        {#if availableUpdate || previewUpdateAvailable}
          <button
            type="button"
            class="primary"
            disabled={installingUpdate || trackingActive}
            onclick={installAvailableUpdate}
            >{installingUpdate ? '更新中…' : '更新して再起動'}</button
          >
        {/if}
      </div>
      <div class="update-setting-feedback" aria-live="polite">
        {#if updateStatus}<p>{updateStatus}</p>{/if}
        {#if updateError}<p class="error">{updateError}</p>{/if}
        {#if (availableUpdate || previewUpdateAvailable) && trackingActive}
          <p>起動中のゲームを終了すると更新できます。</p>
        {/if}
        {#if installingUpdate}
          <p>{updateProgressText()}</p>
          {#if updateContentLength}
            <progress value={updateDownloaded} max={updateContentLength}></progress>
          {/if}
        {/if}
      </div>
    </div>
    <label class="check update-auto-check">
      <input type="checkbox" bind:checked={settings.auto_check_updates} />
      起動時に更新を自動確認して通知する
    </label>
  </div>
  <div class="backup-setting">
    <div>
      <span class="setting-label">データの移行</span>
      <p class="hint">ゲーム、プレイ履歴、設定、画像を1つのファイルに保存します。</p>
    </div>
    <div class="backup-setting-actions">
      <button type="button" disabled={backupBusy || trackingActive} onclick={exportData}>
        {backupBusy && !importPreview ? '処理中…' : 'エクスポート'}
      </button>
      <button type="button" disabled={backupBusy || trackingActive} onclick={chooseImport}>
        インポート
      </button>
    </div>
    {#if trackingActive}<p class="hint">起動中のゲームを終了すると操作できます。</p>{/if}
    <div class="backup-setting-feedback" aria-live="polite">
      {#if backupMessage}<p>{backupMessage}</p>{/if}
      {#if backupError}<p class="error">{backupError}</p>{/if}
    </div>
  </div>
  <button class="primary" disabled={recordingHotkey || checkingHotkey} onclick={save}>保存</button
  >{#if message}<p>
      {message}
    </p>{/if}{#if error}<p class="error">{error}</p>{/if}
</section>

{#if importPreview}<div class="modal backup-import-modal">
    <div
      class="panel backup-import-dialog"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="backup-import-title"
      aria-describedby="backup-import-message"
    >
      <div class="confirm-icon">!</div>
      <h2 id="backup-import-title">現在のデータを置き換えますか？</h2>
      <p id="backup-import-message">
        インポートは追加・統合ではありません。現在のデータをバックアップ内容で置き換えて再起動します。置換前のデータは自動で退避します。
      </p>
      <dl class="backup-import-summary">
        <div>
          <dt></dt>
          <dd>現在</dd>
          <dd>バックアップ</dd>
        </div>
        <div>
          <dt>ゲーム</dt>
          <dd>{importPreview.current_summary.game_count} 件</dd>
          <dd>{importPreview.summary.game_count} 件</dd>
        </div>
        <div>
          <dt>プレイ履歴</dt>
          <dd>{importPreview.current_summary.session_count} 件</dd>
          <dd>{importPreview.summary.session_count} 件</dd>
        </div>
        <div>
          <dt>タイムスタンプ</dt>
          <dd>{importPreview.current_summary.timestamp_count} 件</dd>
          <dd>{importPreview.summary.timestamp_count} 件</dd>
        </div>
        <div>
          <dt>スクリーンショット</dt>
          <dd>{importPreview.current_summary.screenshot_count} 件</dd>
          <dd>{importPreview.summary.screenshot_count} 件</dd>
        </div>
        <div>
          <dt>サムネイル</dt>
          <dd>{importPreview.current_summary.thumbnail_count} 件</dd>
          <dd>{importPreview.summary.thumbnail_count} 件</dd>
        </div>
      </dl>
      <p class="hint">
        作成日時: {new Date(importPreview.exported_at).toLocaleString('ja-JP')} ／ アプリ {importPreview.app_version}
        ／ {formatBytes(importPreview.file_size)}
      </p>
      {#if importPreview.missing_executable_count > 0}<p class="backup-warning">
          登録済みの実行ファイルのうち {importPreview.missing_executable_count}
          件はこのPCで見つかりません。移行後にゲーム詳細から登録し直してください。
        </p>{/if}
      <div class="confirm-actions">
        <button disabled={backupBusy} onclick={cancelImport}>キャンセル</button>
        <button class="danger" disabled={backupBusy || trackingActive} onclick={confirmImport}>
          {backupBusy ? '準備中…' : '置き換えて再起動'}
        </button>
      </div>
    </div>
  </div>{/if}
