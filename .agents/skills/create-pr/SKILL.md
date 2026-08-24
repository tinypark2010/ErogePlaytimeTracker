---
name: create-pr
description: Prepare and open GitHub pull requests for this repository when the user asks to create, open, or publish a PR. Organize current changes into policy-compliant branches and commits, validate and push topic branches, and create the PR with GitHub CLI. Do not use for ordinary implementation, commit-only requests, status reports, or PR review.
---

# Create pull request

Create focused, policy-compliant pull requests without modifying or merging `main` directly.

## Required context

Read the root `AGENTS.md` and `docs/development-workflow.md` completely before changing Git state. The detailed workflow document is the source of truth for branch names, commit prefixes, size guidelines, PR bodies, stacked PRs, and merge policy.

Check `gh --version` and `gh auth status` before creating commits or pushing. Run the authentication check with the same network access required for publishing: a restricted sandbox can report a valid credential as invalid when GitHub is unreachable. If a sandboxed check reports an invalid token or a connection failure, rerun `gh auth status` with the required network or escalated permission before diagnosing an authentication problem. Do not ask the user to replace credentials or reauthenticate unless that unrestricted check also fails. Never print the token value. If GitHub CLI is unavailable or the unrestricted check confirms it is unauthenticated, report the prerequisite and stop before partially publishing the change.

## Outcome partition gate

Run this gate before changing Git state and repeat it before pushing:

1. Enumerate every requested behavior and every outcome already present in the base diff. Give each outcome one independently verifiable acceptance criterion.
2. Treat two outcomes as independent when either can be accepted, reviewed, released, or reverted without the other and neither technically depends on the other's schema, API, code, migration, or other prerequisite.
3. Do not use a shared request, screen, component, file, theme, prefix, branch, timing, or likely merge conflict as evidence that outcomes belong in one PR. Do not hide separate outcomes under an umbrella title.
4. Map every commit to one outcome. Multiple commits may share a PR only when every commit is necessary for the same acceptance criterion, such as a required migration, preparatory refactor, implementation, test, or documentation.
5. Determine the number of PRs from this outcome map. A user's singular phrase such as "create a PR" authorizes publication but does not require one PR. If the user explicitly requires one PR and that conflicts with repository policy, report the conflict and proposed split before publishing.

Fail closed: if independent outcomes are present or commit cohesion cannot be explained by every commit's necessity for the same acceptance criterion, do not push or create the PR. Split each independent outcome into a branch from `main`; stack only for a genuine technical prerequisite.

Examples:

- Separate PRs: rearrange controls on a detail screen and add a new editing capability on that screen. Each has its own acceptance criterion and can ship or revert independently.
- One PR: add a schema migration and the feature that requires the migrated schema. Neither part safely delivers the acceptance criterion alone.

## Prepare the change

1. Inspect the current branch, status, base commits, complete diff, and existing unpushed commits. Preserve unrelated and user-authored changes.
2. Apply the outcome partition gate and create an outcome map covering the planned PR(s), acceptance criteria, commits, and technical dependencies.
3. If currently on `main`, create a topic branch without discarding the working tree, subject to the base and dirty-tree checks in `docs/development-workflow.md`. If currently on another topic branch, confirm the change is a continuation or an intentional stack; do not use that branch as the base for independent work merely because it is checked out. Never commit on or push to `main`.
4. Before mutating Git state, send a concise commentary update listing one acceptance outcome per planned PR, the base/head branches, commits, and any dependency. Continue without a blocking question when the partition is unambiguous.
5. Stage explicit paths or hunks; do not use `git add .`. Keep implementation and its corresponding tests in the same commit. Do not rewrite user-authored or already-pushed commits without explicit authorization.
6. Write messages exactly as required by `docs/development-workflow.md`. A primary change and its tests use the approved `[primary, test]` form; never combine two primary prefixes.

When independent work must become separate PRs, base each branch on `main` and publish them sequentially where practical. Stack only when a later change genuinely depends on an unmerged earlier change. Preserve all local changes without destructive reset or an implicit stash. Record the parent PR, base branch, and merge order in every dependent PR.

## Validate and publish

1. Run focused tests while organizing commits, then run the full handoff checks from `AGENTS.md`.
2. Run `npm run commit-policy:check -- --base <base> --head HEAD`. Message errors must be fixed. Size and commit-count warnings require a split or an explanation in the PR body.
3. Re-read `git status`, the complete base diff, and the commit list, then repeat the outcome partition gate. Confirm there are no generated outputs, unrelated changes, secrets, independent outcomes, or commits that cannot be mapped to the single PR outcome.
4. Push only the topic branch with an upstream. Do not force-push.
5. Fill `.github/pull_request_template.md` with one standalone acceptance outcome, concrete changes, exact verification results, commit count, production additions, related PRs, and commit cohesion. `Why this cannot be split` must not be `None` for a multi-commit PR; explain why every commit is necessary for the same acceptance criterion or split the PR.
6. Create the PR non-interactively with explicit `--base`, `--head`, `--title`, and `--body-file`. The title uses the primary prefix only and is at most 72 characters.
7. Verify the created PR using `gh pr view --json url,title,baseRefName,headRefName,commits,additions,deletions,files`.

Do not merge the PR unless the user separately and explicitly requests it. After verifying the PR, remain on its head branch for CI or review follow-up; do not switch to `main` or delete branches unless separately requested. Finish by reporting the PR URL, base/head, commits, checks, current branch, and any policy exception.
