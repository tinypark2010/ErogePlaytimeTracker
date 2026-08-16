<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import { api } from '../lib/api';
  import { duration, local, inputTime, utc, imageSrc } from '../lib/time';
  import type { FocusInterval, GameDetail, GameTimestamp, Session } from '../lib/types';
  type ConfirmAction = 'session' | 'all-sessions' | 'game';
  export let gameId: number;
  export let onback: () => void;
  let game: GameDetail | null = null,
    sessions: Session[] = [],
    timestamps: GameTimestamp[] = [],
    selected: Session | null = null,
    intervals: FocusInterval[] = [],
    error = '',
    newPath = '',
    manualStart = '',
    manualEnd = '',
    timestampName = '',
    nowMs = Date.now(),
    refreshingMeta = false,
    creatingTimestamp = false,
    toast = '',
    toastError = false,
    toastTimer: number | undefined,
    editingGame = false,
    editTitle = '',
    editBrand = '',
    editSourceUrl = '';
  let confirmAction: ConfirmAction | null = null;
  const confirmTitle = () =>
    confirmAction === 'session'
      ? 'セッションの削除'
      : confirmAction === 'all-sessions'
        ? 'すべてのセッションを削除'
        : 'ゲームの削除';
  const confirmMessage = () =>
    confirmAction === 'session'
      ? 'このセッションを本当に削除しますか？'
      : confirmAction === 'all-sessions'
        ? `${sessions.length}件のセッションと最前面記録をすべて削除します。元に戻せません。`
        : 'ゲームとすべての履歴を削除します。元に戻せません。';
  async function load() {
    try {
      [game, sessions, timestamps] = await Promise.all([
        api.getGame(gameId),
        api.sessions(gameId),
        api.timestamps(gameId),
      ]);
    } catch (e) {
      error = String(e);
    }
  }
  load();
  onMount(() => {
    let unlisten = () => {};
    const refreshIntervals = () => {
      load();
      if (selected) api.intervals(selected.id).then((value) => (intervals = value));
    };
    listen('tracking-status', refreshIntervals).then((fn) => (unlisten = fn));
    const timer = setInterval(() => {
      nowMs = Date.now();
      load();
    }, 1000);
    return () => {
      unlisten();
      clearInterval(timer);
      if (toastTimer) clearTimeout(toastTimer);
    };
  });
  async function select(s: Session) {
    selected = s;
    intervals = await api.intervals(s.id);
  }
  async function addExe() {
    if (newPath) {
      await api.addExecutable(gameId, newPath);
      newPath = '';
      await load();
    }
  }
  async function selectExe() {
    const selected = await open({
      title: 'ゲームの実行ファイルを選択',
      multiple: false,
      directory: false,
      filters: [
        { name: 'ゲーム実行ファイル', extensions: ['exe', 'bin'] },
        { name: 'すべてのファイル', extensions: ['*'] },
      ],
    });
    if (selected) newPath = selected;
  }
  async function removeExe(id: number) {
    await api.removeExecutable(id);
    await load();
  }
  async function addManual() {
    try {
      await api.manualSession(gameId, utc(manualStart), utc(manualEnd));
      manualStart = manualEnd = '';
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  async function saveSession() {
    if (!selected) return;
    try {
      await api.updateSession(
        selected.id,
        utc(selected.launched_at),
        selected.exited_at ? utc(selected.exited_at) : null,
      );
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  function removeSession() {
    if (selected) confirmAction = 'session';
  }
  function removeAllSessions() {
    if (sessions.length) confirmAction = 'all-sessions';
  }
  async function confirmDelete() {
    const action = confirmAction;
    confirmAction = null;
    try {
      if (action === 'session' && selected) {
        await api.deleteSession(selected.id);
        selected = null;
        intervals = [];
        await load();
      } else if (action === 'all-sessions') {
        await api.deleteAllSessions(gameId);
        selected = null;
        intervals = [];
        await load();
      } else if (action === 'game') {
        await api.deleteGame(gameId);
        onback();
      }
    } catch (e) {
      error = `削除できませんでした: ${String(e)}`;
    }
  }
  async function saveInterval(i: FocusInterval) {
    try {
      await api.updateInterval(i.id, utc(i.started_at), utc(i.ended_at!));
      intervals = await api.intervals(i.play_session_id);
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  async function addInterval() {
    if (!selected) return;
    try {
      const s = prompt('開始日時 (例 2026-08-16T20:00)');
      const e = prompt('終了日時');
      if (s && e) {
        await api.createInterval(selected.id, utc(s), utc(e));
        intervals = await api.intervals(selected.id);
        await load();
      }
    } catch (e) {
      error = String(e);
    }
  }
  function showToast(message: string, isError = false) {
    toast = message;
    toastError = isError;
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = window.setTimeout(() => (toast = ''), 4000);
  }
  function beginGameEdit() {
    if (!game) return;
    editTitle = game.title;
    editBrand = game.brand ?? '';
    editSourceUrl = game.source_url ?? '';
    editingGame = true;
  }
  async function saveGameInfo() {
    if (!game || !editTitle.trim()) return showToast('タイトルを入力してください', true);
    try {
      await api.updateGame(game.id, {
        title: editTitle.trim(),
        brand: editBrand.trim() || undefined,
        release_date: game.release_date ?? undefined,
        source_url: editSourceUrl.trim() || undefined,
      });
      editingGame = false;
      await load();
      showToast('ゲーム情報を保存しました');
    } catch (e) {
      showToast(`ゲーム情報を保存できませんでした: ${String(e)}`, true);
    }
  }
  async function openSourceUrl() {
    if (!game?.source_url) return;
    try {
      await api.openExternalUrl(game.source_url);
    } catch (e) {
      showToast(String(e), true);
    }
  }
  async function refreshMeta() {
    if (refreshingMeta) return;
    refreshingMeta = true;
    error = '';
    try {
      await api.refreshMetadata(gameId);
      await load();
      showToast('ErogameScapeからゲーム情報を更新しました');
    } catch (e) {
      error = String(e);
      showToast(`情報を更新できませんでした: ${String(e)}`, true);
    } finally {
      refreshingMeta = false;
    }
  }
  function removeGame() {
    confirmAction = 'game';
  }
  async function launch() {
    try {
      await api.launchGame(gameId);
    } catch (e) {
      error = String(e);
    }
  }
  async function createTimestamp() {
    const name = timestampName.trim();
    if (!name) return;
    creatingTimestamp = true;
    try {
      await api.createTimestamp(gameId, name);
      timestampName = '';
      await load();
      showToast(`「${name}」を記録しました`);
    } catch (e) {
      showToast(`記録できませんでした: ${String(e)}`, true);
    } finally {
      creatingTimestamp = false;
    }
  }
  async function deleteTimestamp(id: number) {
    try {
      await api.deleteTimestamp(id);
      await load();
      showToast('プレイ記録ポイントを削除しました');
    } catch (e) {
      showToast(`削除できませんでした: ${String(e)}`, true);
    }
  }
</script>

<button class="back-button" onclick={onback}>← ライブラリに戻る</button
>{#if game}{#if game.thumbnail_path}<div
      class="detail-backdrop"
      style:background-image={`url("${imageSrc(game.thumbnail_path)}")`}
    ></div>{/if}
  <section class="detail">
    <div class="actions detail-actions">
      <button class="metadata-refresh" onclick={refreshMeta} disabled={refreshingMeta}
        >{#if refreshingMeta}<span class="spinner" aria-hidden="true"
          ></span>取得中…{:else}ErogameScapeから情報を更新{/if}</button
      ><button class="danger" onclick={removeGame}>ゲームを削除</button>
    </div>
    <div class="hero">
      <div class="detail-image">
        {#if game.thumbnail_path}<img src={imageSrc(game.thumbnail_path)} alt="" />{:else}<div
            class="placeholder"
          >
            NO IMAGE
          </div>{/if}<button class="launch-overlay" onclick={launch}>▶ 起動</button>
      </div>
      <div>
        <h1>{game.title}</h1>
        <p>{game.brand ?? 'ブランド未設定'} ・ {game.release_date ?? '発売日未設定'}</p>
        <small>プレイ時間</small>
        <h2>{duration(game.total_playtime_seconds)}</h2>
        <p>{game.session_count} セッション</p>
      </div>
    </div>
    <section class="panel game-info">
      <div class="panel-heading">
        <h2>ゲーム情報</h2>
        {#if !editingGame}<button onclick={beginGameEdit}>編集</button>{/if}
      </div>
      {#if editingGame}<div class="game-info-form">
          <label>タイトル<input bind:value={editTitle} /></label><label
            >ブランド<input bind:value={editBrand} placeholder="未設定" /></label
          ><label
            >ErogameScape URL<input
              type="url"
              bind:value={editSourceUrl}
              placeholder="https://erogamescape.dyndns.org/…"
            /></label
          >
          <div class="actions">
            <button class="primary" onclick={saveGameInfo}>保存</button><button
              onclick={() => (editingGame = false)}>キャンセル</button
            >
          </div>
        </div>{:else}<dl>
          <div>
            <dt>タイトル</dt>
            <dd>{game.title}</dd>
          </div>
          <div>
            <dt>ブランド</dt>
            <dd>{game.brand ?? '未設定'}</dd>
          </div>
          <div>
            <dt>ErogameScape URL</dt>
            <dd>
              {#if game.source_url}<button
                  class="external-link"
                  title="既定のブラウザで開く"
                  onclick={openSourceUrl}>{game.source_url}<span aria-hidden="true">↗</span></button
                >{:else}未設定{/if}
            </dd>
          </div>
        </dl>{/if}
    </section>
    <section class="panel timestamp-panel">
      <div class="panel-heading"><h2>プレイ記録ポイント</h2></div>
      <p class="hint">
        ルートクリアなどの節目を記録すると、到達までにかかったプレイ時間を確認できます。
      </p>
      <div class="row timestamp-create">
        <input
          maxlength="100"
          bind:value={timestampName}
          placeholder="例: ○○ルートクリア"
          onkeydown={(e) => {
            if (e.key === 'Enter') createTimestamp();
          }}
        /><button
          class="primary"
          disabled={creatingTimestamp || !timestampName.trim()}
          onclick={createTimestamp}>{creatingTimestamp ? '記録中…' : '現在時刻で記録'}</button
        >
      </div>
      {#if !timestamps.length}<p class="timestamp-empty">まだ記録ポイントはありません。</p>{/if}
      <div class="timestamp-list">
        {#each timestamps as point, index}<article class="timestamp-item">
            <div class="timestamp-marker" aria-hidden="true"></div>
            <div class="timestamp-content">
              <h3>{point.name}</h3>
              <small>{local(point.marked_at)}</small>
              <div class="timestamp-times">
                <span
                  ><small>累計プレイ時間</small><strong>{duration(point.playtime_seconds)}</strong
                  ></span
                ><span
                  ><small>{index === 0 ? 'ゲーム開始から' : '前のポイントから'}</small><strong
                    >{duration(point.since_previous_seconds)}</strong
                  ></span
                >
              </div>
            </div>
            <button onclick={() => deleteTimestamp(point.id)}>削除</button>
          </article>{/each}
      </div>
    </section>
    <section class="panel">
      <h2>実行ファイル</h2>
      {#each game.executables as x}<div class="listrow">
          <code>{x.path}</code><button onclick={() => removeExe(x.id)}>削除</button>
        </div>{/each}
      <div class="row">
        <input bind:value={newPath} placeholder="exeを選択してください" readonly /><button
          type="button"
          onclick={selectExe}>参照…</button
        ><button onclick={addExe}>追加</button>
      </div>
    </section>
    <section class="panel">
      <h2>手動セッション追加</h2>
      <div class="row">
        <input type="datetime-local" step="1" bind:value={manualStart} /><span>〜</span><input
          type="datetime-local"
          step="1"
          bind:value={manualEnd}
        /><button onclick={addManual}>追加</button>
      </div>
    </section>
    <section class="panel">
      <div class="panel-heading">
        <h2>Session History</h2>
        <button class="danger" disabled={!sessions.length} onclick={removeAllSessions}
          >すべてのセッションを削除</button
        >
      </div>
      {#each sessions as s}<button
          class:selected={selected?.id === s.id}
          class="session"
          onclick={() => select(s)}
          ><span
            >{local(s.launched_at)} → {local(s.exited_at)}{s.needs_review ? ' ・ 要確認' : ''}</span
          ><strong>{duration(s.foreground_seconds)}</strong></button
        >{/each}
    </section>
  </section>{/if}
{#if selected}<div class="modal">
    <section class="panel editor">
      <button class="close" onclick={() => (selected = null)}>×</button>
      <h2>Session #{selected.id}</h2>
      <label
        >開始<input
          type="datetime-local"
          step="1"
          value={inputTime(selected.launched_at)}
          onchange={(e) => (selected!.launched_at = (e.currentTarget as HTMLInputElement).value)}
        /></label
      ><label
        >終了<input
          type="datetime-local"
          step="1"
          value={inputTime(selected.exited_at)}
          onchange={(e) => (selected!.exited_at = (e.currentTarget as HTMLInputElement).value)}
        /></label
      >
      <div class="actions">
        <button class="primary" onclick={saveSession}>保存</button><button
          class="danger"
          onclick={removeSession}>削除</button
        >
      </div>
      <h3>最前面でプレイしていた時間</h3>
      <p class="hint">ゲームが最前面ではなかった時間はプレイ時間から除外されます。</p>
      {#each intervals as i}<div class:live-interval={!i.ended_at} class="interval">
          {#if i.ended_at}<input
              type="datetime-local"
              step="1"
              value={inputTime(i.started_at)}
              onchange={(e) => (i.started_at = (e.currentTarget as HTMLInputElement).value)}
            /><span>〜</span><input
              type="datetime-local"
              step="1"
              value={inputTime(i.ended_at)}
              onchange={(e) => (i.ended_at = (e.currentTarget as HTMLInputElement).value)}
            /><button onclick={() => saveInterval(i)}>保存</button><button
              onclick={async () => {
                await api.deleteInterval(i.id);
                intervals = await api.intervals(selected!.id);
                await load();
              }}>削除</button
            >{:else}<span
              >{local(i.started_at)} ～ 最前面で記録中（{duration(
                Math.max(0, Math.floor((nowMs - new Date(i.started_at).getTime()) / 1000)),
              )}）</span
            >{/if}
        </div>{/each}<button onclick={addInterval}>最前面区間を追加</button>{#if error}<p
          class="error"
        >
          {error}
        </p>{/if}
    </section>
  </div>{/if}{#if error && !selected}<p class="error">{error}</p>{/if}
{#if confirmAction}<div class="modal confirm-modal">
    <div
      class="panel confirm-box"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-message"
    >
      <div class="confirm-icon">!</div>
      <h2 id="confirm-title">{confirmTitle()}</h2>
      <p id="confirm-message">{confirmMessage()}</p>
      <div class="confirm-actions">
        <button onclick={() => (confirmAction = null)}>キャンセル</button><button
          class="danger"
          onclick={confirmDelete}>削除する</button
        >
      </div>
    </div>
  </div>{/if}
{#if toast}<div class:error-toast={toastError} class="toast" role="status">
    <span>{toastError ? '!' : '✓'}</span>
    <p>{toast}</p>
    <button aria-label="通知を閉じる" onclick={() => (toast = '')}>×</button>
  </div>{/if}
