import type { StatisticsDay } from './types';

export interface DailyTrendPoint extends StatisticsDay {
  cumulative_seconds: number;
}

export function compactDuration(seconds: number) {
  const totalMinutes = Math.floor(Math.max(0, seconds) / 60);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  if (hours) return minutes ? `${hours}時間 ${minutes}分` : `${hours}時間`;
  if (totalMinutes) return `${totalMinutes}分`;
  return seconds > 0 ? `${Math.floor(seconds)}秒` : '0分';
}

export function formatDateKey(value: string) {
  const [year, month, day] = value.split('-').map(Number);
  return `${year}年${month}月${day}日`;
}

export function dailyTrend(days: StatisticsDay[]): DailyTrendPoint[] {
  let cumulative = 0;
  return days.map((day) => {
    cumulative += day.playtime_seconds;
    return { ...day, cumulative_seconds: cumulative };
  });
}

export function nextChartZoom(current: number, maximum: number, deltaY: number, deltaMode: number) {
  const multiplier = deltaMode === 1 ? 16 : deltaMode === 2 ? 100 : 1;
  const delta = deltaY * multiplier;
  if (!delta) return current;
  const exponent =
    Math.abs(delta) < 20 ? Math.abs(delta) * 0.015 : Math.max(0.3, Math.abs(delta) * 0.0015);
  const factor = Math.exp(-Math.sign(delta) * exponent);
  return Math.min(maximum, Math.max(1, current * factor));
}
