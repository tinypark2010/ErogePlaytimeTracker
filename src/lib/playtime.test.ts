import { describe, expect, it } from 'vitest';
import { backgroundIntervalDescription } from './playtime';

describe('playtime calculation descriptions', () => {
  it('explains subtraction when background time is excluded', () => {
    expect(backgroundIntervalDescription(true)).toContain('プレイ時間から除外します');
  });

  it('explains inclusion when exclusion is disabled', () => {
    expect(backgroundIntervalDescription(false)).toContain('プレイ時間に含めます');
  });
});
