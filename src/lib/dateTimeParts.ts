export interface DateTimeParts {
  year: string;
  month: string;
  day: string;
  hour: string;
  minute: string;
  second: string;
}

export type DateTimePart = keyof DateTimeParts;

export function emptyDateTimeParts(): DateTimeParts {
  return { year: '', month: '', day: '', hour: '', minute: '', second: '' };
}

export function parseDateTimeValue(value: string, withTime: boolean): DateTimeParts {
  if (!value) return emptyDateTimeParts();
  const pattern = withTime
    ? /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/
    : /^(\d{4})-(\d{2})-(\d{2})$/;
  const match = value.match(pattern);
  if (!match) return emptyDateTimeParts();
  return {
    year: match[1],
    month: match[2],
    day: match[3],
    hour: withTime ? match[4] : '',
    minute: withTime ? match[5] : '',
    second: withTime ? (match[6] ?? '00') : '',
  };
}

export function daysInMonth(year: string, month: string) {
  if (!year || !month) return 31;
  const numericYear = Number(year);
  const numericMonth = Number(month);
  if (!Number.isInteger(numericYear) || !Number.isInteger(numericMonth)) return 31;
  if (numericMonth < 1 || numericMonth > 12) return 31;
  return new Date(Date.UTC(numericYear, numericMonth, 0)).getUTCDate();
}

function twoDigits(value: string) {
  return value.padStart(2, '0');
}

export function updateDateTimePart(
  parts: DateTimeParts,
  part: DateTimePart,
  value: string,
): DateTimeParts {
  const next = { ...parts, [part]: value };
  if (next.day && next.year && next.month) {
    const selectedDay = Number(next.day);
    if (Number.isInteger(selectedDay)) {
      next.day = twoDigits(String(Math.min(selectedDay, daysInMonth(next.year, next.month))));
    }
  }
  return next;
}

export function formatDateTimeValue(parts: DateTimeParts, withTime: boolean) {
  if (!parts.year || !parts.month || !parts.day) return '';
  if (withTime && (!parts.hour || !parts.minute || !parts.second)) return '';

  const year = Number(parts.year);
  const month = Number(parts.month);
  const day = Number(parts.day);
  const hour = Number(parts.hour);
  const minute = Number(parts.minute);
  const second = Number(parts.second);
  if (
    !Number.isInteger(year) ||
    parts.year.length !== 4 ||
    !Number.isInteger(month) ||
    month < 1 ||
    month > 12 ||
    !Number.isInteger(day) ||
    day < 1 ||
    day > daysInMonth(parts.year, parts.month) ||
    (withTime &&
      (!Number.isInteger(hour) ||
        hour < 0 ||
        hour > 23 ||
        !Number.isInteger(minute) ||
        minute < 0 ||
        minute > 59 ||
        !Number.isInteger(second) ||
        second < 0 ||
        second > 59))
  ) {
    return '';
  }

  const date = `${parts.year}-${twoDigits(parts.month)}-${twoDigits(parts.day)}`;
  return withTime
    ? `${date}T${twoDigits(parts.hour)}:${twoDigits(parts.minute)}:${twoDigits(parts.second)}`
    : date;
}
