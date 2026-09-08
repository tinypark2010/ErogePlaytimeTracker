import { describe, expect, it } from 'vitest';
import { screenshotNavigation } from './screenshotNavigation';

describe('screenshotNavigation', () => {
  const screenshots = [{ id: 30 }, { id: 20 }, { id: 10 }];

  it('follows the screenshot list order in both directions', () => {
    expect(screenshotNavigation(screenshots, 20)).toEqual({
      index: 1,
      previous: screenshots[0],
      next: screenshots[2],
    });
  });

  it('stops at either end without wrapping', () => {
    expect(screenshotNavigation(screenshots, 30)).toEqual({
      index: 0,
      previous: null,
      next: screenshots[1],
    });
    expect(screenshotNavigation(screenshots, 10)).toEqual({
      index: 2,
      previous: screenshots[1],
      next: null,
    });
    expect(screenshotNavigation([{ id: 30 }], 30)).toEqual({
      index: 0,
      previous: null,
      next: null,
    });
  });

  it('can move across a ten-item gallery page boundary', () => {
    const allScreenshots = Array.from({ length: 12 }, (_, index) => ({ id: 12 - index }));
    expect(screenshotNavigation(allScreenshots, 3).next).toBe(allScreenshots[10]);
    expect(screenshotNavigation(allScreenshots, 2).previous).toBe(allScreenshots[9]);
  });

  it('keeps the selected image anchored by ID when new screenshots arrive', () => {
    const refreshed = [{ id: 40 }, ...screenshots.map((shot) => ({ ...shot }))];
    expect(screenshotNavigation(refreshed, 20)).toEqual({
      index: 2,
      previous: refreshed[1],
      next: refreshed[3],
    });
  });

  it('has no navigation when the selection is missing or removed', () => {
    for (const [items, selectedId] of [
      [[], 20],
      [screenshots, null],
      [screenshots, 99],
    ] as const) {
      expect(screenshotNavigation(items, selectedId)).toEqual({
        index: -1,
        previous: null,
        next: null,
      });
    }
  });
});
