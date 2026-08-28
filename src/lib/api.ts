import { invoke } from '@tauri-apps/api/core';
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
  TrackingStatus,
} from './types';
export const api = {
  listGames: (
    search = '',
    brand = '',
    playStatus = '',
    sort: SortKey = 'last_played',
    descending = true,
  ) =>
    invoke<GameSummary[]>('list_games', {
      search,
      brand: brand || null,
      playStatus: playStatus || null,
      sort,
      descending,
    }),
  listBrands: () => invoke<string[]>('list_brands'),
  systemFonts: () => invoke<string[]>('list_system_fonts'),
  getGame: (id: number) => invoke<GameDetail>('get_game', { id }),
  createGame: (input: {
    title: string;
    brand?: string;
    release_date?: string;
    thumbnail_path?: string;
    erogamescape_id?: number;
    source_url?: string;
    executable_paths: string[];
  }) => invoke<number>('create_game', { input }),
  updateGame: (
    id: number,
    input: { title: string; brand?: string; release_date?: string; source_url?: string },
  ) => invoke<void>('update_game', { id, input }),
  updateGameThumbnail: (gameId: number, thumbnailPath: string | null) =>
    invoke<void>('update_game_thumbnail', { gameId, thumbnailPath }),
  updateGamePlayStatus: (id: number, status: PlayStatus) =>
    invoke<void>('update_game_play_status', { id, status }),
  openExternalUrl: (url: string) => invoke<void>('open_external_url', { url }),
  deleteGame: (id: number) => invoke<void>('delete_game', { id }),
  addExecutable: (gameId: number, path: string) =>
    invoke<void>('add_game_executable', { gameId, path }),
  removeExecutable: (id: number) => invoke<void>('remove_game_executable', { id }),
  launchGame: (gameId: number) => invoke<void>('launch_game', { gameId }),
  sessions: (gameId: number) => invoke<Session[]>('list_sessions', { gameId }),
  intervals: (sessionId: number) =>
    invoke<BackgroundInterval[]>('list_background_intervals', { sessionId }),
  manualSession: (gameId: number, start: string, end: string) =>
    invoke<number>('create_manual_session', { gameId, start, end }),
  updateSession: (id: number, start: string, end: string | null) =>
    invoke<void>('update_session', { id, start, end }),
  deleteSession: (id: number) => invoke<void>('delete_session', { id }),
  deleteAllSessions: (gameId: number) => invoke<number>('delete_all_sessions', { gameId }),
  createInterval: (sessionId: number, start: string, end: string) =>
    invoke<number>('create_background_interval', { sessionId, start, end }),
  updateInterval: (id: number, start: string, end: string | null) =>
    invoke<void>('update_background_interval', { id, start, end }),
  deleteInterval: (id: number) => invoke<void>('delete_background_interval', { id }),
  timestamps: (gameId: number) => invoke<GameTimestamp[]>('list_game_timestamps', { gameId }),
  createTimestamp: (gameId: number, name: string) =>
    invoke<number>('create_game_timestamp', { gameId, name }),
  updateTimestampName: (id: number, name: string) =>
    invoke<void>('update_game_timestamp_name', { id, name }),
  deleteTimestamp: (id: number) => invoke<void>('delete_game_timestamp', { id }),
  screenshots: (gameId: number) => invoke<GameScreenshot[]>('list_game_screenshots', { gameId }),
  deleteScreenshot: (id: number) => invoke<void>('delete_game_screenshot', { id }),
  openScreenshotDirectory: (gameId: number) =>
    invoke<void>('open_screenshot_directory', { gameId }),
  saveSocialImage: (gameId: number, pngBase64: string) =>
    invoke<string>('save_social_image', { gameId, pngBase64 }),
  openSocialImageDirectory: (gameId: number) =>
    invoke<void>('open_social_image_directory', { gameId }),
  fetchMetadata: (value: string) => invoke<Metadata>('fetch_erogamescape_metadata', { value }),
  importThumbnail: (path: string) => invoke<string>('import_thumbnail', { path }),
  saveCroppedThumbnail: (pngBase64: string) =>
    invoke<string>('save_cropped_thumbnail', { pngBase64 }),
  refreshMetadata: (gameId: number) => invoke<void>('refresh_game_metadata', { gameId }),
  settings: () => invoke<Settings>('get_settings'),
  updateSettings: (settings: Settings) => invoke<void>('update_settings', { settings }),
  skipUpdateVersion: (version: string) => invoke<void>('skip_update_version', { version }),
  validateScreenshotHotkey: (hotkey: string) =>
    invoke<void>('validate_screenshot_hotkey', { hotkey }),
  suspendScreenshotHotkey: () => invoke<void>('suspend_screenshot_hotkey'),
  resumeScreenshotHotkey: () => invoke<void>('resume_screenshot_hotkey'),
  status: () => invoke<TrackingStatus>('get_tracking_status'),
};
