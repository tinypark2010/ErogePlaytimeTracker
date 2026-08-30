export interface HistoryRange {
  id?: number;
  start: string;
  end: string | null;
}

function timestamp(value: string) {
  const parsed = Date.parse(value);
  return Number.isFinite(parsed) ? parsed : null;
}

function validateRange(start: string, end: string, emptyMessage: string) {
  if (!start || !end) return emptyMessage;
  const startMs = timestamp(start);
  const endMs = timestamp(end);
  if (startMs === null || endMs === null) return '日時を正しく入力してください。';
  if (endMs < startMs) return '終了日時は開始日時以降にしてください。';
  return '';
}

function overlapsExistingRange(
  startMs: number,
  endMs: number,
  ranges: HistoryRange[],
  excludedId?: number,
) {
  return ranges.some((range) => {
    if (range.id === excludedId) return false;
    const rangeStartMs = timestamp(range.start);
    const rangeEndMs = range.end ? timestamp(range.end) : Number.POSITIVE_INFINITY;
    return (
      rangeStartMs !== null && rangeEndMs !== null && endMs > rangeStartMs && rangeEndMs > startMs
    );
  });
}

export function validateManualSession(start: string, end: string, sessions: HistoryRange[] = []) {
  const rangeError = validateRange(start, end, '開始日時と終了日時を入力してください。');
  if (rangeError) return rangeError;
  return overlapsExistingRange(timestamp(start)!, timestamp(end)!, sessions)
    ? '既存のセッションと重複しない日時を入力してください。'
    : '';
}

export function validateSessionEdit(
  start: string,
  end: string,
  intervals: HistoryRange[],
  sessions: HistoryRange[] = [],
  excludedSessionId?: number,
) {
  if (!start || !end) return '開始日時と終了日時を入力してください。';
  const startMs = timestamp(start);
  const endMs = timestamp(end);
  if (startMs === null || endMs === null) return '日時を正しく入力してください。';
  if (endMs < startMs) return '終了日時は開始日時以降にしてください。';

  const intervalOutsideSession = intervals.some((interval) => {
    const intervalStartMs = timestamp(interval.start);
    const intervalEndMs = interval.end ? timestamp(interval.end) : null;
    return (
      intervalStartMs === null ||
      intervalStartMs < startMs ||
      intervalEndMs === null ||
      intervalEndMs > endMs
    );
  });
  if (intervalOutsideSession) {
    return '開始・終了日時には、すべての除外区間を含む範囲を指定してください。';
  }
  return overlapsExistingRange(startMs, endMs, sessions, excludedSessionId)
    ? '既存のセッションと重複しない日時を入力してください。'
    : '';
}

export function validateRunningSessionEdit(
  start: string,
  intervals: HistoryRange[],
  sessions: HistoryRange[] = [],
  excludedSessionId?: number,
) {
  if (!start) return '開始日時を入力してください。';
  const startMs = timestamp(start);
  if (startMs === null) return '日時を正しく入力してください。';
  const intervalOutsideSession = intervals.some((interval) => {
    const intervalStartMs = timestamp(interval.start);
    return intervalStartMs === null || intervalStartMs < startMs;
  });
  if (intervalOutsideSession) {
    return '開始日時には、すべての除外区間を含む範囲を指定してください。';
  }
  return overlapsExistingRange(startMs, Number.POSITIVE_INFINITY, sessions, excludedSessionId)
    ? '既存のセッションと重複しない日時を入力してください。'
    : '';
}

export function validateBackgroundInterval(
  start: string,
  end: string | null,
  session: HistoryRange,
  intervals: HistoryRange[],
  excludedIntervalId?: number,
) {
  if (end === null) {
    if (!start) return '除外区間の開始日時を入力してください。';
    if (timestamp(start) === null) return '日時を正しく入力してください。';
  } else {
    const rangeError = validateRange(
      start,
      end,
      '除外区間の開始日時と終了日時を入力してください。',
    );
    if (rangeError) return rangeError;
  }

  const startMs = timestamp(start)!;
  const endMs = end === null ? Number.POSITIVE_INFINITY : timestamp(end)!;
  const sessionStartMs = timestamp(session.start);
  const sessionEndMs = session.end ? timestamp(session.end) : null;
  if (
    sessionStartMs === null ||
    startMs < sessionStartMs ||
    (sessionEndMs !== null && (end === null || endMs > sessionEndMs))
  ) {
    return '除外区間はセッションの開始・終了日時の範囲内で入力してください。';
  }

  const overlaps = overlapsExistingRange(startMs, endMs, intervals, excludedIntervalId);
  return overlaps ? '既存の除外区間と重複しない日時を入力してください。' : '';
}
