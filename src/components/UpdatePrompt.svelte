<script lang="ts">
  import { onMount } from 'svelte';
  import { relaunch } from '@tauri-apps/plugin-process';
  import { check } from '@tauri-apps/plugin-updater';

  let update: Awaited<ReturnType<typeof check>> = null;
  let dismissed = false;
  let installing = false;
  let downloaded = 0;
  let contentLength: number | undefined;
  let error = '';

  onMount(() => {
    check()
      .then((result) => (update = result))
      .catch((checkError) => console.warn('更新の確認に失敗しました', checkError));
  });

  async function install() {
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

{#if update && !dismissed}
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
        バージョン {update.version} を利用できます。更新するとアプリが再起動します。
      </p>
      {#if update.body}
        <details>
          <summary>リリースノート</summary>
          <div class="update-notes">{update.body}</div>
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
