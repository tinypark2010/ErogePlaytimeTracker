<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Library from './components/Library.svelte';
  import GameDetail from './components/GameDetail.svelte';
  import AddGame from './components/AddGame.svelte';
  import Settings from './components/Settings.svelte';
  import UpdatePrompt from './components/UpdatePrompt.svelte';
  import { api } from './lib/api';
  import { trackingStatusGroups, trackingStatusText } from './lib/trackingStatus';
  import type { Theme, TrackingStatus } from './lib/types';
  type Page = 'library' | 'game' | 'add' | 'settings';
  let page: Page = 'library',
    gameId = 0,
    refresh = 0,
    status: TrackingStatus = { games: [] },
    savedTheme: Theme = 'dark',
    autoCheckUpdates = true,
    skippedUpdateVersion: string | null = null,
    settingsLoaded = false,
    settingsDirty = false,
    pendingPage: Page | null = null,
    pendingReload = false;
  function goTo(next: Page, shouldReload = false) {
    if (page === 'settings' && next !== 'settings' && settingsDirty) {
      pendingPage = next;
      pendingReload = shouldReload;
      return;
    }
    page = next;
    if (shouldReload) reload();
  }
  function discardSettingsAndLeave() {
    if (!pendingPage) return;
    applyTheme(savedTheme);
    settingsDirty = false;
    page = pendingPage;
    if (pendingReload) reload();
    pendingPage = null;
    pendingReload = false;
  }
  const openGame = (id: number) => {
    gameId = id;
    goTo('game');
  };
  const reload = () => refresh++;
  $: statusGroups = trackingStatusGroups(status);
  const applyTheme = (theme: Theme) => (document.documentElement.dataset.theme = theme);
  function updateStatus(next: TrackingStatus) {
    status = next;
  }
  onMount(() => {
    api.settings().then((v) => {
      savedTheme = v.theme;
      autoCheckUpdates = v.auto_check_updates;
      skippedUpdateVersion = v.skipped_update_version;
      applyTheme(v.theme);
      settingsLoaded = true;
    });
    api.status().then(updateStatus);
    const timer = setInterval(() => api.status().then(updateStatus), 3000);
    let off = () => {};
    listen<TrackingStatus>('tracking-status', (e) => updateStatus(e.payload)).then(
      (f) => (off = f),
    );
    return () => {
      clearInterval(timer);
      off();
    };
  });
</script>

<header>
  <button
    class="brand"
    onclick={() => {
      goTo('library', true);
    }}>Eroge Playtime Tracker</button
  >
  <nav>
    <button onclick={() => goTo('library')}>ライブラリ</button><button onclick={() => goTo('add')}
      >ゲーム追加</button
    ><button onclick={() => goTo('settings')}>設定</button>
  </nav>
  <div class="tracking-statuses">
    {#if statusGroups.length === 0}<span class="tracking-status idle">● 待機中</span>{/if}
    {#each statusGroups as group}<span
        class="tracking-status {group.phase}"
        title={group.games.map((game) => game.title).join('\n')}>● {trackingStatusText(group)}</span
      >{/each}
  </div>
</header>
<main>
  {#if page === 'library'}<Library {refresh} {openGame} />{:else if page === 'game'}<GameDetail
      {gameId}
      onback={() => {
        page = 'library';
        reload();
      }}
    />{:else if page === 'add'}<AddGame
      ondone={(id) => {
        openGame(id);
      }}
      oncancel={() => (page = 'library')}
    />{:else}<Settings
      trackingActive={status.games.length > 0}
      ontheme={applyTheme}
      ondirty={(dirty) => (settingsDirty = dirty)}
      onsaved={(settings) => {
        savedTheme = settings.theme;
        autoCheckUpdates = settings.auto_check_updates;
        skippedUpdateVersion = settings.skipped_update_version;
      }}
    />{/if}
</main>
{#if settingsLoaded}
  <UpdatePrompt
    autoCheck={autoCheckUpdates}
    skippedVersion={skippedUpdateVersion}
    trackingActive={status.games.length > 0}
    onskip={(version) => (skippedUpdateVersion = version)}
  />
{/if}
{#if pendingPage}<div class="modal confirm-modal">
    <div
      class="panel confirm-box"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="unsaved-settings-title"
      aria-describedby="unsaved-settings-message"
    >
      <div class="confirm-icon">!</div>
      <h2 id="unsaved-settings-title">設定の変更を破棄しますか？</h2>
      <p id="unsaved-settings-message">
        保存されていない変更があります。移動すると変更内容は破棄されます。
      </p>
      <div class="confirm-actions">
        <button onclick={() => (pendingPage = null)}>設定に戻る</button>
        <button class="danger" onclick={discardSettingsAndLeave}>破棄して移動</button>
      </div>
    </div>
  </div>{/if}
