<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import { duration, lastPlayed, imageSrc, playStatusLabel, playStatusOptions } from '../lib/time';
  import type { GameSummary, SortKey } from '../lib/types';
  type ViewMode = 'grid' | 'list';
  export let refresh = 0;
  export let openGame: (id: number) => void;
  let games: GameSummary[] = [],
    search = '',
    brand = '',
    playStatus = '',
    sort: SortKey = 'last_played',
    descending = true,
    error = '',
    viewMode: ViewMode = 'grid';
  let loadedKey = '';
  $: totalPlaytimeSeconds = games.reduce((total, game) => total + game.total_playtime_seconds, 0);
  async function load() {
    try {
      games = await api.listGames(search, brand, playStatus, sort, descending);
    } catch (e) {
      error = String(e);
    }
  }
  $: {
    const key = `${refresh}|${search}|${brand}|${playStatus}|${sort}|${descending}`;
    if (key !== loadedKey) {
      loadedKey = key;
      load();
    }
  }
  let brands: string[] = [];
  onMount(() => {
    const saved = localStorage.getItem('library-view-mode');
    if (saved === 'grid' || saved === 'list') viewMode = saved;
    api
      .listBrands()
      .then((value) => (brands = value))
      .catch((e) => (error = String(e)));
    const timer = setInterval(load, 1000);
    return () => clearInterval(timer);
  });
  function setViewMode(mode: ViewMode) {
    viewMode = mode;
    localStorage.setItem('library-view-mode', mode);
  }
  async function launch(gameId: number) {
    try {
      await api.launchGame(gameId);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="toolbar">
  <label class="search-control"
    ><span>タイトル検索</span><input
      placeholder="ゲームタイトルを入力"
      bind:value={search}
    /></label
  ><label
    ><span>ブランド</span><select bind:value={brand}
      ><option value="">すべてのブランド</option>{#each brands as b}<option>{b}</option
        >{/each}</select
    ></label
  ><label
    ><span>プレイ状況</span><select bind:value={playStatus}
      ><option value="">すべての状況</option>{#each playStatusOptions as option}<option
          value={option.value}>{option.label}</option
        >{/each}</select
    ></label
  ><label
    ><span>並び順</span><select bind:value={sort}
      ><option value="last_played">最終プレイ</option><option value="total_playtime"
        >プレイ時間</option
      ><option value="title">タイトル</option><option value="brand">ブランド</option><option
        value="release_date">発売日</option
      ><option value="created_at">登録日</option><option value="session_count">セッション数</option
      ></select
    ></label
  ><button onclick={() => (descending = !descending)}>{descending ? '降順' : '昇順'}</button>
  <div class="view-control">
    <span>表示形式</span>
    <div class="view-switch" aria-label="ライブラリの表示形式">
      <button
        class:active={viewMode === 'grid'}
        aria-pressed={viewMode === 'grid'}
        title="グリッド表示"
        onclick={() => setViewMode('grid')}>▦<span>グリッド</span></button
      ><button
        class:active={viewMode === 'list'}
        aria-pressed={viewMode === 'list'}
        title="リスト表示"
        onclick={() => setViewMode('list')}>☷<span>リスト</span></button
      >
    </div>
  </div>
</section>
<section class="library-summary" aria-live="polite">
  <span>表示中 <strong>{games.length}</strong> 本</span>
  <span>合計プレイ時間 <strong>{duration(totalPlaytimeSeconds)}</strong></span>
</section>
{#if error}<p class="error">{error}</p>{/if}{#if !games.length}<div class="empty">
    ゲームがありません。「ゲーム追加」から登録してください。
  </div>{/if}
{#if viewMode === 'grid'}
  <div class="grid">
    {#each games as g}<article class="card">
        <button
          class="card-main"
          onclick={() => openGame(g.id)}
          aria-label={`${g.title}の詳細を開く`}
          ><div class="card-image">
            {#if g.thumbnail_path}<img src={imageSrc(g.thumbnail_path)} alt="" />{:else}<div
                class="placeholder"
              >
                NO IMAGE
              </div>{/if}
          </div>
          <div class="card-info">
            <h2>{g.title}</h2>
            <p>{g.brand ?? 'ブランド未設定'}</p>
            <span class="play-status status-{g.play_status}">{playStatusLabel(g.play_status)}</span>
            <strong>{duration(g.total_playtime_seconds)}</strong><small
              >最終: {lastPlayed(g.last_played)} ・ {g.session_count}回</small
            >
          </div></button
        ><button class="launch-overlay" onclick={() => launch(g.id)} aria-label={`${g.title}を起動`}
          >▶ 起動</button
        >
      </article>{/each}
  </div>
{:else}
  <div class="library-list">
    {#each games as g}<article class="library-row">
        <button
          class="library-row-main"
          onclick={() => openGame(g.id)}
          aria-label={`${g.title}の詳細を開く`}
          ><div class="library-row-image">
            {#if g.thumbnail_path}<img src={imageSrc(g.thumbnail_path)} alt="" />{:else}<div
                class="placeholder"
              >
                NO IMAGE
              </div>{/if}
          </div>
          <div class="library-row-info">
            <h2>{g.title}</h2>
            <p>
              {g.brand ?? 'ブランド未設定'}{#if g.release_date}<span>
                  ・ {g.release_date}</span
                >{/if}
            </p>
            <span class="play-status status-{g.play_status}">{playStatusLabel(g.play_status)}</span>
          </div>
          <div class="library-row-stats">
            <strong>{duration(g.total_playtime_seconds)}</strong><small
              >最終: {lastPlayed(g.last_played)}</small
            ><small>{g.session_count} セッション</small>
          </div></button
        ><button
          class="library-row-launch"
          onclick={() => launch(g.id)}
          aria-label={`${g.title}を起動`}>▶ 起動</button
        >
      </article>{/each}
  </div>
{/if}
