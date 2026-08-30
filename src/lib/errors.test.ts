import { describe, expect, it } from 'vitest';
import {
  GENERIC_ERROR_MESSAGE,
  UserFacingError,
  normalizeCommandError,
  userErrorMessage,
} from './errors';

describe('user-facing errors', () => {
  it('accepts only errors marked safe by the command boundary', () => {
    const error = normalizeCommandError('ept:user-error:入力内容を確認してください。');
    expect(error).toBeInstanceOf(UserFacingError);
    expect(String(error)).toBe('入力内容を確認してください。');
  });

  it('replaces unmarked command errors with a generic message', () => {
    const error = normalizeCommandError('UNIQUE constraint failed: game_executables.path');
    expect(String(error)).toBe(GENERIC_ERROR_MESSAGE);
  });

  it('does not expose unexpected plugin or JavaScript errors', () => {
    expect(
      userErrorMessage(new Error('private implementation detail'), '更新できませんでした。'),
    ).toBe('更新できませんでした。');
  });
});
