<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { UpdateCompletionNotice } from '../lib/types';

  export let notice: UpdateCompletionNotice;
  export let preview = false;
  export let onclose: (warning: string) => void;

  let dialog: HTMLDialogElement;
  let closing = false;
  $: releases = notice.releases.filter((release) => release.changes.length > 0);

  onMount(() => {
    const show = () => {
      if (!dialog.open && document.hasFocus() && document.visibilityState === 'visible') {
        dialog.showModal();
      }
    };
    // Wait for the user to view the app. Never show/activate the native window.
    window.addEventListener('focus', show);
    document.addEventListener('visibilitychange', show);
    show();
    return () => {
      window.removeEventListener('focus', show);
      document.removeEventListener('visibilitychange', show);
      dialog.close();
    };
  });

  async function close() {
    if (closing) return;
    closing = true;
    let warning = '';
    try {
      if (!preview) await api.acknowledgeUpdateCompletion(notice.version);
    } catch {
      warning = '通知の確認状態を保存できませんでした。次回起動時に再表示される場合があります。';
    }
    // A persistence failure must not lock the user out of the app.
    onclose(warning);
  }
</script>

<dialog
  bind:this={dialog}
  class="panel confirm-box update-box update-completion-dialog"
  aria-labelledby="update-completion-title"
  aria-describedby="update-completion-message"
  oncancel={(event) => {
    event.preventDefault();
    void close();
  }}
>
  <div class="confirm-icon update-icon">✓</div>
  <h2 id="update-completion-title">アップデートが完了しました</h2>
  <p id="update-completion-message">バージョン {notice.version} に更新しました。</p>
  {#if preview}<p>プレビュー表示です。実際の更新や既読状態の保存は行いません。</p>{/if}
  {#if releases.length > 0}
    <div class="update-release-notes">
      <h3>更新内容</h3>
      {#each releases as release (release.version)}
        {#if releases.length > 1 || release.version !== notice.version}
          <h4>バージョン {release.version}</h4>
        {/if}
        <ul>
          {#each release.changes as change}<li>{change}</li>{/each}
        </ul>
      {/each}
    </div>
  {/if}
  <div class="confirm-actions">
    <button type="button" class="primary" disabled={closing} onclick={close}>閉じる</button>
  </div>
</dialog>
