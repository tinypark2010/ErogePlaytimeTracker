import { describe, expect, it } from 'vitest';
import {
  advanceChartInertia,
  chartAxisTicks,
  chartPanScrollLeft,
  compactDuration,
  dailyTrend,
  nextChartZoom,
} from './statistics';
import type { StatisticsDay } from './types';

function day(date: string, playtime_seconds: number): StatisticsDay {
  return { date, playtime_seconds, games: [] };
}

describe('statistics helpers', () => {
  it('formats compact durations', () => {
    expect(compactDuration(0)).toBe('0分');
    expect(compactDuration(42)).toBe('42秒');
    expect(compactDuration(3_900)).toBe('1時間 5分');
  });

  it('keeps daily values and calculates their cumulative total', () => {
    const points = dailyTrend([
      day('2026-01-01', 60),
      day('2026-01-02', 120),
      day('2026-02-01', 300),
    ]);
    expect(points).toEqual([
      {
        date: '2026-01-01',
        games: [],
        playtime_seconds: 60,
        cumulative_seconds: 60,
      },
      {
        date: '2026-01-02',
        games: [],
        playtime_seconds: 120,
        cumulative_seconds: 180,
      },
      {
        date: '2026-02-01',
        games: [],
        playtime_seconds: 300,
        cumulative_seconds: 480,
      },
    ]);
  });

  it('builds evenly spaced duration ticks from zero to the chart maximum', () => {
    expect(chartAxisTicks(10_800)).toEqual([
      { ratio: 0, value: 0 },
      { ratio: 1 / 3, value: 3_600 },
      { ratio: 2 / 3, value: 7_200 },
      { ratio: 1, value: 10_800 },
    ]);
    expect(chartAxisTicks(0).map((tick) => tick.value)).toEqual([0, 20, 40, 60]);
  });

  it('zooms the chart in when scrolling up and leaves minimum zoom unchanged', () => {
    const zoomedIn = nextChartZoom(1, 8, -120, 0);
    expect(zoomedIn).toBeGreaterThan(1);
    expect(nextChartZoom(zoomedIn, 8, 120, 0)).toBeCloseTo(1);
    expect(nextChartZoom(1, 8, 120, 0)).toBe(1);
  });

  it('moves a zoomed chart with the pointer and keeps the scroll position in range', () => {
    expect(chartPanScrollLeft(300, 80, 600)).toBe(220);
    expect(chartPanScrollLeft(300, -80, 600)).toBe(380);
    expect(chartPanScrollLeft(30, 80, 600)).toBe(0);
    expect(chartPanScrollLeft(580, -80, 600)).toBe(600);
  });

  it('continues chart movement with decreasing velocity and stops at an edge', () => {
    const next = advanceChartInertia(200, 1, 16, 600);
    expect(next.scrollLeft).toBeGreaterThan(200);
    expect(next.velocity).toBeGreaterThan(0);
    expect(next.velocity).toBeLessThan(1);

    expect(advanceChartInertia(595, 1, 16, 600)).toEqual({
      scrollLeft: 600,
      velocity: 0,
    });
  });
});
