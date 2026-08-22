import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const primaryPrefixes = [
  'update',
  'fix',
  'refactor',
  'docs',
  'build',
  'ci',
  'format',
  'release',
  'chore',
  'test',
];
const testablePrefixes = ['update', 'fix', 'refactor', 'build', 'ci'];
const primaryPattern = new RegExp(`^\\[(${primaryPrefixes.join('|')})\\] (.+)$`);
const testedPattern = new RegExp(`^\\[(${testablePrefixes.join('|')}), test\\] (.+)$`);

export function validateCommitMessage(message) {
  const normalized = message.replace(/\r\n/g, '\n').trimEnd();
  const lines = normalized.split('\n');
  const subject = lines[0] ?? '';
  const errors = [];
  const match = subject.match(primaryPattern) ?? subject.match(testedPattern);

  if (!match) {
    errors.push(
      'subject must start with one allowed primary prefix and may use only test as an allowed secondary prefix',
    );
  } else if (!match[2].trim()) {
    errors.push('subject summary must not be empty');
  }
  if (subject.length > 72) errors.push(`subject is ${subject.length} characters; maximum is 72`);
  if (subject.endsWith('.')) errors.push('subject must not end with a period');

  if (lines.length > 1) {
    if (lines[1] !== '') {
      errors.push('subject and body must be separated by one blank line');
    } else {
      const body = lines.slice(2);
      if (body.some((line) => line === '')) errors.push('body must not contain blank lines');
      if (body.length > 3) errors.push(`body has ${body.length} lines; maximum is 3`);
      for (const line of body) {
        if (!/^- \S/.test(line))
          errors.push(`body line must be a bullet beginning with "- ": ${line}`);
      }
    }
  }

  return errors;
}

export function isProductionPath(file) {
  const path = file.replaceAll('\\', '/');
  if (/^src\/.*\.(test|spec)\.[^.]+$/.test(path)) return false;
  return (
    path.startsWith('src/') || path.startsWith('src-tauri/src/') || path === 'src-tauri/build.rs'
  );
}

export function parseNumstat(output) {
  return output
    .replace(/\r\n/g, '\n')
    .split('\n')
    .filter(Boolean)
    .map((line) => {
      const [added, deleted, ...pathParts] = line.split('\t');
      return {
        added: added === '-' ? 0 : Number.parseInt(added, 10),
        deleted: deleted === '-' ? 0 : Number.parseInt(deleted, 10),
        path: pathParts.join('\t'),
      };
    });
}

function git(args) {
  return execFileSync('git', args, { encoding: 'utf8', maxBuffer: 10 * 1024 * 1024 });
}

function parseArgs(argv) {
  const options = { base: 'origin/main', head: 'HEAD', messageFile: null };
  for (let index = 0; index < argv.length; index++) {
    const value = argv[index];
    if (value === '--base') options.base = argv[++index];
    else if (value === '--head') options.head = argv[++index];
    else if (value === '--message-file') options.messageFile = argv[++index];
    else throw new Error(`unknown argument: ${value}`);
  }
  if (!options.base || !options.head) throw new Error('--base and --head require values');
  return options;
}

function readCommits(base, head) {
  const output = git(['log', '--format=%H%x1f%B%x1e', `${base}..${head}`]);
  return output
    .split('\x1e')
    .map((entry) => entry.replace(/^\r?\n/, ''))
    .filter((entry) => entry.trim())
    .map((entry) => {
      const separator = entry.indexOf('\x1f');
      return {
        hash: entry.slice(0, separator),
        message: entry.slice(separator + 1).replace(/\r?\n$/, ''),
      };
    });
}

function productionAdditions(args) {
  return parseNumstat(git(args))
    .filter((entry) => isProductionPath(entry.path))
    .reduce((total, entry) => total + entry.added, 0);
}

function checkMessageFile(path) {
  const errors = validateCommitMessage(readFileSync(path, 'utf8'));
  if (errors.length) {
    for (const error of errors) console.error(`ERROR: ${error}`);
    process.exitCode = 1;
  } else {
    console.log('Commit message follows repository policy.');
  }
}

function checkRange(base, head) {
  const commits = readCommits(base, head);
  if (!commits.length) throw new Error(`no commits found in ${base}..${head}`);

  let failed = false;
  for (const commit of commits) {
    const subject = commit.message.split(/\r?\n/, 1)[0];
    const errors = validateCommitMessage(commit.message);
    for (const error of errors) {
      console.error(`ERROR ${commit.hash.slice(0, 12)} ${subject}: ${error}`);
      failed = true;
    }
    const additions = productionAdditions(['show', '--numstat', '--format=', commit.hash]);
    console.log(`${commit.hash.slice(0, 12)} production additions: ${additions}`);
    if (additions > 200) {
      console.warn(`WARNING ${commit.hash.slice(0, 12)} exceeds the 200-line commit guideline`);
    }
  }

  const additions = productionAdditions(['diff', '--numstat', `${base}..${head}`]);
  console.log(`Validated commits: ${commits.length}`);
  console.log(`Pull request production additions: ${additions}`);
  if (commits.length > 5) console.warn('WARNING pull request exceeds the 5-commit guideline');
  if (additions > 500) console.warn('WARNING pull request exceeds the 500-line guideline');
  if (failed) process.exitCode = 1;
}

function main() {
  try {
    const options = parseArgs(process.argv.slice(2));
    if (options.messageFile) checkMessageFile(options.messageFile);
    else checkRange(options.base, options.head);
  } catch (error) {
    console.error(`ERROR: ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) main();
