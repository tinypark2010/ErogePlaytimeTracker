import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const allowed = new Set([
  '0BSD',
  'Apache-2.0',
  'BSD-2-Clause',
  'BSD-3-Clause',
  'BSL-1.0',
  'CC0-1.0',
  'CDLA-Permissive-2.0',
  'ISC',
  'MIT',
  'MIT-0',
  'MPL-2.0',
  'OpenSSL',
  'Unicode-3.0',
  'Unlicense',
  'Zlib',
  'LLVM-exception',
]);

function checkExpression(expression, packageName) {
  if (!expression) throw new Error(`${packageName}: license metadata is missing`);
  const ids = expression.match(/[A-Za-z0-9][A-Za-z0-9.+-]*/g) ?? [];
  const rejected = ids.filter((id) => !['AND', 'OR', 'WITH'].includes(id) && !allowed.has(id));
  if (rejected.length)
    throw new Error(`${packageName}: unapproved license ${rejected.join(', ')} (${expression})`);
}

function legalTexts(packageDir) {
  if (!existsSync(packageDir)) return null;
  const files = readdirSync(packageDir)
    .sort()
    .filter((name) => /^(licen[cs]e|copying|notice)([-_.].*)?$/i.test(name));
  if (files.length === 0) return null;

  const texts = files.map((file) =>
    readFileSync(join(packageDir, file), 'utf8').replace(/\r\n/g, '\n').trim(),
  );
  if (files.length === 1) return texts[0];
  return files.map((file, index) => `----- ${file} -----\n\n${texts[index]}`).join('\n\n');
}

