import { invoke } from '@tauri-apps/api/core';
import { normalizeCommandError } from './errors';
import type {
  BackgroundInterval,
  GameDetail,
  GameSummary,
  GameTimestamp,
  GameScreenshot,
  Metadata,
  Session,
  Settings,
  PlayStatus,
  SortKey,
  StatisticsPeriodInput,
  StatisticsReport,
  TrackingStatus,
} from './types';

function command<T>(name: string, args?: Record<string, unknown>) {
  return invoke<T>(name, args).catch((cause) => {
    throw normalizeCommandError(cause);
  });
}

export const api = {
  listGames: (
    search = '',
    brand = '',
    playStatus = '',
    sort: SortKey = 'last_played',
    descending = true,
  ) =>
    command<GameSummary[]>('list_games', {
      search,
      brand: brand || null,
      playStatus: playStatus || null,
      sort,
      descending,
    }),
  listBrands: () => command<string[]>('list_brands'),
  getGame: (id: number) => command<GameDetail>('get_game', { id }),
  createGame: (input: {
    title: string;
    brand?: string;
    release_date?: string;
    thumbnail_path?: string;
    erogamescape_id?: number;
    source_url?: string;
    executable_paths: string[];
  }) => command<number>('create_game', { input }),
  updateGame: (
    id: number,
    input: { title: string; brand?: string; release_date?: string; source_url?: string },
  ) => command<void>('update_game', { id, input }),
  updateGameThumbnail: (gameId: number, thumbnailPath: string | null) =>
    command<void>('update_game_thumbnail', { gameId, thumbnailPath }),
  updateGamePlayStatus: (id: number, status: PlayStatus) =>
    command<void>('update_game_play_status', { id, status }),
  openExternalUrl: (url: string) => command<void>('open_external_url', { url }),
  deleteGame: (id: number) => command<void>('delete_game', { id }),
  addExecutable: (gameId: number, path: string) =>
    command<void>('add_game_executable', { gameId, path }),
  removeExecutable: (id: number) => command<void>('remove_game_executable', { id }),
  launchGame: (gameId: number) => command<void>('launch_game', { gameId }),
  sessions: (gameId: number) => command<Session[]>('list_sessions', { gameId }),
  statistics: (period: StatisticsPeriodInput) =>
    command<StatisticsReport>('get_statistics', { period }),
  intervals: (sessionId: number) =>
    command<BackgroundInterval[]>('list_background_intervals', { sessionId }),
  manualSession: (gameId: number, start: string, end: string) =>
    command<number>('create_manual_session', { gameId, start, end }),
  updateSession: (id: number, start: string, end: string | null) =>
    command<void>('update_session', { id, start, end }),
  confirmSessionReview: (id: number) => command<void>('confirm_session_review', { id }),
  deleteSession: (id: number) => command<void>('delete_session', { id }),
  deleteAllSessions: (gameId: number) => command<number>('delete_all_sessions', { gameId }),
  createInterval: (sessionId: number, start: string, end: string) =>
    command<number>('create_background_interval', { sessionId, start, end }),
  updateInterval: (id: number, start: string, end: string | null) =>
    command<void>('update_background_interval', { id, start, end }),
  deleteInterval: (id: number) => command<void>('delete_background_interval', { id }),
  timestamps: (gameId: number) => command<GameTimestamp[]>('list_game_timestamps', { gameId }),
  createTimestamp: (gameId: number, name: string) =>
    command<number>('create_game_timestamp', { gameId, name }),
  updateTimestamp: (id: number, name: string, markedAt: string) =>
    command<void>('update_game_timestamp', { id, name, markedAt }),
  deleteTimestamp: (id: number) => command<void>('delete_game_timestamp', { id }),
  screenshots: (gameId: number) => command<GameScreenshot[]>('list_game_screenshots', { gameId }),
  deleteScreenshot: (id: number) => command<void>('delete_game_screenshot', { id }),
  openScreenshotDirectory: (gameId: number) =>
    command<void>('open_screenshot_directory', { gameId }),
  fetchMetadata: (value: string) => command<Metadata>('fetch_erogamescape_metadata', { value }),
  importThumbnail: (path: string) => command<string>('import_thumbnail', { path }),
  saveCroppedThumbnail: (pngBase64: string) =>
    command<string>('save_cropped_thumbnail', { pngBase64 }),
  refreshMetadata: (gameId: number) => command<void>('refresh_game_metadata', { gameId }),
  settings: () => command<Settings>('get_settings'),
  updateSettings: (settings: Settings) => command<void>('update_settings', { settings }),
  skipUpdateVersion: (version: string) => command<void>('skip_update_version', { version }),
  validateScreenshotHotkey: (hotkey: string) =>
    command<void>('validate_screenshot_hotkey', { hotkey }),
  suspendScreenshotHotkey: () => command<void>('suspend_screenshot_hotkey'),
  resumeScreenshotHotkey: () => command<void>('resume_screenshot_hotkey'),
  status: () => command<TrackingStatus>('get_tracking_status'),
};
