import { describe, expect, it } from 'vitest';
import {
  daysInMonth,
  emptyDateTimeParts,
  formatDateTimeValue,
  parseDateTimeValue,
  updateDateTimePart,
} from './dateTimeParts';

describe('dateTimeParts', () => {
  it('parses and formats date and datetime values', () => {
    expect(parseDateTimeValue('2026-08-29', false)).toEqual({
      year: '2026',
      month: '08',
      day: '29',
      hour: '',
      minute: '',
      second: '',
    });
    expect(formatDateTimeValue(parseDateTimeValue('2026-08-29T21:04:03', true), true)).toBe(
      '2026-08-29T21:04:03',
    );
  });

  it('defaults omitted seconds to zero', () => {
    expect(formatDateTimeValue(parseDateTimeValue('2026-08-29T21:04', true), true)).toBe(
      '2026-08-29T21:04:00',
    );
  });

  it('does not emit a value until every required part is selected', () => {
    const parts = { ...emptyDateTimeParts(), year: '2026', month: '08', day: '29' };
    expect(formatDateTimeValue(parts, true)).toBe('');
    expect(formatDateTimeValue(parts, false)).toBe('2026-08-29');
  });

  it('uses leap-year month lengths', () => {
    expect(daysInMonth('', '02')).toBe(31);
    expect(daysInMonth('2024', '02')).toBe(29);
    expect(daysInMonth('2025', '02')).toBe(28);
    expect(daysInMonth('2026', '04')).toBe(30);
  });

  it('clamps the day when the selected month is shorter', () => {
    const parts = parseDateTimeValue('2026-01-31T12:00:00', true);
    expect(updateDateTimePart(parts, 'month', '02').day).toBe('28');
  });

  it('rejects impossible values', () => {
    expect(
      formatDateTimeValue(
        { year: '2026', month: '02', day: '29', hour: '12', minute: '00', second: '00' },
        true,
      ),
    ).toBe('');
    expect(
      formatDateTimeValue(
        { year: '2024', month: '02', day: '29', hour: '24', minute: '00', second: '00' },
        true,
      ),
    ).toBe('');
    expect(
      formatDateTimeValue(
        { year: '2024', month: '2.5', day: '01', hour: '12', minute: '00', second: '00' },
        true,
      ),
    ).toBe('');
  });
});
