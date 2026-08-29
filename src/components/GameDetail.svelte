<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { open } from '@tauri-apps/plugin-dialog';
  import DateTimeSelect from './DateTimeSelect.svelte';
  import DeleteButton from './DeleteButton.svelte';
  import ThumbnailCropEditor from './ThumbnailCropEditor.svelte';
  import HistoryDataRow from './HistoryDataRow.svelte';
  import { api } from '../lib/api';
  import {
    validateBackgroundInterval,
    validateManualSession,
    validateRunningSessionEdit,
    validateSessionEdit,
  } from '../lib/historyValidation';
  import {
    duration,
    local,
    inputTime,
    utc,
    imageSrc,
    playStatusLabel,
    playStatusOptions,
  } from '../lib/time';
  import type {
    BackgroundInterval,
    GameDetail,
    GameTimestamp,
    GameScreenshot,
    PlayStatus,
    Session,
  } from '../lib/types';
  type SocialStyle = 'midnight' | 'rose' | 'ocean';
  type SocialLayout = 'auto' | 'left' | 'right' | 'top' | 'bottom' | 'custom';
  type SocialPreset = Exclude<SocialLayout, 'custom'>;
  type TimestampTimeMode = 'total' | 'difference';
  type SocialElement = 'image' | 'info';
  type ResizeHandle = 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';
  type SocialRect = { x: number; y: number; width: number; height: number };
  const defaultSocialFont = 'Yu Gothic UI';
  export let gameId: number;
  export let onback: () => void;
  let game: GameDetail | null = null,
    sessions: Session[] = [],
    timestamps: GameTimestamp[] = [],
    screenshots: GameScreenshot[] = [],
    selectedScreenshot: GameScreenshot | null = null,
    selected: Session | null = null,
    intervals: BackgroundInterval[] = [],
    pageError = '',
    newPath = '',
    manualSessionOpen = false,
    manualStart = '',
    manualEnd = '',
    manualSessionError = '',
    manualSessionSaving = false,
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
    editReleaseDate = '',
    editReleaseDateComplete = true,
    editReleaseDateError = '',
    editSourceUrl = '',
    sessionEditOpen = false,
    sessionStart = '',
    sessionEnd = '',
    sessionFormDirty = false,
    sessionFormError = '',
    sessionActionError = '',
    intervalListError = '',
    sessionSaving = false,
    newIntervalOpen = false,
    newIntervalStart = '',
    newIntervalEnd = '',
    newIntervalError = '',
    intervalCreating = false,
    editingIntervalId: number | null = null,
    intervalEditStart = '',
    intervalEditEnd = '',
    intervalEditError = '',
    savingIntervalId: number | null = null;
  let thumbnailEditorOpen = false,
    thumbnailDraftPath: string | null = null,
    thumbnailOriginalPath: string | null = null,
    thumbnailImporting = false,
    thumbnailCropBusy = false,
    thumbnailSaving = false,
    thumbnailError = '';
  let editingTimestampId: number | null = null,
    savingTimestampId: number | null = null,
    editingTimestampName = '';
  let socialOpen = false,
    socialCanvas: HTMLCanvasElement,
    socialScreenshotId = 0,
    socialTimestampIds: number[] = [],
    socialStyle: SocialStyle = 'midnight',
    socialLayout: SocialLayout = 'auto',
    socialFontSize = 85,
    socialFont = defaultSocialFont,
    socialFontSearch = '',
    socialLocalFonts: string[] = [],
    socialFontsLoading = false,
    socialImageRatio = 70,
    socialImageRect: SocialRect = { x: 1, y: 2, width: 98, height: 68 },
    socialInfoRect: SocialRect = { x: 1, y: 72, width: 98, height: 26 },
    socialSelectedElement: SocialElement | null = null,
    socialTimestampTimeMode: TimestampTimeMode = 'total',
    socialSaving = false,
    socialRenderToken = 0,
    socialRenderFrame = 0;
  const socialImageCache = new Map<string, Promise<HTMLImageElement>>();
  let socialFields = {
    title: true,
    brand: true,
    playtime: true,
    playStatus: true,
    sessionCount: false,
    lastPlayed: false,
    timestamp: false,
    capturedAt: false,
  };
  let socialImageEditorRect: SocialRect;
  let socialFilteredFonts: string[];
  const pageSizeOptions = [10, 25, 50];
  let screenshotPage = 1,
    screenshotPageSize = 10,
    sessionPage = 1,
    sessionPageSize = 10;
  let intervalBeingEdited: BackgroundInterval | null;
  $: screenshotPageCount = Math.max(1, Math.ceil(screenshots.length / screenshotPageSize));
  $: sessionPageCount = Math.max(1, Math.ceil(sessions.length / sessionPageSize));
  $: pagedScreenshots = screenshots.slice(
    (screenshotPage - 1) * screenshotPageSize,
    screenshotPage * screenshotPageSize,
  );
  $: pagedSessions = sessions.slice(
    (sessionPage - 1) * sessionPageSize,
    sessionPage * sessionPageSize,
  );
  $: intervalBeingEdited =
    editingIntervalId === null
      ? null
      : (intervals.find((interval) => interval.id === editingIntervalId) ?? null);
  $: socialImageEditorRect = fittedSocialImageRect(
    socialImageRect,
    socialScreenshotId,
    screenshots,
  );
  $: socialFilteredFonts = socialFontSearch.trim()
    ? socialLocalFonts.filter((font) =>
        font.toLocaleLowerCase().includes(socialFontSearch.trim().toLocaleLowerCase()),
      )
    : socialLocalFonts;
  $: socialRenderKey = JSON.stringify({
    socialOpen,
    socialScreenshotId,
    socialTimestampIds,
    socialStyle,
    socialLayout,
    socialFontSize,
    socialFont,
    socialImageRatio,
    socialImageRect,
    socialInfoRect,
    socialTimestampTimeMode,
    socialFields,
    game,
  });
  $: if (socialOpen && socialCanvas) {
    socialRenderKey;
    scheduleSocialPreview();
  }
  async function load() {
    try {
      const [nextGame, nextSessions, nextTimestamps, nextScreenshots] = await Promise.all([
        api.getGame(gameId),
        api.sessions(gameId),
        api.timestamps(gameId),
        api.screenshots(gameId),
      ]);
      game = nextGame;
      sessions = nextSessions;
      timestamps = nextTimestamps;
      screenshots = nextScreenshots;
      screenshotPage = Math.min(
        screenshotPage,
        Math.max(1, Math.ceil(nextScreenshots.length / screenshotPageSize)),
      );
      sessionPage = Math.min(
        sessionPage,
        Math.max(1, Math.ceil(nextSessions.length / sessionPageSize)),
      );
      if (selected) {
        selected = nextSessions.find((session) => session.id === selected?.id) ?? null;
        if (selected && sessionEditOpen && !sessionFormDirty) {
          sessionStart = inputTime(selected.launched_at);
          sessionEnd = inputTime(selected.exited_at);
        }
      }
    } catch {
      pageError = 'ゲーム情報を読み込めませんでした。しばらくしてからもう一度お試しください。';
    }
  }
  load();
  onMount(() => {
    let unlisten = () => {};
    const refreshIntervals = () => {
      load();
      const selectedId = selected?.id;
      if (selectedId) {
        api.intervals(selectedId).then((value) => {
          if (selected?.id === selectedId) intervals = value;
        });
      }
    };
    listen('tracking-status', refreshIntervals).then((fn) => (unlisten = fn));
    let unlistenScreenshot = () => {};
    listen<number>('screenshot-captured', (event) => {
      if (event.payload === gameId) {
        screenshotPage = 1;
        load();
        showToast('スクリーンショットを保存しました');
      }
    }).then((fn) => (unlistenScreenshot = fn));
    let unlistenScreenshotError = () => {};
    listen<string>('screenshot-error', (event) => showToast(event.payload, true)).then(
      (fn) => (unlistenScreenshotError = fn),
    );
    const timer = setInterval(() => {
      nowMs = Date.now();
      load();
    }, 1000);
    return () => {
      unlisten();
      unlistenScreenshot();
      unlistenScreenshotError();
      clearInterval(timer);
      if (toastTimer) clearTimeout(toastTimer);
    };
  });
  function resetSessionEditor() {
    sessionEditOpen = false;
    sessionStart = '';
    sessionEnd = '';
    sessionFormDirty = false;
    sessionFormError = '';
    sessionActionError = '';
    intervalListError = '';
    newIntervalOpen = false;
    newIntervalStart = '';
    newIntervalEnd = '';
    newIntervalError = '';
    editingIntervalId = null;
    intervalEditStart = '';
    intervalEditEnd = '';
    intervalEditError = '';
  }
  function closeSessionEditor() {
    selected = null;
    intervals = [];
    resetSessionEditor();
  }
  async function beginSessionDetail(s: Session) {
    selected = s;
    intervals = [];
    resetSessionEditor();
    try {
      const nextIntervals = await api.intervals(s.id);
      if (selected?.id === s.id) intervals = nextIntervals;
    } catch {
      if (selected?.id === s.id) {
        intervalListError =
          '除外区間を読み込めませんでした。モーダルを閉じて、もう一度お試しください。';
      }
    }
  }
  function beginSessionEdit() {
    if (!selected) return;
    sessionStart = inputTime(selected.launched_at);
    sessionEnd = inputTime(selected.exited_at);
    sessionFormDirty = false;
    sessionFormError = '';
    sessionEditOpen = true;
  }
  function cancelSessionEdit() {
    sessionEditOpen = false;
    sessionStart = '';
    sessionEnd = '';
    sessionFormDirty = false;
    sessionFormError = '';
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
    try {
      await api.removeExecutable(id);
      await load();
      showToast('実行ファイルの登録を削除しました');
    } catch (e) {
      showToast(`実行ファイルの登録を削除できませんでした: ${String(e)}`, true);
    }
  }
  function beginManualSession() {
    manualStart = '';
    manualEnd = '';
    manualSessionError = '';
    manualSessionOpen = true;
  }
  function cancelManualSession() {
    manualSessionOpen = false;
    manualStart = '';
    manualEnd = '';
    manualSessionError = '';
  }
  async function addManual() {
    manualSessionError = validateManualSession(manualStart, manualEnd);
    if (manualSessionError) return;
    manualSessionSaving = true;
    try {
      await api.manualSession(gameId, utc(manualStart), utc(manualEnd));
      cancelManualSession();
      await load();
    } catch {
      manualSessionError =
        '手動セッションを追加できませんでした。入力内容を確認して、もう一度お試しください。';
    } finally {
      manualSessionSaving = false;
    }
  }
  async function saveSession() {
    if (!selected) return;
    sessionFormError = selected.exited_at
      ? validateSessionEdit(sessionStart, sessionEnd, intervalRanges())
      : validateRunningSessionEdit(sessionStart, intervalRanges());
    if (sessionFormError) return;
    sessionSaving = true;
    try {
      await api.updateSession(
        selected.id,
        utc(sessionStart),
        selected.exited_at ? utc(sessionEnd) : null,
      );
      sessionFormDirty = false;
      cancelSessionEdit();
      await load();
    } catch {
      sessionFormError =
        'セッションを保存できませんでした。入力内容を確認して、もう一度お試しください。';
    } finally {
      sessionSaving = false;
    }
  }
  async function removeSession() {
    if (!selected) return;
    const sessionId = selected.id;
    sessionActionError = '';
    try {
      await api.deleteSession(sessionId);
      closeSessionEditor();
      await load();
    } catch {
      sessionActionError = 'セッションを削除できませんでした。もう一度お試しください。';
    }
  }
  async function removeAllSessions() {
    if (!sessions.length) return;
    pageError = '';
    try {
      await api.deleteAllSessions(gameId);
      closeSessionEditor();
      await load();
    } catch {
      pageError =
        'すべてのセッションを削除できませんでした。しばらくしてからもう一度お試しください。';
    }
  }
  function intervalRanges() {
    return intervals.map((interval) => ({
      id: interval.id,
      start: interval.started_at,
      end: interval.ended_at,
    }));
  }
  function intervalDurationSeconds(interval: BackgroundInterval) {
    const start = new Date(interval.started_at).getTime();
    const end = interval.ended_at ? new Date(interval.ended_at).getTime() : nowMs;
    return Math.max(0, Math.floor((end - start) / 1000));
  }
  function beginEditInterval(interval: BackgroundInterval) {
    editingIntervalId = interval.id;
    intervalEditStart = inputTime(interval.started_at);
    intervalEditEnd = inputTime(interval.ended_at);
    intervalEditError = '';
  }
  function cancelEditInterval() {
    editingIntervalId = null;
    intervalEditStart = '';
    intervalEditEnd = '';
    intervalEditError = '';
  }
  async function saveInterval() {
    if (!selected || editingIntervalId === null) return;
    const interval = intervals.find((item) => item.id === editingIntervalId);
    if (!interval) return;
    const intervalEnd = interval.ended_at ? intervalEditEnd : null;
    const validationError = validateBackgroundInterval(
      intervalEditStart,
      intervalEnd,
      { start: selected.launched_at, end: selected.exited_at },
      intervalRanges(),
      interval.id,
    );
    if (validationError) {
      intervalEditError = validationError;
      return;
    }
    intervalEditError = '';
    savingIntervalId = interval.id;
    try {
      await api.updateInterval(
        interval.id,
        utc(intervalEditStart),
        intervalEnd ? utc(intervalEnd) : null,
      );
      intervals = await api.intervals(interval.play_session_id);
      cancelEditInterval();
      await load();
    } catch {
      intervalEditError =
        '除外区間を保存できませんでした。入力内容を確認して、もう一度お試しください。';
    } finally {
      savingIntervalId = null;
    }
  }
  async function deleteEditingInterval() {
    const interval = intervalBeingEdited;
    if (!interval?.ended_at) return;
    intervalEditError = '';
    savingIntervalId = interval.id;
    try {
      await api.deleteInterval(interval.id);
      intervals = await api.intervals(interval.play_session_id);
      cancelEditInterval();
      await load();
    } catch {
      intervalEditError = '除外区間を削除できませんでした。もう一度お試しください。';
    } finally {
      savingIntervalId = null;
    }
  }
  function beginAddInterval() {
    newIntervalStart = '';
    newIntervalEnd = '';
    newIntervalError = '';
    newIntervalOpen = true;
  }
  function cancelAddInterval() {
    newIntervalOpen = false;
    newIntervalStart = '';
    newIntervalEnd = '';
    newIntervalError = '';
  }
  async function addInterval() {
    if (!selected) return;
    newIntervalError = validateBackgroundInterval(
      newIntervalStart,
      newIntervalEnd,
      { start: selected.launched_at, end: selected.exited_at },
      intervalRanges(),
    );
    if (newIntervalError) return;
    intervalCreating = true;
    try {
      await api.createInterval(selected.id, utc(newIntervalStart), utc(newIntervalEnd));
      intervals = await api.intervals(selected.id);
      cancelAddInterval();
      await load();
    } catch {
      newIntervalError =
        '除外区間を追加できませんでした。入力内容を確認して、もう一度お試しください。';
    } finally {
      intervalCreating = false;
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
    editReleaseDate = game.release_date ?? '';
    editReleaseDateComplete = true;
    editReleaseDateError = '';
    editSourceUrl = game.source_url ?? '';
    editingGame = true;
  }
  function cancelGameEdit() {
    editingGame = false;
    editReleaseDateError = '';
  }
  async function saveGameInfo() {
    if (!game || !editTitle.trim()) return showToast('タイトルを入力してください', true);
    if (!editReleaseDateComplete) {
      editReleaseDateError = '発売日は年・月・日をすべて選択してください。';
      return;
    }
    try {
      await api.updateGame(game.id, {
        title: editTitle.trim(),
        brand: editBrand.trim() || undefined,
        release_date: editReleaseDate || undefined,
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
    pageError = '';
    try {
      await api.refreshMetadata(gameId);
      await load();
      showToast('ErogameScapeからゲーム情報を更新しました');
    } catch (e) {
      pageError = String(e);
      showToast(`情報を更新できませんでした: ${String(e)}`, true);
    } finally {
      refreshingMeta = false;
    }
  }
  function openThumbnailEditor() {
    if (!game) return;
    thumbnailOriginalPath = game.thumbnail_path;
    thumbnailDraftPath = game.thumbnail_path;
    thumbnailError = '';
    thumbnailEditorOpen = true;
  }
  function closeThumbnailEditor() {
    if (thumbnailImporting || thumbnailCropBusy || thumbnailSaving) return;
    thumbnailEditorOpen = false;
    thumbnailError = '';
  }
  async function selectThumbnail() {
    const selected = await open({
      title: 'サムネイル画像を選択',
      multiple: false,
      directory: false,
      filters: [{ name: '画像ファイル', extensions: ['jpg', 'jpeg', 'png', 'webp'] }],
    });
    if (!selected) return;
    thumbnailImporting = true;
    thumbnailError = '';
    try {
      thumbnailDraftPath = await api.importThumbnail(selected);
    } catch (e) {
      thumbnailError = String(e);
    } finally {
      thumbnailImporting = false;
    }
  }
  async function saveThumbnail(path: string | null = thumbnailDraftPath) {
    if (!game || thumbnailImporting || thumbnailSaving) return;
    thumbnailSaving = true;
    thumbnailError = '';
    try {
      await api.updateGameThumbnail(game.id, path);
      thumbnailEditorOpen = false;
      await load();
      showToast('サムネイルを保存しました');
    } catch (e) {
      thumbnailDraftPath = path;
      thumbnailError = String(e);
    } finally {
      thumbnailSaving = false;
    }
  }
  async function removeGame() {
    pageError = '';
    try {
      await api.deleteGame(gameId);
      onback();
    } catch {
      pageError = 'ゲームを削除できませんでした。しばらくしてからもう一度お試しください。';
    }
  }
  async function launch() {
    try {
      await api.launchGame(gameId);
    } catch (e) {
      pageError = String(e);
    }
  }
  async function updatePlayStatus(status: PlayStatus) {
    if (!game || game.play_status === status) return;
    try {
      await api.updateGamePlayStatus(game.id, status);
      await load();
      showToast(`プレイ状況を「${playStatusLabel(status)}」に変更しました`);
    } catch (e) {
      showToast(`プレイ状況を変更できませんでした: ${String(e)}`, true);
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
  function beginTimestampEdit(point: GameTimestamp) {
    editingTimestampId = point.id;
    editingTimestampName = point.name;
  }
  function cancelTimestampEdit() {
    editingTimestampId = null;
    editingTimestampName = '';
  }
  async function saveTimestampName(point: GameTimestamp) {
    const name = editingTimestampName.trim();
    if (!name) return showToast('プレイ記録ポイントの名称を入力してください', true);
    if (name === point.name) {
      cancelTimestampEdit();
      return;
    }
    if (savingTimestampId !== null) return;
    savingTimestampId = point.id;
    try {
      await api.updateTimestampName(point.id, name);
      await load();
      cancelTimestampEdit();
      showToast('プレイ記録ポイントの名称を変更しました');
    } catch (e) {
      showToast(`名称を変更できませんでした: ${String(e)}`, true);
    } finally {
      savingTimestampId = null;
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
  async function deleteScreenshot(id: number) {
    try {
      await api.deleteScreenshot(id);
      if (selectedScreenshot?.id === id) selectedScreenshot = null;
      await load();
      showToast('スクリーンショットを削除しました');
    } catch (e) {
      showToast(`スクリーンショットを削除できませんでした: ${String(e)}`, true);
    }
  }
  async function openScreenshotDirectory() {
    try {
      await api.openScreenshotDirectory(gameId);
    } catch (e) {
      showToast(`保存先を開けませんでした: ${String(e)}`, true);
    }
  }
  async function openSocialCreator(screenshotId?: number) {
    void loadLocalFonts();
    socialScreenshotId = screenshotId ?? screenshots[0]?.id ?? 0;
    resetSocialLayout();
    socialTimestampIds = timestamps.map((point) => point.id);
    socialOpen = true;
    await tick();
    renderSocialPreview();
  }
  function resetSocialLayout() {
    socialLayout = 'auto';
    socialFontSize = 85;
    socialImageRatio = 70;
    applySocialPreset('auto');
  }
  function applySocialPreset(layout: SocialPreset) {
    const shot = screenshots.find((item) => item.id === socialScreenshotId);
    const aspect = shot && shot.height ? shot.width / shot.height : 16 / 9;
    const resolved = layout === 'auto' ? (aspect < 1.4 ? 'left' : 'bottom') : layout;
    socialLayout = layout;
    const sideImageLength = 96 * (socialImageRatio / 100);
    const sideInfoLength = 96 - sideImageLength;
    const verticalImageLength = 93 * (socialImageRatio / 100);
    const verticalInfoLength = 93 - verticalImageLength;
    if (resolved === 'left') {
      socialInfoRect = { x: 1, y: 2, width: sideInfoLength, height: 96 };
      socialImageRect = { x: 3 + sideInfoLength, y: 2, width: sideImageLength, height: 96 };
    } else if (resolved === 'right') {
      socialImageRect = { x: 1, y: 2, width: sideImageLength, height: 96 };
      socialInfoRect = { x: 3 + sideImageLength, y: 2, width: sideInfoLength, height: 96 };
    } else if (resolved === 'top') {
      socialInfoRect = { x: 1, y: 2, width: 98, height: verticalInfoLength };
      socialImageRect = { x: 1, y: 5 + verticalInfoLength, width: 98, height: verticalImageLength };
    } else {
      socialImageRect = { x: 1, y: 2, width: 98, height: verticalImageLength };
      socialInfoRect = { x: 1, y: 5 + verticalImageLength, width: 98, height: verticalInfoLength };
    }
    socialSelectedElement = null;
  }
  function applySocialRatio(value: number) {
    socialImageRatio = value;
    let layout: SocialPreset;
    if (socialLayout !== 'custom' && socialLayout !== 'auto') layout = socialLayout;
    else {
      const imageCenterX = socialImageRect.x + socialImageRect.width / 2;
      const imageCenterY = socialImageRect.y + socialImageRect.height / 2;
      const infoCenterX = socialInfoRect.x + socialInfoRect.width / 2;
      const infoCenterY = socialInfoRect.y + socialInfoRect.height / 2;
      const dx = (imageCenterX - infoCenterX) * 16;
      const dy = (imageCenterY - infoCenterY) * 9;
      layout =
        Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? 'left' : 'right') : dy > 0 ? 'top' : 'bottom';
    }
    applySocialPreset(layout);
  }
  function socialFontFamily() {
    return `"${socialFont.replace(/\\/g, '\\\\').replace(/"/g, '\\"')}", sans-serif`;
  }
  async function loadLocalFonts() {
    if (socialFontsLoading || socialLocalFonts.length) return;
    socialFontsLoading = true;
    try {
      socialLocalFonts = await api.systemFonts();
      if (!socialLocalFonts.length) throw new Error('利用可能なフォントが見つかりませんでした');
    } catch (e) {
      showToast(`フォント一覧を取得できませんでした: ${String(e)}`, true);
    } finally {
      socialFontsLoading = false;
    }
  }
  function socialRect(element: SocialElement) {
    return element === 'image' ? socialImageRect : socialInfoRect;
  }
  function fittedSocialImageRect(
    rect: SocialRect,
    screenshotId: number,
    availableScreenshots: GameScreenshot[],
  ) {
    const shot = availableScreenshots.find((item) => item.id === screenshotId);
    if (!shot?.width || !shot.height) return rect;
    const areaWidth = rect.width * 16;
    const areaHeight = rect.height * 9;
    const scale = Math.min(areaWidth / shot.width, areaHeight / shot.height);
    const fittedWidth = (shot.width * scale) / 16;
    const fittedHeight = (shot.height * scale) / 9;
    return {
      x: rect.x + (rect.width - fittedWidth) / 2,
      y: rect.y + (rect.height - fittedHeight) / 2,
      width: fittedWidth,
      height: fittedHeight,
    };
  }
  function setSocialRect(element: SocialElement, rect: SocialRect) {
    const next = {
      x: Math.max(0, Math.min(100 - rect.width, rect.x)),
      y: Math.max(0, Math.min(100 - rect.height, rect.y)),
      width: rect.width,
      height: rect.height,
    };
    if (element === 'image') socialImageRect = next;
    else socialInfoRect = next;
  }
  function beginSocialTransform(
    event: PointerEvent,
    element: SocialElement,
    handle?: ResizeHandle,
  ) {
    event.preventDefault();
    event.stopPropagation();
    socialSelectedElement = element;
    socialLayout = 'custom';
    const editor = (event.currentTarget as HTMLElement).closest(
      '.social-canvas-editor',
    ) as HTMLElement | null;
    if (!editor) return;
    const startX = event.clientX;
    const startY = event.clientY;
    const start = { ...(element === 'image' ? socialImageEditorRect : socialInfoRect) };
    if (element === 'image') socialImageRect = start;
    const move = (nextEvent: PointerEvent) => {
      const dx = ((nextEvent.clientX - startX) / editor.clientWidth) * 100;
      const dy = ((nextEvent.clientY - startY) / editor.clientHeight) * 100;
      if (!handle) {
        setSocialRect(element, { ...start, x: start.x + dx, y: start.y + dy });
        return;
      }
      if (element === 'image') {
        const shot = screenshots.find((item) => item.id === socialScreenshotId);
        const aspect = shot?.width && shot.height ? (shot.width / shot.height) * (9 / 16) : 1;
        const horizontalEdge = handle === 'e' || handle === 'w';
        const verticalEdge = handle === 'n' || handle === 's';
        const centerX = start.x + start.width / 2;
        const centerY = start.y + start.height / 2;
        const anchorX = handle.includes('w') ? start.x + start.width : start.x;
        const anchorY = handle.includes('n') ? start.y + start.height : start.y;
        const horizontalWidth = handle.includes('w')
          ? anchorX - (start.x + dx)
          : start.x + start.width + dx - anchorX;
        const verticalHeight = handle.includes('n')
          ? anchorY - (start.y + dy)
          : start.y + start.height + dy - anchorY;
        const useHorizontal = horizontalEdge
          ? true
          : verticalEdge
            ? false
            : Math.abs(dx) / Math.max(1, start.width) >= Math.abs(dy) / Math.max(1, start.height);
        const minimumWidth = Math.max(8, 8 * aspect);
        const maximumWidthX = horizontalEdge
          ? handle === 'w'
            ? anchorX
            : 100 - anchorX
          : verticalEdge
            ? Math.min(centerX, 100 - centerX) * 2
            : handle.includes('w')
              ? anchorX
              : 100 - anchorX;
        const maximumHeight = verticalEdge
          ? handle === 'n'
            ? anchorY
            : 100 - anchorY
          : horizontalEdge
            ? Math.min(centerY, 100 - centerY) * 2
            : handle.includes('n')
              ? anchorY
              : 100 - anchorY;
        const maximumWidth = Math.min(maximumWidthX, maximumHeight * aspect);
        const desiredWidth = useHorizontal ? horizontalWidth : verticalHeight * aspect;
        const nextWidth = Math.max(
          Math.min(minimumWidth, maximumWidth),
          Math.min(maximumWidth, desiredWidth),
        );
        const nextHeight = nextWidth / aspect;
        setSocialRect(element, {
          x: verticalEdge
            ? centerX - nextWidth / 2
            : handle.includes('w')
              ? anchorX - nextWidth
              : anchorX,
          y: horizontalEdge
            ? centerY - nextHeight / 2
            : handle.includes('n')
              ? anchorY - nextHeight
              : anchorY,
          width: nextWidth,
          height: nextHeight,
        });
        return;
      }
      let left = start.x;
      let top = start.y;
      let right = start.x + start.width;
      let bottom = start.y + start.height;
      if (handle.includes('w')) left = Math.min(right - 8, Math.max(0, left + dx));
      if (handle.includes('e')) right = Math.max(left + 8, Math.min(100, right + dx));
      if (handle.includes('n')) top = Math.min(bottom - 8, Math.max(0, top + dy));
      if (handle.includes('s')) bottom = Math.max(top + 8, Math.min(100, bottom + dy));
      setSocialRect(element, { x: left, y: top, width: right - left, height: bottom - top });
    };
    const end = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', end);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', end, { once: true });
    window.addEventListener('pointercancel', end, { once: true });
  }
  function toggleSocialTimestamp(id: number, selected: boolean) {
    socialTimestampIds = selected
      ? [...socialTimestampIds, id].filter(
          (value, index, values) => values.indexOf(value) === index,
        )
      : socialTimestampIds.filter((value) => value !== id);
  }
  function setSocialTimestampEnabled(enabled: boolean) {
    socialFields.timestamp = enabled;
    if (enabled) socialTimestampIds = timestamps.map((point) => point.id);
  }
  function canvasImage(path: string) {
    const cached = socialImageCache.get(path);
    if (cached) return cached;
    const pending = new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.crossOrigin = 'anonymous';
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error('スクリーンショットを読み込めませんでした'));
      image.src = imageSrc(path);
    });
    socialImageCache.set(path, pending);
    pending.catch(() => socialImageCache.delete(path));
    return pending;
  }
  function scheduleSocialPreview() {
    cancelAnimationFrame(socialRenderFrame);
    socialRenderFrame = requestAnimationFrame(() => renderSocialPreview());
  }
  function fitText(
    context: CanvasRenderingContext2D,
    text: string,
    maxWidth: number,
    maxSize: number,
    minSize: number,
    weight = 700,
  ) {
    let size = maxSize;
    do {
      context.font = `${weight} ${size}px ${socialFontFamily()}`;
      if (context.measureText(text).width <= maxWidth) return size;
      size -= 2;
    } while (size >= minSize);
    return minSize;
  }
  function wrapCanvasText(context: CanvasRenderingContext2D, text: string, maxWidth: number) {
    const lines: string[] = [];
    let line = '';
    for (const character of text) {
      const candidate = line + character;
      if (line && context.measureText(candidate).width > maxWidth) {
        lines.push(line);
        line = character;
      } else {
        line = candidate;
      }
    }
    if (line) lines.push(line);
    return lines;
  }
  async function renderSocialPreview() {
    const canvas = socialCanvas;
    const shot = screenshots.find((item) => item.id === socialScreenshotId);
    if (!canvas || !shot || !game) return;
    const token = ++socialRenderToken;
    try {
      const image = await canvasImage(shot.path);
      if (token !== socialRenderToken) return;
      const context = canvas.getContext('2d');
      if (!context) return;
      const palette =
        socialStyle === 'rose'
          ? { accent: '#ff82b2', panel: 'rgba(55, 15, 34, .88)', glow: 'rgba(255, 80, 150, .42)' }
          : socialStyle === 'ocean'
            ? { accent: '#69c8ff', panel: 'rgba(8, 29, 50, .88)', glow: 'rgba(40, 155, 240, .4)' }
            : { accent: '#b49aff', panel: 'rgba(18, 14, 29, .9)', glow: 'rgba(121, 87, 213, .42)' };
      const selectedPoints = timestamps.filter((item) => socialTimestampIds.includes(item.id));
      const pointText = (point: GameTimestamp) => {
        const difference = socialTimestampTimeMode === 'difference';
        const seconds = difference ? point.since_previous_seconds : point.playtime_seconds;
        return `◆ ${point.name} — ${duration(seconds)}`;
      };
      const hasVisibleInfo =
        socialFields.title ||
        socialFields.brand ||
        socialFields.playtime ||
        socialFields.playStatus ||
        socialFields.sessionCount ||
        socialFields.lastPlayed ||
        socialFields.capturedAt ||
        (socialFields.timestamp && selectedPoints.length > 0);
      const width = 1600,
        height = 900;
      const fromPercentRect = (rect: SocialRect) => ({
        x: (rect.x / 100) * width,
        y: (rect.y / 100) * height,
        width: (rect.width / 100) * width,
        height: (rect.height / 100) * height,
      });
      const imageArea = fromPercentRect(socialImageRect);
      const infoArea = fromPercentRect(socialInfoRect);
      const sideLayout = infoArea.height >= infoArea.width;
      if (canvas.width !== width) canvas.width = width;
      if (canvas.height !== height) canvas.height = height;

      // Use the screenshot as a subdued background only where custom spacing creates empty space.
      const backgroundScale = Math.max(width / image.naturalWidth, height / image.naturalHeight);
      const backgroundWidth = image.naturalWidth * backgroundScale,
        backgroundHeight = image.naturalHeight * backgroundScale;
      context.save();
      context.filter = 'blur(28px) brightness(0.55)';
      context.drawImage(
        image,
        (width - backgroundWidth) / 2 - 20,
        (height - backgroundHeight) / 2 - 20,
        backgroundWidth + 40,
        backgroundHeight + 40,
      );
      context.restore();
      context.fillStyle = 'rgba(5, 5, 9, .42)';
      context.fillRect(0, 0, width, height);
      const imageScale = Math.min(
        imageArea.width / image.naturalWidth,
        imageArea.height / image.naturalHeight,
      );
      const foregroundWidth = image.naturalWidth * imageScale,
        foregroundHeight = image.naturalHeight * imageScale,
        foregroundX = imageArea.x + (imageArea.width - foregroundWidth) / 2,
        foregroundY = imageArea.y + (imageArea.height - foregroundHeight) / 2,
        frameX = foregroundX,
        frameY = foregroundY,
        frameWidth = foregroundWidth,
        frameHeight = foregroundHeight;
      context.save();
      context.shadowColor = 'rgba(0,0,0,.68)';
      context.shadowBlur = 34;
      context.fillStyle = '#08070b';
      context.beginPath();
      context.roundRect(frameX - 5, frameY - 5, frameWidth + 10, frameHeight + 10, 18);
      context.fill();
      context.shadowBlur = 0;
      context.beginPath();
      context.roundRect(frameX, frameY, frameWidth, frameHeight, 14);
      context.clip();
      context.drawImage(image, foregroundX, foregroundY, foregroundWidth, foregroundHeight);
      context.restore();
      if (hasVisibleInfo) {
        const boxX = infoArea.x,
          boxY = infoArea.y,
          boxWidth = infoArea.width,
          boxHeight = infoArea.height;
        const referenceWidth = sideLayout ? width * 0.28 : width * 0.98;
        const referenceHeight = sideLayout ? height * 0.96 : height * 0.26;
        const panelScale = Math.max(
          0.5,
          Math.min(1.35, boxWidth / referenceWidth, boxHeight / referenceHeight),
        );
        const scaled = (value: number) => value * panelScale;
        const fontScaled = (value: number) => scaled(value) * (socialFontSize / 100);
        context.shadowColor = palette.glow;
        context.shadowBlur = scaled(32);
        context.fillStyle = palette.panel;
        context.beginPath();
        context.roundRect(boxX, boxY, boxWidth, boxHeight, scaled(24));
        context.fill();
        context.shadowBlur = 0;
        context.fillStyle = palette.accent;
        context.fillRect(boxX, boxY, scaled(9), boxHeight);
        context.save();
        context.beginPath();
        context.roundRect(boxX, boxY, boxWidth, boxHeight, scaled(24));
        context.clip();
        const textX = boxX + fontScaled(42);
        let textY = boxY + fontScaled(sideLayout ? 72 : 50);
        if (socialFields.title) {
          const titleSize = fitText(
            context,
            game.title,
            boxWidth - fontScaled(82),
            fontScaled(sideLayout ? 46 : 42),
            fontScaled(sideLayout ? 24 : 25),
            800,
          );
          context.font = `800 ${titleSize}px ${socialFontFamily()}`;
          context.fillStyle = '#fff';
          context.fillText(game.title, textX, textY);
          textY += titleSize + fontScaled(sideLayout ? 36 : 17);
        }
        const facts: string[] = [];
        if (socialFields.brand && game.brand) facts.push(game.brand);
        if (socialFields.playStatus) facts.push(`プレイ状況: ${playStatusLabel(game.play_status)}`);
        if (socialFields.playtime)
          facts.push(`プレイ時間: ${duration(game.total_playtime_seconds)}`);
        if (socialFields.sessionCount) facts.push(`${game.session_count}セッション`);
        if (socialFields.lastPlayed && game.last_played)
          facts.push(`最終プレイ: ${local(game.last_played)}`);
        context.fillStyle = '#eeeaf5';
        if (sideLayout) {
          for (const fact of facts) {
            const factSize = fitText(
              context,
              fact,
              boxWidth - fontScaled(82),
              fontScaled(27),
              fontScaled(19),
              600,
            );
            context.font = `600 ${factSize}px ${socialFontFamily()}`;
            context.fillText(fact, textX, textY);
            textY += fontScaled(46);
          }
        } else {
          const factLine = facts.join('  •  ');
          if (factLine) {
            const factSize = fitText(
              context,
              factLine,
              boxWidth - fontScaled(90),
              fontScaled(25),
              fontScaled(18),
              600,
            );
            context.font = `600 ${factSize}px ${socialFontFamily()}`;
            context.fillText(factLine, textX, textY);
            textY += fontScaled(37);
          }
        }
        if (socialFields.timestamp && selectedPoints.length) {
          context.fillStyle = palette.accent;
          const reservedBottom = fontScaled(socialFields.capturedAt ? 55 : 22);
          const availablePointHeight = Math.max(
            fontScaled(30),
            boxY + boxHeight - reservedBottom - textY,
          );
          let pointSize = fontScaled(sideLayout ? 23 : 21);
          const minimumPointSize = fontScaled(16);
          let wrappedPoints: string[][] = [];
          while (pointSize >= minimumPointSize) {
            context.font = `700 ${pointSize}px ${socialFontFamily()}`;
            wrappedPoints = selectedPoints.map((point) =>
              wrapCanvasText(context, pointText(point), boxWidth - fontScaled(84)),
            );
            const lineCount = wrappedPoints.reduce((total, lines) => total + lines.length, 0);
            if (lineCount * (pointSize + fontScaled(6)) <= availablePointHeight) break;
            pointSize -= 1;
          }
          pointSize = Math.max(minimumPointSize, pointSize);
          context.font = `700 ${pointSize}px ${socialFontFamily()}`;
          for (const lines of wrappedPoints) {
            for (const line of lines) {
              context.fillText(line, textX, textY);
              textY += pointSize + fontScaled(6);
            }
            textY += fontScaled(sideLayout ? 5 : 2);
          }
        }
        if (socialFields.capturedAt) {
          context.fillStyle = 'rgba(255,255,255,.72)';
          context.font = `500 ${fontScaled(22)}px ${socialFontFamily()}`;
          context.textAlign = 'right';
          context.fillText(
            `撮影: ${local(shot.captured_at)}`,
            boxX + boxWidth - fontScaled(35),
            boxY + boxHeight - fontScaled(28),
          );
          context.textAlign = 'left';
        }
        context.restore();
      }
    } catch (e) {
      showToast(String(e), true);
    }
  }
  async function saveSocialImage() {
    if (!socialCanvas || socialSaving) return;
    socialSaving = true;
    try {
      await renderSocialPreview();
      const encoded = socialCanvas.toDataURL('image/png').split(',', 2)[1];
      await api.saveSocialImage(gameId, encoded);
      showToast('SNS投稿用画像を保存しました');
    } catch (e) {
      showToast(`画像を保存できませんでした: ${String(e)}`, true);
    } finally {
      socialSaving = false;
    }
  }
  async function openSocialImageDirectory() {
    try {
      await api.openSocialImageDirectory(gameId);
    } catch (e) {
      showToast(`保存先を開けませんでした: ${String(e)}`, true);
    }
  }
</script>

<button class="back-button" onclick={onback}>← ライブラリに戻る</button
>{#if game}{#if game.thumbnail_path}<div
      class="detail-backdrop"
      style:background-image={`url("${imageSrc(game.thumbnail_path)}")`}
    ></div>{/if}
  <section class="detail">
    <div class="hero">
      <div class="detail-image">
        {#if game.thumbnail_path}<img src={imageSrc(game.thumbnail_path)} alt="" />{:else}<div
            class="placeholder"
          >
            NO IMAGE
          </div>{/if}<button class="launch-overlay" onclick={launch}>▶ 起動</button><button
          type="button"
          class="detail-thumbnail-edit"
          aria-label="サムネイルを編集"
          onclick={openThumbnailEditor}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M4 16.5V20h3.5L18.1 9.4l-3.5-3.5L4 16.5Zm16.7-9.8a1 1 0 0 0 0-1.4l-2-2a1 1 0 0 0-1.4 0l-1.6 1.6 3.5 3.5 1.5-1.7Z"
            />
          </svg>
        </button>
      </div>
      <div>
        <h1>{game.title}</h1>
        <p>{game.brand ?? 'ブランド未設定'} ・ {game.release_date ?? '発売日未設定'}</p>
        <span class="play-status status-{game.play_status}"
          >{playStatusLabel(game.play_status)}</span
        >
        <small>プレイ時間</small>
        <h2>{duration(game.total_playtime_seconds)}</h2>
        <p>{game.session_count} セッション</p>
        <button class="primary social-create-button" onclick={() => openSocialCreator()}
          >SNS用プレイ記録画像を作成</button
        >
      </div>
    </div>
    <section class="panel game-info">
      <div class="panel-heading">
        <h2>ゲーム情報</h2>
        {#if !editingGame}<div class="game-info-heading-actions">
            <button class="metadata-refresh" onclick={refreshMeta} disabled={refreshingMeta}
              >{#if refreshingMeta}<span class="spinner" aria-hidden="true"
                ></span>取得中…{:else}ErogameScapeから情報を更新{/if}</button
            ><button onclick={beginGameEdit}>編集</button>
          </div>{/if}
      </div>
      {#if editingGame}<div class="game-info-form">
          <label>タイトル<input bind:value={editTitle} /></label><label
            >ブランド<input bind:value={editBrand} placeholder="未設定" /></label
          >
          <div class="game-info-form-field">
            <DateTimeSelect
              label="発売日"
              value={editReleaseDate}
              withTime={false}
              optional
              invalid={Boolean(editReleaseDateError)}
              onchange={(value, complete) => {
                editReleaseDate = value;
                editReleaseDateComplete = complete;
                editReleaseDateError = '';
              }}
            />
            {#if editReleaseDateError}<p class="form-error" role="alert">
                {editReleaseDateError}
              </p>{/if}
          </div>
          <label
            >ErogameScape URL<input
              type="url"
              bind:value={editSourceUrl}
              placeholder="https://erogamescape.dyndns.org/…"
            /></label
          >
          <div class="actions">
            <button class="primary" onclick={saveGameInfo}>保存</button><button
              onclick={cancelGameEdit}>キャンセル</button
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
            <dt>発売日</dt>
            <dd>{game.release_date ?? '未設定'}</dd>
          </div>
          <div>
            <dt>プレイ状況</dt>
            <dd>
              <select
                class="play-status-select"
                value={game.play_status}
                onchange={(event) =>
                  updatePlayStatus((event.currentTarget as HTMLSelectElement).value as PlayStatus)}
              >
                {#each playStatusOptions as option}<option value={option.value}
                    >{option.label}</option
                  >{/each}
              </select>
            </dd>
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
    <section class="panel screenshot-panel">
      <div class="panel-heading">
        <h2>スクリーンショット</h2>
        <div class="screenshot-heading-actions">
          <small>{screenshots.length}枚</small>
          <label class="page-size-control"
            >表示<select
              value={screenshotPageSize}
              onchange={(event) => {
                screenshotPageSize = Number((event.currentTarget as HTMLSelectElement).value);
                screenshotPage = 1;
              }}
              >{#each pageSizeOptions as size}<option value={size}>{size}件</option>{/each}</select
            ></label
          >
          <button onclick={openScreenshotDirectory}>保存先を開く</button>
        </div>
      </div>
      {#if screenshots.length}
        <div class="screenshot-grid">
          {#each pagedScreenshots as shot}
            <article class="screenshot-card">
              <button class="screenshot-preview" onclick={() => (selectedScreenshot = shot)}>
                <img
                  src={imageSrc(shot.path)}
                  alt={`${local(shot.captured_at)}のスクリーンショット`}
                />
              </button>
              <div>
                <small>{local(shot.captured_at)}</small>
              </div>
            </article>
          {/each}
        </div>
        {#if screenshotPageCount > 1}<div
            class="pagination"
            aria-label="スクリーンショットのページ移動"
          >
            <button disabled={screenshotPage === 1} onclick={() => (screenshotPage -= 1)}
              >← 前へ</button
            >
            <span>{screenshotPage} / {screenshotPageCount}</span>
            <button
              disabled={screenshotPage === screenshotPageCount}
              onclick={() => (screenshotPage += 1)}>次へ →</button
            >
          </div>{/if}
      {:else}
        <p class="hint">計測中のゲームがフォアグラウンドにあるとき、設定したキーで撮影できます。</p>
      {/if}
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
              {#if editingTimestampId === point.id}<input
                  class="timestamp-name-input"
                  aria-label="プレイ記録ポイントの名称"
                  maxlength="100"
                  bind:value={editingTimestampName}
                  onkeydown={(event) => {
                    if (event.key === 'Enter') saveTimestampName(point);
                    if (event.key === 'Escape') cancelTimestampEdit();
                  }}
                />{:else}<h3>{point.name}</h3>{/if}
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
            <div class="timestamp-item-actions">
              {#if editingTimestampId === point.id}<button
                  class="primary"
                  disabled={savingTimestampId === point.id || !editingTimestampName.trim()}
                  onclick={() => saveTimestampName(point)}
                  >{savingTimestampId === point.id ? '保存中…' : '保存'}</button
                ><button disabled={savingTimestampId === point.id} onclick={cancelTimestampEdit}
                  >キャンセル</button
                >{:else}<button onclick={() => beginTimestampEdit(point)}>編集</button><DeleteButton
                  title="プレイ記録ポイントの削除"
                  message={`プレイ記録ポイント「${point.name}」を削除します。元に戻せません。`}
                  onconfirm={() => deleteTimestamp(point.id)}
                />{/if}
            </div>
          </article>{/each}
      </div>
    </section>
    <section class="panel">
      <h2>実行ファイル</h2>
      {#each game.executables as x}<div class="listrow">
          <code>{x.path}</code><DeleteButton
            title="実行ファイル登録の削除"
            message={`実行ファイル「${x.path}」の登録を削除します。ファイル自体は削除されません。`}
            onconfirm={() => removeExe(x.id)}
          />
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
      <p class="hint">開始日時と終了日時を指定して、過去のプレイ記録を追加できます。</p>
      <button class="primary" type="button" onclick={beginManualSession}
        >手動セッションを追加</button
      >
    </section>
    <section class="panel">
      <div class="panel-heading">
        <h2>Session History</h2>
        <label class="page-size-control"
          >表示<select
            value={sessionPageSize}
            onchange={(event) => {
              sessionPageSize = Number((event.currentTarget as HTMLSelectElement).value);
              sessionPage = 1;
            }}
            >{#each pageSizeOptions as size}<option value={size}>{size}件</option>{/each}</select
          ></label
        >
      </div>
      {#each pagedSessions as s}<HistoryDataRow
          label={`${local(s.launched_at)} → ${local(s.exited_at)}${s.needs_review ? ' ・ 要確認' : ''}`}
          seconds={s.playtime_seconds}
          selected={selected?.id === s.id}
          recording={!s.exited_at}
          onselect={() => beginSessionDetail(s)}
        />{/each}
      {#if sessionPageCount > 1}<div class="pagination" aria-label="セッション履歴のページ移動">
          <button disabled={sessionPage === 1} onclick={() => (sessionPage -= 1)}>← 前へ</button>
          <span>{sessionPage} / {sessionPageCount}</span>
          <button disabled={sessionPage === sessionPageCount} onclick={() => (sessionPage += 1)}
            >次へ →</button
          >
        </div>{/if}
    </section>
    <section class="panel danger-zone">
      <div class="danger-zone-heading">
        <h2>危険な操作</h2>
        <p>ここで行った削除は元に戻せません。</p>
      </div>
      <div class="danger-zone-item">
        <div>
          <strong>すべてのセッションを削除</strong>
          <p>このゲームのセッションと、各セッションに含まれる除外時間を削除します。</p>
        </div>
        <DeleteButton
          label="すべてのセッションを削除"
          title="すべてのセッションを削除"
          message={`${sessions.length}件のセッションと除外時間の記録をすべて削除します。元に戻せません。`}
          disabled={!sessions.length}
          onconfirm={removeAllSessions}
        />
      </div>
      <div class="danger-zone-item">
        <div>
          <strong>ゲームを削除</strong>
          <p>ゲーム情報と、このゲームに関連するすべての記録を削除します。</p>
        </div>
        <DeleteButton
          label="ゲームを削除"
          title="ゲームの削除"
          message={`「${game.title}」のゲーム情報、すべてのプレイ履歴、記録ポイント、スクリーンショット、SNS画像を削除します。元に戻せません。`}
          onconfirm={removeGame}
        />
      </div>
    </section>
  </section>{/if}
{#if thumbnailEditorOpen}<div class="modal detail-thumbnail-modal">
    <div
      class="panel detail-thumbnail-dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="detail-thumbnail-title"
    >
      <button
        class="close"
        aria-label="閉じる"
        disabled={thumbnailImporting || thumbnailCropBusy || thumbnailSaving}
        onclick={closeThumbnailEditor}>×</button
      >
      <h2 id="detail-thumbnail-title">サムネイルを編集</h2>
      {#if thumbnailError}<p class="error">{thumbnailError}</p>{/if}
      {#if thumbnailDraftPath}{#key thumbnailDraftPath}<ThumbnailCropEditor
            imagePath={thumbnailDraftPath}
            ondone={saveThumbnail}
            onremove={() => (thumbnailDraftPath = null)}
            onselect={selectThumbnail}
            onbusy={(busy) => (thumbnailCropBusy = busy)}
            selecting={thumbnailImporting}
            saveDisabled={thumbnailDraftPath === thumbnailOriginalPath}
          />{/key}{:else}<div class="thumbnail-preview detail-thumbnail-empty">
          <span>NO IMAGE</span>
        </div>
        <div class="actions detail-thumbnail-actions">
          <button
            type="button"
            disabled={thumbnailImporting ||
              thumbnailSaving ||
              thumbnailDraftPath === thumbnailOriginalPath}
            onclick={() => (thumbnailDraftPath = thumbnailOriginalPath)}>リセット</button
          >
          <button
            type="button"
            disabled={thumbnailImporting || thumbnailSaving}
            onclick={selectThumbnail}>{thumbnailImporting ? '取込中…' : '画像を選択…'}</button
          >
          <button
            type="button"
            class="primary"
            disabled={thumbnailImporting ||
              thumbnailSaving ||
              thumbnailDraftPath === thumbnailOriginalPath}
            onclick={() => saveThumbnail()}>{thumbnailSaving ? '保存中…' : '保存'}</button
          >
        </div>{/if}
    </div>
  </div>{/if}
{#if socialOpen}<div class="modal social-image-modal">
    <div
      class="panel social-image-creator"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="social-image-title"
      onpointerdown={() => (socialSelectedElement = null)}
    >
      <div class="panel-heading">
        <div>
          <h2 id="social-image-title">SNS投稿用画像を作成</h2>
          <p class="hint">1600×900のPNG画像として保存します。</p>
        </div>
        <button class="close" aria-label="閉じる" onclick={() => (socialOpen = false)}>×</button>
      </div>
      <div class="social-image-layout">
        <div class="social-image-controls">
          <fieldset>
            <legend>1. スクリーンショット</legend>
            <div class="social-shot-picker">
              {#if screenshots.length}
                {#each screenshots as shot}<button
                    type="button"
                    class:selected={socialScreenshotId === shot.id}
                    aria-pressed={socialScreenshotId === shot.id}
                    onclick={() => (socialScreenshotId = shot.id)}
                    ><img src={imageSrc(shot.path)} alt="" /><small>{local(shot.captured_at)}</small
                    ></button
                  >{/each}
              {:else}
                <div class="social-no-screenshot error">
                  <strong>スクリーンショットがありません</strong>
                  <span>計測中のゲームを撮影してから、もう一度画像を作成してください。</span>
                </div>
              {/if}
            </div>
          </fieldset>
          <fieldset>
            <legend>2. デザイン</legend>
            <div class="social-style-options">
              <label
                ><input type="radio" bind:group={socialStyle} value="midnight" />ミッドナイト</label
              >
              <label><input type="radio" bind:group={socialStyle} value="rose" />ローズ</label>
              <label><input type="radio" bind:group={socialStyle} value="ocean" />オーシャン</label>
            </div>
            <div class="social-layout-controls">
              <label
                >配置プリセット<select
                  value={socialLayout}
                  onchange={(event) =>
                    applySocialPreset(
                      (event.currentTarget as HTMLSelectElement).value as SocialPreset,
                    )}
                  ><option value="auto">自動</option><option value="left">左</option><option
                    value="right">右</option
                  ><option value="top">上</option><option value="bottom">下</option><option
                    value="custom"
                    disabled>カスタム</option
                  ></select
                ></label
              >
              <p class="social-editor-help">
                プレビュー内の要素をドラッグして移動できます。四隅や辺をドラッグすると大きさを変更できます。
              </p>
              <div class="social-font-picker">
                <label
                  >フォントを検索<input
                    bind:value={socialFontSearch}
                    placeholder="名前で検索"
                  /></label
                >
                <label class="social-font-select"
                  >フォント<select
                    value={socialFont}
                    size="7"
                    onchange={(event) =>
                      (socialFont = (event.currentTarget as HTMLSelectElement).value)}
                    >{#each socialFilteredFonts as font}<option value={font}>{font}</option
                      >{/each}</select
                  ></label
                >
                <button
                  type="button"
                  disabled={socialFont === defaultSocialFont}
                  onclick={() => (socialFont = defaultSocialFont)}>標準フォントに戻す</button
                >
              </div>
              <label
                ><span>フォントサイズ <output>{socialFontSize}%</output></span><input
                  type="range"
                  min="60"
                  max="130"
                  step="5"
                  bind:value={socialFontSize}
                /></label
              >
              <label
                ><span
                  >スクショと情報パネルの比率 <output
                    >{socialImageRatio}:{100 - socialImageRatio}</output
                  ></span
                ><input
                  type="range"
                  min="45"
                  max="82"
                  step="1"
                  value={socialImageRatio}
                  oninput={(event) =>
                    applySocialRatio(Number((event.currentTarget as HTMLInputElement).value))}
                /></label
              >
              <button type="button" onclick={resetSocialLayout}>レイアウトを初期値に戻す</button>
            </div>
          </fieldset>
          <fieldset>
            <legend>3. 画像に載せる情報</legend>
            <div class="social-field-options">
              <label
                ><input type="checkbox" bind:checked={socialFields.title} />ゲームタイトル</label
              >
              <label><input type="checkbox" bind:checked={socialFields.brand} />ブランド</label>
              <label><input type="checkbox" bind:checked={socialFields.playtime} />プレイ時間</label
              >
              <label
                ><input type="checkbox" bind:checked={socialFields.playStatus} />プレイ状況</label
              >
              <label
                ><input
                  type="checkbox"
                  bind:checked={socialFields.sessionCount}
                />セッション数</label
              >
              <label
                ><input
                  type="checkbox"
                  bind:checked={socialFields.lastPlayed}
                />最終プレイ日時</label
              >
              <label><input type="checkbox" bind:checked={socialFields.capturedAt} />撮影日時</label
              >
              <label
                ><input
                  type="checkbox"
                  checked={socialFields.timestamp}
                  disabled={!timestamps.length}
                  onchange={(event) =>
                    setSocialTimestampEnabled((event.currentTarget as HTMLInputElement).checked)}
                />プレイ記録ポイント</label
              >
            </div>
            {#if socialFields.timestamp && timestamps.length}<div class="social-timestamp-picker">
                <label class="social-timestamp-time-mode"
                  >ポイントの時間<select bind:value={socialTimestampTimeMode}
                    ><option value="total">ゲーム開始からの累計</option><option value="difference"
                      >前のポイントからの差分</option
                    ></select
                  ></label
                >
                <div class="social-timestamp-selection-heading">
                  <span>掲載する記録（{socialTimestampIds.length}/{timestamps.length}件）</span>
                  <span class="social-timestamp-actions">
                    <button
                      type="button"
                      onclick={() => (socialTimestampIds = timestamps.map((point) => point.id))}
                      >すべて選択</button
                    >
                    <button type="button" onclick={() => (socialTimestampIds = [])}
                      >すべて解除</button
                    >
                  </span>
                </div>
                <div class="social-timestamp-list">
                  {#each timestamps as point}<label
                      ><input
                        type="checkbox"
                        checked={socialTimestampIds.includes(point.id)}
                        onchange={(event) =>
                          toggleSocialTimestamp(
                            point.id,
                            (event.currentTarget as HTMLInputElement).checked,
                          )}
                      /><span>{point.name}</span><small>{duration(point.playtime_seconds)}</small
                      ></label
                    >{/each}
                </div>
              </div>{/if}
          </fieldset>
          <p class="social-guideline-note">
            投稿前に作品の画像投稿ガイドライン、ネタバレ、成人向け表現を確認してください。
          </p>
        </div>
        <div class="social-preview-area">
          {#if screenshots.length}<div class="social-canvas-editor">
              <canvas
                bind:this={socialCanvas}
                width="1600"
                height="900"
                aria-label="生成画像のプレビュー"
              ></canvas>
              {#each ['image', 'info'] as element}
                {@const editableElement = element as SocialElement}
                {@const rect = editableElement === 'image' ? socialImageEditorRect : socialInfoRect}
                <div
                  class="social-edit-region"
                  class:selected={socialSelectedElement === editableElement}
                  class:info={editableElement === 'info'}
                  style={`left:${rect.x}%;top:${rect.y}%;width:${rect.width}%;height:${rect.height}%`}
                >
                  <button
                    type="button"
                    class="social-edit-move"
                    aria-label={`${editableElement === 'image' ? 'スクリーンショット' : '情報パネル'}を移動`}
                    onpointerdown={(event) => beginSocialTransform(event, editableElement)}
                  ></button>
                  {#each ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'] as handle}
                    <button
                      type="button"
                      class="social-resize-handle {handle}"
                      aria-label={`${editableElement === 'image' ? 'スクリーンショット' : '情報パネル'}のサイズを変更`}
                      onpointerdown={(event) =>
                        beginSocialTransform(event, editableElement, handle as ResizeHandle)}
                    ></button>
                  {/each}
                </div>
              {/each}
            </div>{:else}<div class="social-empty-preview">
              <strong>プレビューできる画像がありません</strong>
              <span>スクリーンショットを1枚以上撮影すると、ここに生成結果が表示されます。</span>
            </div>{/if}
          <div class="social-image-actions">
            <button onclick={openSocialImageDirectory}>保存先を開く</button>
            <button
              class="primary"
              disabled={socialSaving || !screenshots.length}
              onclick={saveSocialImage}>{socialSaving ? '保存中…' : 'PNGを保存'}</button
            >
          </div>
        </div>
      </div>
    </div>
  </div>{/if}
{#if manualSessionOpen}<div class="modal manual-session-modal">
    <div
      class="panel editor manual-session-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="manual-session-title"
    >
      <button
        class="close"
        aria-label="閉じる"
        disabled={manualSessionSaving}
        onclick={cancelManualSession}>×</button
      >
      <h2 id="manual-session-title">手動セッションを追加</h2>
      <form
        class="history-range-form"
        novalidate
        onsubmit={(event) => {
          event.preventDefault();
          addManual();
        }}
      >
        <div class="history-range-fields">
          <DateTimeSelect
            label="開始日時"
            value={manualStart}
            disabled={manualSessionSaving}
            invalid={Boolean(manualSessionError)}
            onchange={(value) => {
              manualStart = value;
              manualSessionError = '';
            }}
          />
          <DateTimeSelect
            label="終了日時"
            value={manualEnd}
            disabled={manualSessionSaving}
            invalid={Boolean(manualSessionError)}
            onchange={(value) => {
              manualEnd = value;
              manualSessionError = '';
            }}
          />
        </div>
        <div class="actions">
          <button type="button" disabled={manualSessionSaving} onclick={cancelManualSession}
            >キャンセル</button
          ><button class="primary" type="submit" disabled={manualSessionSaving}
            >{manualSessionSaving ? '追加中…' : '追加'}</button
          >
        </div>
        {#if manualSessionError}<p class="form-error" role="alert">{manualSessionError}</p>{/if}
      </form>
    </div>
  </div>{/if}
{#if selected}<div class="modal">
    <div
      class="panel editor"
      role="dialog"
      aria-modal="true"
      aria-labelledby="session-detail-title"
    >
      <button
        class="close"
        aria-label="閉じる"
        disabled={sessionSaving || savingIntervalId !== null}
        onclick={closeSessionEditor}>×</button
      >
      <h2 id="session-detail-title">セッション詳細</h2>
      <p class="hint">Session #{selected.id}</p>
      <div class="session-breakdown">
        <span
          ><small>起動時間</small><strong
            >{duration(
              selected.running_seconds ??
                Math.max(0, Math.floor((nowMs - new Date(selected.launched_at).getTime()) / 1000)),
            )}</strong
          ></span
        >
        <span><small>プレイ時間</small><strong>{duration(selected.playtime_seconds)}</strong></span>
        <span><small>除外時間</small><strong>{duration(selected.background_seconds)}</strong></span>
      </div>
      <div class="session-breakdown session-date-breakdown">
        <span><small>開始日時</small><strong>{local(selected.launched_at)}</strong></span>
        <span><small>終了日時</small><strong>{local(selected.exited_at)}</strong></span>
      </div>
      <div class="actions session-detail-actions">
        <button class="primary" type="button" onclick={beginSessionEdit}>セッションを編集</button
        ><DeleteButton
          title="セッションの削除"
          message={`${local(selected.launched_at)} から始まるセッションを削除します。除外時間の記録も削除され、元に戻せません。`}
          onconfirm={removeSession}
        />
      </div>
      {#if sessionActionError}<p class="form-error" role="alert">{sessionActionError}</p>{/if}
      <h3>プレイ時間から除外した時間</h3>
      <p class="hint">
        アプリがバックグラウンドにあった区間です。起動時間からこの合計を除外します。
      </p>
      {#if intervalListError}<p class="form-error" role="alert">{intervalListError}</p>{/if}
      {#each intervals as i (i.id)}<HistoryDataRow
          label={`${local(i.started_at)} ～ ${
            i.ended_at ? local(i.ended_at) : 'バックグラウンド時間を記録中'
          }`}
          seconds={intervalDurationSeconds(i)}
          recording={!i.ended_at}
          disabled={savingIntervalId === i.id}
          onselect={() => beginEditInterval(i)}
        />{/each}
      <button type="button" onclick={beginAddInterval}>除外区間を追加</button>
    </div>
  </div>{/if}
{#if sessionEditOpen && selected}<div class="modal history-entry-modal">
    <div
      class="panel editor history-entry-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="session-editor-title"
    >
      <button class="close" aria-label="閉じる" disabled={sessionSaving} onclick={cancelSessionEdit}
        >×</button
      >
      <h2 id="session-editor-title">セッションを編集</h2>
      <form
        class="history-range-form"
        novalidate
        onsubmit={(event) => {
          event.preventDefault();
          saveSession();
        }}
      >
        <div class="history-range-fields">
          <DateTimeSelect
            label="開始日時"
            value={sessionStart}
            disabled={sessionSaving}
            invalid={Boolean(sessionFormError)}
            onchange={(value) => {
              sessionStart = value;
              sessionFormDirty = true;
              sessionFormError = '';
            }}
          />
          {#if selected.exited_at}<DateTimeSelect
              label="終了日時"
              value={sessionEnd}
              disabled={sessionSaving}
              invalid={Boolean(sessionFormError)}
              onchange={(value) => {
                sessionEnd = value;
                sessionFormDirty = true;
                sessionFormError = '';
              }}
            />{:else}<div class="running-end-note">
              <small>終了日時</small>
              <strong>実行中</strong>
              <span>終了日時は追跡終了時に自動で記録されます。</span>
            </div>{/if}
        </div>
        <div class="actions">
          <button type="button" disabled={sessionSaving} onclick={cancelSessionEdit}
            >キャンセル</button
          ><button class="primary" type="submit" disabled={sessionSaving}
            >{sessionSaving ? '保存中…' : '保存'}</button
          >
        </div>
        {#if sessionFormError}<p class="form-error" role="alert">{sessionFormError}</p>{/if}
      </form>
    </div>
  </div>{/if}
{#if editingIntervalId !== null && selected && intervalBeingEdited}<div
    class="modal history-entry-modal"
  >
    <div
      class="panel editor history-entry-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="interval-editor-title"
    >
      <button
        class="close"
        aria-label="閉じる"
        disabled={savingIntervalId === editingIntervalId}
        onclick={cancelEditInterval}>×</button
      >
      <h2 id="interval-editor-title">除外区間を編集</h2>
      <p class="hint">
        セッション範囲：{local(selected.launched_at)} ～ {local(selected.exited_at)}
      </p>
      <form
        class="history-range-form"
        novalidate
        onsubmit={(event) => {
          event.preventDefault();
          saveInterval();
        }}
      >
        <div class="history-range-fields">
          <DateTimeSelect
            label="開始日時"
            value={intervalEditStart}
            disabled={savingIntervalId === editingIntervalId}
            invalid={Boolean(intervalEditError)}
            onchange={(value) => {
              intervalEditStart = value;
              intervalEditError = '';
            }}
          />
          {#if intervalBeingEdited.ended_at}<DateTimeSelect
              label="終了日時"
              value={intervalEditEnd}
              disabled={savingIntervalId === editingIntervalId}
              invalid={Boolean(intervalEditError)}
              onchange={(value) => {
                intervalEditEnd = value;
                intervalEditError = '';
              }}
            />{:else}<div class="running-end-note">
              <small>終了日時</small>
              <strong>記録中</strong>
              <span>終了日時はフォアグラウンド復帰時に自動で記録されます。</span>
            </div>{/if}
        </div>
        <div class="actions interval-editor-actions">
          <button
            type="button"
            disabled={savingIntervalId === editingIntervalId}
            onclick={cancelEditInterval}>キャンセル</button
          ><button class="primary" type="submit" disabled={savingIntervalId === editingIntervalId}
            >{savingIntervalId === editingIntervalId ? '保存中…' : '保存'}</button
          >
          {#if intervalBeingEdited.ended_at}<DeleteButton
              className="interval-editor-delete"
              disabled={savingIntervalId === editingIntervalId}
              title="除外区間の削除"
              message={`${local(intervalBeingEdited.started_at)} から ${local(intervalBeingEdited.ended_at)} までの除外区間を削除します。該当時間がプレイ時間に加算され、元に戻せません。`}
              onconfirm={deleteEditingInterval}
            />{/if}
        </div>
        {#if intervalEditError}<p class="form-error" role="alert">{intervalEditError}</p>{/if}
      </form>
    </div>
  </div>{/if}
{#if newIntervalOpen && selected}<div class="modal history-entry-modal">
    <div
      class="panel editor history-entry-dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="new-interval-title"
    >
      <button
        class="close"
        aria-label="閉じる"
        disabled={intervalCreating}
        onclick={cancelAddInterval}>×</button
      >
      <h2 id="new-interval-title">除外区間を追加</h2>
      <p class="hint">
        セッション範囲：{local(selected.launched_at)} ～ {local(selected.exited_at)}
      </p>
      <form
        class="history-range-form"
        novalidate
        onsubmit={(event) => {
          event.preventDefault();
          addInterval();
        }}
      >
        <div class="history-range-fields">
          <DateTimeSelect
            label="開始日時"
            value={newIntervalStart}
            disabled={intervalCreating}
            invalid={Boolean(newIntervalError)}
            onchange={(value) => {
              newIntervalStart = value;
              newIntervalError = '';
            }}
          />
          <DateTimeSelect
            label="終了日時"
            value={newIntervalEnd}
            disabled={intervalCreating}
            invalid={Boolean(newIntervalError)}
            onchange={(value) => {
              newIntervalEnd = value;
              newIntervalError = '';
            }}
          />
        </div>
        <div class="actions">
          <button type="button" disabled={intervalCreating} onclick={cancelAddInterval}
            >キャンセル</button
          ><button class="primary" type="submit" disabled={intervalCreating}
            >{intervalCreating ? '追加中…' : '追加'}</button
          >
        </div>
        {#if newIntervalError}<p class="form-error" role="alert">{newIntervalError}</p>{/if}
      </form>
    </div>
  </div>{/if}
{#if pageError && !selected}<p class="error">{pageError}</p>{/if}
{#if selectedScreenshot}<div
    class="modal screenshot-modal"
    role="presentation"
    onclick={() => (selectedScreenshot = null)}
  >
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <section class="screenshot-viewer" onclick={(event) => event.stopPropagation()}>
      <button class="close" aria-label="閉じる" onclick={() => (selectedScreenshot = null)}
        >×</button
      >
      <img src={imageSrc(selectedScreenshot.path)} alt="スクリーンショット拡大表示" />
      <footer>
        <span
          >{local(selectedScreenshot.captured_at)} ・ {selectedScreenshot.width}×{selectedScreenshot.height}</span
        >
        <DeleteButton
          title="スクリーンショットの削除"
          message={`${local(selectedScreenshot.captured_at)} のスクリーンショットを削除します。画像ファイルも削除され、元に戻せません。`}
          onconfirm={() => deleteScreenshot(selectedScreenshot!.id)}
        />
      </footer>
    </section>
  </div>{/if}
{#if toast}<div class:error-toast={toastError} class="toast" role="status">
    <span>{toastError ? '!' : '✓'}</span>
    <p>{toast}</p>
    <button aria-label="通知を閉じる" onclick={() => (toast = '')}>×</button>
  </div>{/if}
