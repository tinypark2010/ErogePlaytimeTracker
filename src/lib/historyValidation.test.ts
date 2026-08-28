import { describe, expect, it } from 'vitest';
import {
  validateBackgroundInterval,
  validateManualSession,
  validateRunningSessionEdit,
  validateSessionEdit,
} from './historyValidation';

describe('validateManualSession', () => {
  it('requires both timestamps and rejects a reversed range', () => {
    expect(validateManualSession('', '')).toBe('開始日時と終了日時を入力してください。');
    expect(validateManualSession('2026-08-29T11:00:00Z', '2026-08-29T10:00:00Z')).toBe(
      '終了日時は開始日時以降にしてください。',
    );
  });

  it('accepts a valid range', () => {
    expect(validateManualSession('2026-08-29T10:00:00Z', '2026-08-29T11:00:00Z')).toBe('');
  });
});

describe('validateRunningSessionEdit', () => {
  it('requires a valid start containing every interval', () => {
    const intervals = [{ id: 1, start: '2026-08-29T10:15:00Z', end: '2026-08-29T10:30:00Z' }];
    expect(validateRunningSessionEdit('', intervals)).toBe('開始日時を入力してください。');
    expect(validateRunningSessionEdit('2026-08-29T10:20:00Z', intervals)).toBe(
      '開始日時には、すべての除外区間を含む範囲を指定してください。',
    );
    expect(validateRunningSessionEdit('2026-08-29T10:00:00Z', intervals)).toBe('');
  });
});

describe('validateSessionEdit', () => {
  const intervals = [
    {
      id: 1,
      start: '2026-08-29T10:15:00Z',
      end: '2026-08-29T10:30:00Z',
    },
  ];

  it('requires both session timestamps', () => {
    expect(validateSessionEdit('2026-08-29T10:00:00Z', '', intervals)).toBe(
      '開始日時と終了日時を入力してください。',
    );
  });

  it('keeps existing intervals inside edited session bounds', () => {
    expect(validateSessionEdit('2026-08-29T10:20:00Z', '2026-08-29T11:00:00Z', intervals)).toBe(
      '開始・終了日時には、すべての除外区間を含む範囲を指定してください。',
    );
    expect(validateSessionEdit('2026-08-29T10:00:00Z', '2026-08-29T10:20:00Z', intervals)).toBe(
      '開始・終了日時には、すべての除外区間を含む範囲を指定してください。',
    );
  });

  it('accepts bounds containing every interval', () => {
    expect(validateSessionEdit('2026-08-29T10:00:00Z', '2026-08-29T11:00:00Z', intervals)).toBe('');
  });
});

describe('validateBackgroundInterval', () => {
  const session = {
    start: '2026-08-29T10:00:00Z',
    end: '2026-08-29T12:00:00Z',
  };
  const intervals = [
    {
      id: 1,
      start: '2026-08-29T10:30:00Z',
      end: '2026-08-29T11:00:00Z',
    },
  ];

  it('rejects ranges outside the session', () => {
    expect(
      validateBackgroundInterval(
        '2026-08-29T09:59:59Z',
        '2026-08-29T10:15:00Z',
        session,
        intervals,
      ),
    ).toBe('除外区間はセッションの開始・終了日時の範囲内で入力してください。');
  });

  it('rejects overlap but allows touching interval boundaries', () => {
    expect(
      validateBackgroundInterval(
        '2026-08-29T10:45:00Z',
        '2026-08-29T11:15:00Z',
        session,
        intervals,
      ),
    ).toBe('既存の除外区間と重複しない日時を入力してください。');
    expect(
      validateBackgroundInterval(
        '2026-08-29T11:00:00Z',
        '2026-08-29T11:15:00Z',
        session,
        intervals,
      ),
    ).toBe('');
  });

  it('validates an open interval without requiring an end timestamp', () => {
    expect(
      validateBackgroundInterval(
        '2026-08-29T11:00:00Z',
        null,
        { start: session.start, end: null },
        intervals,
      ),
    ).toBe('');
    expect(
      validateBackgroundInterval(
        '2026-08-29T10:45:00Z',
        null,
        { start: session.start, end: null },
        intervals,
      ),
    ).toBe('既存の除外区間と重複しない日時を入力してください。');
  });

  it('ignores the edited interval when checking overlap', () => {
    expect(
      validateBackgroundInterval(
        intervals[0].start,
        intervals[0].end!,
        session,
        intervals,
        intervals[0].id,
      ),
    ).toBe('');
  });
});
