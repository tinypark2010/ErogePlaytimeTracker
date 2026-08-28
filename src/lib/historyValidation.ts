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

export function validateManualSession(start: string, end: string) {
  return validateRange(start, end, '開始日時と終了日時を入力してください。');
}

export function validateSessionEdit(start: string, end: string, intervals: HistoryRange[]) {
  if (!start) return '開始日時を入力してください。';
  const startMs = timestamp(start);
  const endMs = end ? timestamp(end) : null;
  if (startMs === null || (end && endMs === null)) return '日時を正しく入力してください。';
  if (endMs !== null && endMs < startMs) return '終了日時は開始日時以降にしてください。';

  const intervalOutsideSession = intervals.some((interval) => {
    const intervalStartMs = timestamp(interval.start);
    const intervalEndMs = interval.end ? timestamp(interval.end) : null;
    return (
      intervalStartMs === null ||
      intervalStartMs < startMs ||
      (endMs !== null && (intervalEndMs === null || intervalEndMs > endMs))
    );
  });
  return intervalOutsideSession
    ? '開始・終了日時には、すべての除外区間を含む範囲を指定してください。'
    : '';
}

export function validateBackgroundInterval(
  start: string,
  end: string,
  session: HistoryRange,
  intervals: HistoryRange[],
  excludedIntervalId?: number,
) {
  const rangeError = validateRange(start, end, '除外区間の開始日時と終了日時を入力してください。');
  if (rangeError) return rangeError;

  const startMs = timestamp(start)!;
  const endMs = timestamp(end)!;
  const sessionStartMs = timestamp(session.start);
  const sessionEndMs = session.end ? timestamp(session.end) : null;
  if (
    sessionStartMs === null ||
    startMs < sessionStartMs ||
    (sessionEndMs !== null && endMs > sessionEndMs)
  ) {
    return '除外区間はセッションの開始・終了日時の範囲内で入力してください。';
  }

  const overlaps = intervals.some((interval) => {
    if (interval.id === excludedIntervalId) return false;
    const intervalStartMs = timestamp(interval.start);
    const intervalEndMs = interval.end ? timestamp(interval.end) : Number.POSITIVE_INFINITY;
    return (
      intervalStartMs !== null &&
      intervalEndMs !== null &&
      endMs > intervalStartMs &&
      intervalEndMs > startMs
    );
  });
  return overlaps ? '既存の除外区間と重複しない日時を入力してください。' : '';
}
