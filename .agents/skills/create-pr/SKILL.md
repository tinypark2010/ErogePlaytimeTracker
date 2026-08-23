---
name: create-pr
description: Prepare and open GitHub pull requests for this repository when the user asks to create, open, or publish a PR. Organize current changes into policy-compliant branches and commits, validate and push topic branches, and create the PR with GitHub CLI. Do not use for ordinary implementation, commit-only requests, status reports, or PR review.
---

# Create pull request

Create focused, policy-compliant pull requests without modifying or merging `main` directly.

## Required context

Read the root `AGENTS.md` and `docs/development-workflow.md` completely before changing Git state. The detailed workflow document is the source of truth for branch names, commit prefixes, size guidelines, PR bodies, stacked PRs, and merge policy.

Check `gh --version` and `gh auth status` before creating commits or pushing. Run the authentication check with the same network access required for publishing: a restricted sandbox can report a valid credential as invalid when GitHub is unreachable. If a sandboxed check reports an invalid token or a connection failure, rerun `gh auth status` with the required network or escalated permission before diagnosing an authentication problem. Do not ask the user to replace credentials or reauthenticate unless that unrestricted check also fails. Never print the token value. If GitHub CLI is unavailable or the unrestricted check confirms it is unauthenticated, report the prerequisite and stop before partially publishing the change.

## Prepare the change

1. Inspect the current branch, status, base commits, complete diff, and existing unpushed commits. Preserve unrelated and user-authored changes.
2. Partition the work by outcome. One PR may contain multiple commits only when they form one deliverable. Separate independent outcomes even when they share a prefix.
3. If currently on `main`, create a topic branch without discarding the working tree, subject to the base and dirty-tree checks in `docs/development-workflow.md`. If currently on another topic branch, confirm the change is a continuation or an intentional stack; do not use that branch as the base for independent work merely because it is checked out. Never commit on or push to `main`.
4. Before mutating Git state, send a concise commentary update listing the planned PR(s), base/head branches, and commits. Continue without a blocking question when the partition is unambiguous.
5. Stage explicit paths or hunks; do not use `git add .`. Keep implementation and its corresponding tests in the same commit. Do not rewrite user-authored or already-pushed commits without explicit authorization.
6. Write messages exactly as required by `docs/development-workflow.md`. A primary change and its tests use the approved `[primary, test]` form; never combine two primary prefixes.

When independent work must become separate PRs, base each branch on `main` and publish them sequentially where practical. Stack only when a later change genuinely depends on an unmerged earlier change. Preserve all local changes without destructive reset or an implicit stash. Record the parent PR, base branch, and merge order in every dependent PR.

## Validate and publish

1. Run focused tests while organizing commits, then run the full handoff checks from `AGENTS.md`.
2. Run `npm run commit-policy:check -- --base <base> --head HEAD`. Message errors must be fixed. Size and commit-count warnings require a split or an explanation in the PR body.
3. Re-read `git status`, the base diff, and the commit list. Confirm there are no generated outputs, unrelated changes, secrets, or multiple outcomes.
4. Push only the topic branch with an upstream. Do not force-push.
5. Fill `.github/pull_request_template.md` with concrete purpose, changes, exact verification results, commit count, production additions, related PRs, and any split exception.
6. Create the PR non-interactively with explicit `--base`, `--head`, `--title`, and `--body-file`. The title uses the primary prefix only and is at most 72 characters.
7. Verify the created PR using `gh pr view --json url,title,baseRefName,headRefName,commits,additions,deletions,files`.

Do not merge the PR unless the user separately and explicitly requests it. After verifying the PR, remain on its head branch for CI or review follow-up; do not switch to `main` or delete branches unless separately requested. Finish by reporting the PR URL, base/head, commits, checks, current branch, and any policy exception.
