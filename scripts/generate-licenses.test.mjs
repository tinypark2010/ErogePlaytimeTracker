import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, test } from 'vitest';
import {
  collectCargoDependencies,
  collectNpmDependencies,
  renderLicenseNotice,
  repositoryUrl,
} from './generate-licenses.mjs';

const temporaryDirectories = [];

function temporaryDirectory() {
  const directory = mkdtempSync(join(tmpdir(), 'license-generator-'));
  temporaryDirectories.push(directory);
  return directory;
}

function installPackage(baseDir, packagePath, manifest) {
  const directory = join(baseDir, packagePath);
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, 'package.json'), JSON.stringify(manifest));
  writeFileSync(join(directory, 'LICENSE'), `${manifest.name} license text`);
}

afterEach(() => {
  for (const directory of temporaryDirectories.splice(0)) {
    rmSync(directory, { recursive: true, force: true });
  }
});

describe('npm dependency inventory', () => {
  test('reads names and source repositories from installed package manifests', () => {
    const baseDir = temporaryDirectory();
    installPackage(baseDir, 'node_modules/example', {
      name: 'example',
      version: '1.2.3',
      license: 'MIT',
      repository: { type: 'git', url: 'git+https://github.com/example/example.git' },
    });
    const lock = {
      packages: {
        '': { dependencies: { example: '^1.2.0' } },
        'node_modules/example': { version: '1.2.3', license: 'MIT' },
      },
    };

    const dependencies = collectNpmDependencies(
      lock,
      { dependencies: { example: '^1.2.0' } },
      baseDir,
    );

    expect(dependencies).toEqual([
      expect.objectContaining({
        name: 'npm: example@1.2.3',
        sourceUrl: 'https://github.com/example/example',
      }),
    ]);
  });

  test('includes every standalone license and notice file from a package', () => {
    const baseDir = temporaryDirectory();
    installPackage(baseDir, 'node_modules/example', {
      name: 'example',
      version: '1.2.3',
      license: 'Apache-2.0',
    });
    writeFileSync(join(baseDir, 'node_modules/example/NOTICE'), 'required notice');

    const [dependency] = collectNpmDependencies(
      {
        packages: {
          '': { dependencies: { example: '1.2.3' } },
          'node_modules/example': { version: '1.2.3', license: 'Apache-2.0' },
        },
      },
      { dependencies: { example: '1.2.3' } },
      baseDir,
    );

    expect(dependency.text).toContain('----- LICENSE -----\n\nexample license text');
    expect(dependency.text).toContain('----- NOTICE -----\n\nrequired notice');
  });

  test('fails when npm declares production dependencies but inventories none', () => {
    expect(() =>
      collectNpmDependencies(
        { packages: { '': { dependencies: { missing: '1.0.0' } } } },
        { dependencies: { missing: '1.0.0' } },
        temporaryDirectory(),
      ),
    ).toThrow('declares 1 production dependencies, but none were inventoried');
  });

  test('fails when any root production dependency is absent from the inventory', () => {
    const baseDir = temporaryDirectory();
    installPackage(baseDir, 'node_modules/included', {
      name: 'included',
      version: '1.0.0',
      license: 'MIT',
    });

    expect(() =>
      collectNpmDependencies(
        {
          packages: {
            '': { dependencies: { included: '1.0.0', missing: '1.0.0' } },
            'node_modules/included': { version: '1.0.0', license: 'MIT' },
          },
        },
        { dependencies: { included: '1.0.0', missing: '1.0.0' } },
        baseDir,
      ),
    ).toThrow('npm root production dependencies missing from inventory: missing');
  });
});

describe('source URLs', () => {
  test('normalizes common npm repository formats', () => {
    expect(repositoryUrl('owner/project')).toBe('https://github.com/owner/project');
    expect(repositoryUrl('git@github.com:owner/project.git')).toBe(
      'https://github.com/owner/project',
    );
  });

  test('rejects an inventory entry without a source URL', () => {
    expect(() =>
      renderLicenseNotice([
        { name: 'npm: example@1.0.0', license: 'MIT', sourceUrl: null, text: 'license' },
      ]),
    ).toThrow('npm: example@1.0.0: source URL is missing');
  });

  test('uses a versioned crates.io URL and an explicit MPL source notice', () => {
    const packageDir = temporaryDirectory();
    writeFileSync(join(packageDir, 'LICENSE'), 'MPL license text');
    const dependencies = collectCargoDependencies({
      packages: [
        {
          name: 'mpl-example',
          version: '2.0.0',
          license: 'MPL-2.0',
          source: 'registry+https://github.com/rust-lang/crates.io-index',
          repository: 'https://github.com/example/mpl-example.git',
          manifest_path: join(packageDir, 'Cargo.toml'),
          license_file: null,
        },
      ],
    });

    const notice = renderLicenseNotice(dependencies).content;
    expect(notice).toContain(
      'Source code is available at: https://crates.io/crates/mpl-example/2.0.0',
    );
    expect(notice).toContain('Source repository: https://github.com/example/mpl-example');
  });
});
