<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import Library from './components/Library.svelte';
  import GameDetail from './components/GameDetail.svelte';
  import AddGame from './components/AddGame.svelte';
  import Settings from './components/Settings.svelte';
  import { api } from './lib/api';
  import { trackingStatusGroups, trackingStatusText } from './lib/trackingStatus';
  import type { Theme, TrackingStatus } from './lib/types';
  let page: 'library' | 'game' | 'add' | 'settings' = 'library',
    gameId = 0,
    refresh = 0,
    status: TrackingStatus = { games: [] };
  const openGame = (id: number) => {
    gameId = id;
    page = 'game';
  };
  const reload = () => refresh++;
  $: statusGroups = trackingStatusGroups(status);
  const applyTheme = (theme: Theme) => (document.documentElement.dataset.theme = theme);
  function updateStatus(next: TrackingStatus) {
    status = next;
  }
  onMount(() => {
    api.settings().then((v) => applyTheme(v.theme));
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
      page = 'library';
      reload();
    }}>Eroge Playtime Tracker</button
  >
  <nav>
    <button onclick={() => (page = 'library')}>ライブラリ</button><button
      onclick={() => (page = 'add')}>ゲーム追加</button
    ><button onclick={() => (page = 'settings')}>設定</button>
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
    />{:else}<Settings ontheme={applyTheme} />{/if}
</main>
