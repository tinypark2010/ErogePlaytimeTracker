<script lang="ts">
  import { onMount } from 'svelte';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check } from '@tauri-apps/plugin-updater';

  let update: Awaited<ReturnType<typeof check>> = null;
  let preview = false;
  let dismissed = false;
  let installing = false;
  let downloaded = 0;
  let contentLength: number | undefined;
  let error = '';

  $: version = update?.version ?? '0.1.6';
  $: notes =
    update?.body ??
    (preview
      ? '更新通知の表示確認用モックです。\n\n・アプリ内アップデートに対応しました\n・更新内容とダウンロード進捗を表示します'
      : undefined);

  onMount(() => {
    if (import.meta.env.DEV && import.meta.env.VITE_MOCK_UPDATE === 'true') {
      preview = true;
      return;
    }
    check()
      .then((result) => (update = result))
      .catch((checkError) => console.warn('更新の確認に失敗しました', checkError));
  });

  async function install() {
    if (preview) {
      error = 'モック表示のため、実際の更新は行いません。';
      return;
    }
    if (!update || installing) return;

    installing = true;
    error = '';
    downloaded = 0;
    contentLength = undefined;

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          contentLength = event.data.contentLength ?? undefined;
        } else if (event.event === 'Progress') {
          downloaded += event.data.chunkLength;
        }
      });
      await relaunch();
    } catch (installError) {
      error = String(installError);
      installing = false;
    }
  }

  function progressText() {
    if (!contentLength) return '更新をダウンロードしています…';
    return `更新をダウンロードしています… ${Math.min(100, Math.round((downloaded / contentLength) * 100))}%`;
  }
</script>

{#if (update || preview) && !dismissed}
  <div class="modal confirm-modal update-modal">
    <div
      class="panel confirm-box update-box"
      role="dialog"
      aria-modal="true"
      aria-labelledby="update-title"
      aria-describedby="update-message"
    >
      <div class="confirm-icon update-icon">↑</div>
      <h2 id="update-title">新しいバージョンがあります</h2>
      <p id="update-message">
        バージョン {version} を利用できます。更新するとアプリが再起動します。
      </p>
      {#if notes}
        <details>
          <summary>リリースノート</summary>
          <div class="update-notes">{notes}</div>
        </details>
      {/if}
      {#if installing}
        <p class="update-progress">{progressText()}</p>
        {#if contentLength}
          <progress value={downloaded} max={contentLength}></progress>
        {/if}
      {/if}
      {#if error}<p class="error">更新できませんでした: {error}</p>{/if}
      <div class="confirm-actions">
        <button type="button" disabled={installing} onclick={() => (dismissed = true)}>後で</button>
        <button type="button" class="primary" disabled={installing} onclick={install}>
          {installing ? '更新中…' : '更新して再起動'}
        </button>
      </div>
    </div>
  </div>
{/if}
