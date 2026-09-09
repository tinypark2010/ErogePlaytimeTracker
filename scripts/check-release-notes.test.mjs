import { describe, expect, it } from 'vitest';
import { validateReleaseNotes } from './check-release-notes.mjs';

const catalog = (releases) => ({ schema_version: 1, releases });
const release = (version, changes = []) => ({ version, changes });

describe('bundled user-facing release notes', () => {
  it('accepts explicit internal-only releases and Japanese bullet items', () => {
    expect(() =>
      validateReleaseNotes(
        catalog([release('0.1.11'), release('0.1.10', ['機能を追加しました。'])]),
        '0.1.11',
        '0.1.11',
      ),
    ).not.toThrow();
  });

  it('allows bootstrap development before the first notes release, but blocks publication', () => {
    expect(() => validateReleaseNotes(catalog([]), '0.1.11')).not.toThrow();
    expect(() => validateReleaseNotes(catalog([]), '0.1.11', '0.1.11')).toThrow('Missing');
  });

  it('requires notes for the exact release target, not an older entry', () => {
    expect(() => validateReleaseNotes(catalog([release('0.1.10')]), '0.1.11', '0.1.11')).toThrow(
      'Missing',
    );
    expect(() => validateReleaseNotes(catalog([release('0.1.11')]), '0.1.11', '0.1.12')).toThrow(
      'target',
    );
  });

  it.each([
    null,
    {},
    [],
    { schema_version: 2, releases: [] },
    { schema_version: 1, releases: [], extra: true },
    catalog([null]),
    catalog([{ version: '0.1.11' }]),
    catalog([{ version: '0.1.11', changes: null }]),
    catalog([{ version: '0.1.11', changes: [], body: 'unexpected' }]),
  ])('rejects a malformed catalog: %j', (input) => {
    expect(() => validateReleaseNotes(input, '0.1.11')).toThrow();
  });

  it.each(['', ' ', ' padded ', 'two\nlines', 1, null])(
    'rejects invalid bullet items: %j',
    (item) => {
      expect(() => validateReleaseNotes(catalog([release('0.1.11', [item])]), '0.1.11')).toThrow();
    },
  );

  it.each([
    'v0.1.11',
    '0.1',
    '00.1.11',
    '0.1.11-beta.1',
    '0.1.11+build',
    '18446744073709551616.0.0',
  ])('rejects unsupported versions: %s', (version) => {
    expect(() => validateReleaseNotes(catalog([release(version)]), '0.1.11')).toThrow();
  });

  it('compares numerically and rejects duplicate, unordered, or future releases', () => {
    expect(() =>
      validateReleaseNotes(catalog([release('0.1.10'), release('0.1.9')]), '0.1.11'),
    ).not.toThrow();
    for (const versions of [['0.1.10', '0.1.10'], ['0.1.9', '0.1.10'], ['0.1.12']]) {
      expect(() =>
        validateReleaseNotes(catalog(versions.map((version) => release(version))), '0.1.11'),
      ).toThrow();
    }
  });
});
