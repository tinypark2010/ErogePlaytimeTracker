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
  const file = readdirSync(packageDir).find(name => /^(licen[cs]e|copying|notice)([-_.].*)?$/i.test(name));
  return file ? readFileSync(join(packageDir, file), 'utf8').trim() : null;
}

const sections = [];
const npmLock = JSON.parse(readFileSync('package-lock.json', 'utf8'));
for (const [path, pkg] of Object.entries(npmLock.packages)) {
  if (!path || pkg.dev || !pkg.name || !pkg.version) continue;
  checkExpression(pkg.license, `${pkg.name}@${pkg.version}`);
  const text = licenseText(path);
  sections.push(`npm: ${pkg.name}@${pkg.version}\nLicense: ${pkg.license}\n${text ?? 'See the package repository for the complete license text.'}`);
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
  sections.push(`Rust crate: ${pkg.name}@${pkg.version}\nLicense: ${pkg.license}\n${text ?? 'See the crate repository for the complete license text.'}`);
}

sections.sort((a, b) => a.localeCompare(b));
const header = `Eroge Playtime Tracker - Third-Party Licenses\n\nGenerated from package-lock.json and Cargo.lock for the Windows x64 distribution.\nThe project itself is licensed separately under the MIT License in LICENSE.\n\n`;
writeFileSync('THIRD_PARTY_LICENSES.txt', header + sections.join('\n\n' + '='.repeat(78) + '\n\n') + '\n');
console.log(`Generated THIRD_PARTY_LICENSES.txt with ${sections.length} dependency entries.`);
