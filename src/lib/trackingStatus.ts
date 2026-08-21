import type { TrackingStatus } from './types';

export type TrackingGame = TrackingStatus['games'][number];
export type TrackingPhase = TrackingGame['phase'];
export interface TrackingStatusGroup {
  phase: TrackingPhase;
  games: TrackingGame[];
}

const phaseOrder: TrackingPhase[] = ['foreground', 'background', 'starting', 'window_transition'];

export function trackingStatusGroups(status: TrackingStatus): TrackingStatusGroup[] {
  return phaseOrder
    .map((phase) => ({ phase, games: status.games.filter((game) => game.phase === phase) }))
    .filter((group) => group.games.length > 0);
}

export function trackingStatusText(group: TrackingStatusGroup): string {
  const { phase, games } = group;
  const title = games[0]?.title || 'ゲーム';
  if (phase === 'foreground')
    return games.length === 1 ? `${title}をプレイ中` : `${games.length}本をプレイ中`;
  if (phase === 'background')
    return games.length === 1 ? `${title}：バックグラウンド` : `バックグラウンド ${games.length}本`;
  if (phase === 'starting')
    return games.length === 1 ? `${title}を起動中` : `${games.length}本を起動中`;
  return games.length === 1
    ? `${title}：画面切替・終了処理中`
    : `画面切替・終了処理中 ${games.length}本`;
}
