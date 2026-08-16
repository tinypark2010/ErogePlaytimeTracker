import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

const allowed = new Set([
  '0BSD', 'Apache-2.0', 'BSD-2-Clause', 'BSD-3-Clause', 'BSL-1.0',
  'CC0-1.0', 'CDLA-Permissive-2.0', 'ISC', 'MIT', 'MIT-0', 'MPL-2.0',
  'OpenSSL', 'Unicode-3.0', 'Unlicense', 'Zlib', 'LLVM-exception',
]);

function checkExpression(expression, packageName) {
  if (!expression) throw new Error(`${packageName}: license metadata is missing`);
  const ids = expression.match(/[A-Za-z0-9][A-Za-z0-9.+-]*/g) ?? [];
  const rejected = ids.filter(id => !['AND', 'OR', 'WITH'].includes(id) && !allowed.has(id));
  if (rejected.length) throw new Error(`${packageName}: unapproved license ${rejected.join(', ')} (${expression})`);
}

function licenseText(packageDir) {
  if (!existsSync(packageDir)) return null;
  const file = readdirSync(packageDir).sort().find(name => /^(licen[cs]e|copying|notice)([-_.].*)?$/i.test(name));
  return file ? readFileSync(join(packageDir, file), 'utf8').replace(/\r\n/g, '\n').trim() : null;
}

const dependencies = [];
const npmLock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
for (const [path, pkg] of Object.entries(npmLock.packages)) {
  if (!path || pkg.dev || !pkg.name || !pkg.version) continue;
  checkExpression(pkg.license, `${pkg.name}@${pkg.version}`);
  const text = licenseText(path);
  dependencies.push({ name: `npm: ${pkg.name}@${pkg.version}`, license: pkg.license, text });
}

const cargo = JSON.parse(execFileSync('cargo', [
  'metadata', '--manifest-path', 'src-tauri/Cargo.toml', '--format-version', '1',
  '--filter-platform', 'x86_64-pc-windows-msvc', '--locked',
], { encoding: 'utf8', maxBuffer: 50 * 1024 * 1024 }));
for (const pkg of cargo.packages) {
  if (!pkg.source) continue;
  checkExpression(pkg.license, `${pkg.name}@${pkg.version}`);
  const packageDir = dirname(pkg.manifest_path);
  const text = pkg.license_file
    ? readFileSync(join(packageDir, pkg.license_file), 'utf8').trim()
    : licenseText(packageDir);
  dependencies.push({ name: `Rust crate: ${pkg.name}@${pkg.version}`, license: pkg.license, text });
}

dependencies.sort((a, b) => a.name.localeCompare(b.name));
const groups = new Map();
for (const dependency of dependencies) {
  const key = dependency.text ?? `MISSING:${dependency.license}`;
  const group = groups.get(key) ?? { text: dependency.text, usedBy: [] };
  group.usedBy.push(`${dependency.name} — ${dependency.license}`);
  groups.set(key, group);
}

const inventory = dependencies.map(item => `- ${item.name} — ${item.license}`).join('\n');
const licenseSections = [...groups.values()].map((group, index) => {
  const usedBy = group.usedBy.map(item => `- ${item}`).join('\n');
  const text = group.text ?? 'The package did not include a standalone license file. Its declared SPDX license expression is listed above; consult the package repository for the complete terms.';
  return `License text ${index + 1}\n\nUsed by:\n${usedBy}\n\n${text}`;
});
const divider = '\n\n' + '='.repeat(78) + '\n\n';
const header = `Eroge Playtime Tracker - Third-Party Licenses\n\nGenerated from package-lock.json and Cargo.lock for the Windows x64 distribution.\nThe project itself is licensed separately under the MIT License in LICENSE.\nIdentical license texts are included once and shared by every package listed under "Used by".\n\nDEPENDENCY INVENTORY\n\n`;
writeFileSync('THIRD_PARTY_LICENSES.txt', header + inventory + divider + 'LICENSE TEXTS' + divider + licenseSections.join(divider) + '\n');
console.log(`Generated THIRD_PARTY_LICENSES.txt with ${dependencies.length} dependencies and ${groups.size} unique license texts.`);