export function repositoryUrl(repository) {
  let url = typeof repository === 'string' ? repository : repository?.url;
  if (!url) return null;

  if (/^[\w.-]+\/[\w.-]+$/.test(url)) url = `https://github.com/${url}`;
  url = url
    .replace(/^git\+/, '')
    .replace(/^git:\/\//, 'https://')
    .replace(/^ssh:\/\/git@/, 'https://')
    .replace(/^git@([^:]+):/, 'https://$1/')
    .replace(/\.git$/, '');

  return /^https?:\/\//.test(url) ? url : null;
}

function npmSourceUrl(pkg) {
  return (
    repositoryUrl(pkg.repository) ?? `https://www.npmjs.com/package/${pkg.name}/v/${pkg.version}`
  );
}

function rustSourceUrl(pkg) {
  const repository = repositoryUrl(pkg.repository);
  if (repository) return repository;
  if (pkg.source?.startsWith('registry+')) {
    return `https://crates.io/crates/${pkg.name}/${pkg.version}`;
  }

  const source = pkg.source?.startsWith('git+') ? repositoryUrl(pkg.source) : null;
  if (source) return source;
  throw new Error(`${pkg.name}@${pkg.version}: source URL metadata is missing`);
}

export function collectNpmDependencies(npmLock, rootManifest, baseDir = '.') {
  const productionEntries = Object.entries(npmLock.packages ?? {}).filter(
    ([packagePath, pkg]) => packagePath && !pkg.dev,
  );
  const dependencies = new Map();
  const installedPaths = new Set();

  for (const [packagePath, lockPackage] of productionEntries) {
    const packageDir = join(baseDir, packagePath);
    const manifestPath = join(packageDir, 'package.json');
    if (!existsSync(manifestPath)) {
      throw new Error(
        `npm production dependency ${packagePath}: installed package metadata is missing; run npm ci`,
      );
    }

    const pkg = JSON.parse(readFileSync(manifestPath, 'utf8'));
    if (!pkg.name || !pkg.version) {
      throw new Error(`${packagePath}: installed package name or version is missing`);
    }
    if (lockPackage.version && lockPackage.version !== pkg.version) {
      throw new Error(
        `${pkg.name}: installed version ${pkg.version} does not match package-lock.json ${lockPackage.version}`,
      );
    }

    const packageName = `${pkg.name}@${pkg.version}`;
    const license = pkg.license ?? lockPackage.license;
    checkExpression(license, packageName);
    installedPaths.add(packagePath.replaceAll('\\', '/'));
    dependencies.set(packageName, {
      name: `npm: ${packageName}`,
      license,
      sourceUrl: npmSourceUrl(pkg),
      text: legalTexts(packageDir),
    });
  }

  const rootDependencyNames = Object.keys(rootManifest.dependencies ?? {});
  if (rootDependencyNames.length > 0 && dependencies.size === 0) {
    throw new Error(
      `npm declares ${rootDependencyNames.length} production dependencies, but none were inventoried`,
    );
  }

  const missingRootDependencies = rootDependencyNames.filter(
    (name) => !installedPaths.has(`node_modules/${name}`),
  );
  if (missingRootDependencies.length) {
    throw new Error(
      `npm root production dependencies missing from inventory: ${missingRootDependencies.join(', ')}`,
    );
  }

  return [...dependencies.values()];
}

export function collectCargoDependencies(cargo) {
  const dependencies = [];
  for (const pkg of cargo.packages) {
    if (!pkg.source) continue;
    const packageName = `${pkg.name}@${pkg.version}`;
    checkExpression(pkg.license, packageName);
    const packageDir = dirname(pkg.manifest_path);
    const licenseFile = pkg.license_file
      ? isAbsolute(pkg.license_file)
        ? pkg.license_file
        : join(packageDir, pkg.license_file)
      : null;
    const text = licenseFile ? readFileSync(licenseFile, 'utf8').trim() : legalTexts(packageDir);
    dependencies.push({
      name: `Rust crate: ${packageName}`,
      license: pkg.license,
      sourceUrl: rustSourceUrl(pkg),
      versionedSourceUrl: pkg.source.startsWith('registry+')
        ? `https://crates.io/crates/${pkg.name}/${pkg.version}`
        : null,
      text,
    });
  }
  return dependencies;
}

function repositoryFile(baseDir, file, owner) {
  if (typeof file !== 'string' || !file) throw new Error(`${owner}: file path is missing`);
  const root = resolve(baseDir);
  const path = resolve(root, file);
  const relativePath = relative(root, path);
  if (relativePath.startsWith('..') || isAbsolute(relativePath)) {
    throw new Error(`${owner}: file must be inside the repository (${file})`);
  }
  if (!existsSync(path)) throw new Error(`${owner}: file is missing (${file})`);
  return path;
}

export function collectAssetDependencies(assetManifest, baseDir = '.') {
  if (!Array.isArray(assetManifest)) throw new Error('third-party asset manifest must be an array');

  return assetManifest.map((asset) => {
    if (!asset.name) throw new Error('third-party asset name is missing');
    checkExpression(asset.license, asset.name);
    if (!repositoryUrl(asset.sourceUrl)) throw new Error(`${asset.name}: source URL is invalid`);
    if (!Array.isArray(asset.legalFiles) || asset.legalFiles.length === 0) {
      throw new Error(`${asset.name}: at least one legal file is required`);
    }

    const legalTexts = asset.legalFiles.map((file) => ({
      file,
      text: readFileSync(repositoryFile(baseDir, file, asset.name), 'utf8')
        .replace(/\r\n/g, '\n')
        .trim(),
    }));
    for (const file of asset.files ?? []) {
      if (!/^[a-f0-9]{64}$/i.test(file.sha256 ?? '')) {
        throw new Error(`${asset.name}: SHA-256 is invalid for ${file.path ?? 'unknown file'}`);
      }
      const contents = readFileSync(repositoryFile(baseDir, file.path, asset.name));
      const actual = createHash('sha256').update(contents).digest('hex');
      if (actual !== file.sha256.toLowerCase()) {
        throw new Error(`${asset.name}: SHA-256 mismatch for ${file.path}`);
      }
    }

    return {
      name: asset.name,
      license: asset.license,
      sourceUrl: asset.sourceUrl,
      text:
        legalTexts.length === 1
          ? legalTexts[0].text
          : legalTexts.map(({ file, text }) => `----- ${file} -----\n\n${text}`).join('\n\n'),
    };
  });
}

export function renderLicenseNotice(inputDependencies) {
  const dependencies = [...inputDependencies].sort((a, b) => a.name.localeCompare(b.name));
  const groups = new Map();
  for (const dependency of dependencies) {
    if (!dependency.sourceUrl) throw new Error(`${dependency.name}: source URL is missing`);
    const key = dependency.text ?? `MISSING:${dependency.license}`;
    const group = groups.get(key) ?? { text: dependency.text, usedBy: [] };
    group.usedBy.push(`${dependency.name} — ${dependency.license}`);
    groups.set(key, group);
  }

  const inventory = dependencies
    .map((item) => {
      if (!item.license.includes('MPL-2.0')) {
        return `- ${item.name} — ${item.license}\n  Source: ${item.sourceUrl}`;
      }

      const versionedSourceUrl = item.versionedSourceUrl ?? item.sourceUrl;
      const repository =
        versionedSourceUrl === item.sourceUrl ? '' : `\n  Source repository: ${item.sourceUrl}`;
      return `- ${item.name} — ${item.license}\n  Source code is available at: ${versionedSourceUrl}${repository}`;
    })
    .join('\n');
  const licenseSections = [...groups.values()].map((group, index) => {
    const usedBy = group.usedBy.map((item) => `- ${item}`).join('\n');
    const text =
      group.text ??
      'The package did not include a standalone license file. Its declared SPDX license expression is listed above; consult the source URL in the dependency inventory for the complete terms.';
    return `License text ${index + 1}\n\nUsed by:\n${usedBy}\n\n${text}`;
  });
  const divider = '\n\n' + '='.repeat(78) + '\n\n';
  const header = `Eroge Playtime Tracker - Third-Party Licenses\n\nGenerated from package-lock.json, Cargo.lock, and third-party/assets.json for the Windows x64 distribution.\nThe project itself is licensed separately under the MIT License in LICENSE.\nEvery dependency and redistributed asset includes a source location in the inventory below.\nIdentical license texts are included once and shared by every package listed under "Used by".\n\nDEPENDENCY INVENTORY\n\n`;
  return {
    content:
      header +
      inventory +
      divider +
      'LICENSE TEXTS' +
      divider +
      licenseSections.join(divider) +
      '\n',
    groupCount: groups.size,
  };
}

export function generateLicenseFile(baseDir = '.') {
  const npmLock = JSON.parse(readFileSync(join(baseDir, 'package-lock.json'), 'utf8'));
  const rootManifest = JSON.parse(readFileSync(join(baseDir, 'package.json'), 'utf8'));
  const npmDependencies = collectNpmDependencies(npmLock, rootManifest, baseDir);
  const cargo = JSON.parse(
    execFileSync(
      'cargo',
      [
        'metadata',
        '--manifest-path',
        'src-tauri/Cargo.toml',
        '--format-version',
        '1',
        '--filter-platform',
        'x86_64-pc-windows-msvc',
        '--locked',
      ],
      { cwd: baseDir, encoding: 'utf8', maxBuffer: 50 * 1024 * 1024 },
    ),
  );
  const rustDependencies = collectCargoDependencies(cargo);
  const assetDependencies = collectAssetDependencies(
    JSON.parse(readFileSync(join(baseDir, 'third-party/assets.json'), 'utf8')),
    baseDir,
  );
  const dependencies = [...npmDependencies, ...rustDependencies, ...assetDependencies];
  const notice = renderLicenseNotice(dependencies);
  writeFileSync(join(baseDir, 'THIRD_PARTY_LICENSES.txt'), notice.content);
  return {
    dependencyCount: dependencies.length,
    npmDependencyCount: npmDependencies.length,
    rustDependencyCount: rustDependencies.length,
    assetDependencyCount: assetDependencies.length,
    groupCount: notice.groupCount,
  };
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : null;
if (invokedPath === import.meta.url) {
  const result = generateLicenseFile();
  console.log(
    `Generated THIRD_PARTY_LICENSES.txt with ${result.npmDependencyCount} npm dependencies, ${result.rustDependencyCount} Rust crates, ${result.assetDependencyCount} redistributed assets, and ${result.groupCount} unique license texts.`,
  );
}
