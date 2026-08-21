export type SortKey =
  | 'title'
  | 'brand'
  | 'release_date'
  | 'created_at'
  | 'total_playtime'
  | 'last_played'
  | 'session_count';
export type PlayStatus = 'unplayed' | 'playing' | 'completed' | 'retired';
export interface GameSummary {
  id: number;
  title: string;
  brand: string | null;
  release_date: string | null;
  thumbnail_path: string | null;
  created_at: string;
  total_playtime_seconds: number;
  total_running_seconds: number;
  last_played: string | null;
  session_count: number;
  play_status: PlayStatus;
}
export interface Executable {
  id: number;
  game_id: number;
  path: string;
  file_name: string;
  created_at: string;
}
export interface GameDetail extends GameSummary {
  erogamescape_id: number | null;
  source_url: string | null;
  executables: Executable[];
}
export interface Session {
  id: number;
  game_id: number;
  launched_at: string;
  exited_at: string | null;
  needs_review: boolean;
  playtime_seconds: number;
  background_seconds: number;
  running_seconds: number | null;
}
export interface BackgroundInterval {
  id: number;
  play_session_id: number;
  started_at: string;
  ended_at: string | null;
}
export interface GameTimestamp {
  id: number;
  game_id: number;
  name: string;
  marked_at: string;
  playtime_seconds: number;
  since_previous_seconds: number;
}
export interface Metadata {
  erogamescape_id: number;
  title: string;
  brand: string | null;
  release_date: string | null;
  thumbnail_url: string | null;
  source_url: string;
}
export type Theme = 'dark' | 'light' | 'pink' | 'blue';
export interface Settings {
  autostart: boolean;
  reconciliation_seconds: number;
  close_to_tray: boolean;
  theme: Theme;
}
export interface TrackingStatus {
  games: Array<{
    game_id: number;
    title: string;
    session_id: number;
    phase: 'starting' | 'foreground' | 'background' | 'window_transition';
  }>;
}
