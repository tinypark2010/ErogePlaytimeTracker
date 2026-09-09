import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';

function versionParts(version) {
  if (typeof version !== 'string' || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
    throw new Error('Release notes require stable MAJOR.MINOR.PATCH versions.');
  }
  const parts = version.split('.').map(BigInt);
  if (parts.some((part) => part > 18446744073709551615n)) {
    throw new Error('Release notes version is out of range.');
  }
  return parts;
}

function compareVersions(left, right) {
  const a = versionParts(left);
  const b = versionParts(right);
  for (let index = 0; index < 3; index++) {
    if (a[index] !== b[index]) return a[index] > b[index] ? 1 : -1;
  }
  return 0;
}

function hasKeys(value, keys) {
  return (
    value &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    Object.keys(value).length === keys.length &&
    keys.every((key) => Object.hasOwn(value, key))
  );
}

export function validateReleaseNotes(catalog, currentVersion, releaseVersion) {
  versionParts(currentVersion);
  if (
    !hasKeys(catalog, ['schema_version', 'releases']) ||
    catalog.schema_version !== 1 ||
    !Array.isArray(catalog.releases)
  ) {
    throw new Error('Invalid release-notes/ja.json schema.');
  }
  let previous;
  for (const release of catalog.releases) {
    if (!hasKeys(release, ['version', 'changes']) || !Array.isArray(release.changes)) {
      throw new Error('Every release must explicitly specify version and changes (including []).');
    }
    if (compareVersions(release.version, currentVersion) > 0) {
      throw new Error('Release notes cannot contain a version newer than the application.');
    }
    if (previous && compareVersions(previous, release.version) <= 0) {
      throw new Error('Release notes versions must be unique and sorted newest first.');
    }
    previous = release.version;
    if (
      release.changes.some(
        (text) =>
          typeof text !== 'string' || !text.trim() || text !== text.trim() || /[\r\n]/.test(text),
      )
    ) {
      throw new Error('Each change must be a nonempty, trimmed, single-line string.');
    }
  }
  if (releaseVersion !== undefined) {
    if (compareVersions(releaseVersion, currentVersion) !== 0) {
      throw new Error('Release notes target does not match the application version.');
    }
    if (!catalog.releases.some((release) => release.version === releaseVersion)) {
      throw new Error(
        `Missing release notes for ${releaseVersion}; use changes: [] for an internal-only release.`,
      );
    }
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const { values } = parseArgs({ options: { version: { type: 'string' } } });
    const root = new URL('../', import.meta.url);
    const catalog = JSON.parse(readFileSync(new URL('release-notes/ja.json', root), 'utf8'));
    const { version } = JSON.parse(readFileSync(new URL('package.json', root), 'utf8'));
    validateReleaseNotes(catalog, version, values.version);
    console.log(
      `Release notes check passed${values.version ? `: ${values.version}` : ' (catalog)'}.`,
    );
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}
