import assert from 'node:assert/strict';
import { test } from 'vitest';
import { isProductionPath, parseNumstat, validateCommitMessage } from './validate-commits.mjs';

test('accepts one primary prefix and approved test combinations', () => {
  for (const message of [
    '[docs] Define pull request rules',
    '[test] Cover commit message validation',
    '[update, test] Add library filtering',
    '[fix, test] Reject invalid session ranges',
    '[refactor, test] Isolate tracking transitions',
    '[build, test] Validate generated notices',
    '[ci, test] Check pull request commits',
  ]) {
    assert.deepEqual(validateCommitMessage(message), [], message);
  }
});

test('rejects multiple primary prefixes and unapproved combinations', () => {
  for (const message of [
    '[update, fix] Mix a feature and a fix',
    '[update, docs] Mix implementation and documentation tags',
    '[docs, test] Test documentation',
    '[update, test, docs] Use three prefixes',
    'Add a change without a prefix',
  ]) {
    assert.ok(validateCommitMessage(message).length > 0, message);
  }
});

test('accepts an optional body containing at most three bullets', () => {
  assert.deepEqual(
    validateCommitMessage(
      '[fix, test] Keep updates behind the tracking guard\n\n- Check both entry points\n- Preserve the active game state',
    ),
    [],
  );
  assert.ok(validateCommitMessage('[docs] Explain workflow\nMissing blank line').length > 0);
  assert.ok(validateCommitMessage('[docs] Explain workflow\n\nNot a bullet').length > 0);
  assert.ok(
    validateCommitMessage('[docs] Explain workflow\n\n- One\n- Two\n- Three\n- Four').length > 0,
  );
});

test('enforces the subject length and period rules', () => {
  assert.ok(validateCommitMessage('[docs] End with a period.').length > 0);
  assert.ok(validateCommitMessage(`[docs] ${'x'.repeat(70)}`).length > 0);
});

test('counts only application paths as production code', () => {
  assert.equal(isProductionPath('src/App.svelte'), true);
  assert.equal(isProductionPath('src/lib/time.test.ts'), false);
  assert.equal(isProductionPath('src-tauri/src/database/mod.rs'), true);
  assert.equal(isProductionPath('docs/development-workflow.md'), false);
  assert.equal(isProductionPath('package-lock.json'), false);
});

test('parses text and binary numstat entries', () => {
  assert.deepEqual(parseNumstat('12\t3\tsrc/App.svelte\n-\t-\tsrc-tauri/icons/icon.ico\n'), [
    { added: 12, deleted: 3, path: 'src/App.svelte' },
    { added: 0, deleted: 0, path: 'src-tauri/icons/icon.ico' },
  ]);
});
