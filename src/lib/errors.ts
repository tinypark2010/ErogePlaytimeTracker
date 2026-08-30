export const GENERIC_ERROR_MESSAGE =
  '処理を完了できませんでした。しばらくしてからもう一度お試しください。';

const COMMAND_ERROR_PREFIX = 'ept:user-error:';

export class UserFacingError extends Error {
  override toString() {
    return this.message;
  }
}

export function normalizeCommandError(cause: unknown) {
  if (typeof cause === 'string' && cause.startsWith(COMMAND_ERROR_PREFIX)) {
    const message = cause.slice(COMMAND_ERROR_PREFIX.length).trim();
    if (message) return new UserFacingError(message);
  }
  return new UserFacingError(GENERIC_ERROR_MESSAGE);
}

export function userErrorMessage(cause: unknown, fallback = GENERIC_ERROR_MESSAGE) {
  return cause instanceof UserFacingError ? cause.message : fallback;
}
