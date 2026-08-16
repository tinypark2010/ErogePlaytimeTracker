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
    },
    message = '',
    error = '';
  onMount(() =>
    api
      .settings()
      .then((v) => {
        settings = v;
        ontheme(v.theme);
      })
      .catch((e) => (error = String(e))),
  );
  function previewTheme() {
    ontheme(settings.theme);
  }
  async function save() {
    try {
      await api.updateSettings(settings);
      message = '保存しました';
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="panel form">
  <h1>設定</h1>
  <label
    >カラーテーマ<select bind:value={settings.theme} onchange={previewTheme}
      ><option value="dark">ダーク</option><option value="light">ライト</option><option value="pink"
        >ピンク</option
      ><option value="blue">ブルー</option></select
    ></label
  >
  <div class="theme-preview" aria-hidden="true">
    <span class="theme-dot theme-dark"></span><span class="theme-dot theme-light"></span><span
      class="theme-dot theme-pink"
    ></span><span class="theme-dot theme-blue"></span>
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
  ><button class="primary" onclick={save}>保存</button>{#if message}<p>
      {message}
    </p>{/if}{#if error}<p class="error">{error}</p>{/if}
</section>
