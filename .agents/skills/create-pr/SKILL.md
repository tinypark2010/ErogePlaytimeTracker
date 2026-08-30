---
name: create-pr
description: Prepare and open GitHub pull requests for this repository when the user asks to create, open, or publish a PR. Organize current changes into policy-compliant branches and commits, always publish multi-PR splits as one linear stack, validate and push topic branches, and create the PRs with GitHub CLI. Do not use for ordinary implementation, commit-only requests, status reports, or PR review.
---

# Create pull request

Create focused, policy-compliant pull requests without modifying or merging `main` directly.

## Required context

Read the root `AGENTS.md` and `docs/development-workflow.md` completely before changing Git state. The detailed workflow document is the source of truth for branch names, commit prefixes, size guidelines, PR bodies, and merge policy. For a request that produces multiple PRs, the mandatory stack topology in this skill overrides any instruction to base logically independent PRs separately on the integration branch; continue following the workflow document for every other rule.

Check `gh --version` and `gh auth status` before creating commits or pushing. Run the authentication check with the same network access required for publishing: a restricted sandbox can report a valid credential as invalid when GitHub is unreachable. If a sandboxed check reports an invalid token or a connection failure, rerun `gh auth status` with the required network or escalated permission before diagnosing an authentication problem. Do not ask the user to replace credentials or reauthenticate unless that unrestricted check also fails. Never print the token value. If GitHub CLI is unavailable or the unrestricted check confirms it is unauthenticated, report the prerequisite and stop before partially publishing the change.

## Outcome partition gate

Run this gate before changing Git state and repeat it before pushing:

1. Enumerate every requested behavior and every outcome already present in the base diff. Give each outcome one independently verifiable acceptance criterion.
2. Treat two outcomes as independent when either can be accepted, reviewed, released, or reverted without the other and neither technically depends on the other's schema, API, code, migration, or other prerequisite.
3. Do not use a shared request, screen, component, file, theme, prefix, branch, timing, or likely merge conflict as evidence that outcomes belong in one PR. Do not hide separate outcomes under an umbrella title.
4. Map every commit to one outcome. Multiple commits may share a PR only when every commit is necessary for the same acceptance criterion, such as a required migration, preparatory refactor, implementation, test, or documentation.
5. Determine the number of PRs from this outcome map. A user's singular phrase such as "create a PR" authorizes publication but does not require one PR. If the user explicitly requires one PR and that conflicts with repository policy, report the conflict and proposed split before publishing.

Fail closed: if independent outcomes are present or commit cohesion cannot be explained by every commit's necessity for the same acceptance criterion, do not combine them in one PR. Split each independent outcome into its own PR, then arrange every PR produced by the current request in the mandatory linear stack below. Logical independence determines PR boundaries, not branch topology.

Examples:

- Separate PRs: rearrange controls on a detail screen and add a new editing capability on that screen. Each has its own acceptance criterion and can ship or revert independently.
- One PR: add a schema migration and the feature that requires the migrated schema. Neither part safely delivers the acceptance criterion alone.

## Mandatory linear stack for multiple PRs

When the current request produces two or more PRs, publish them as one linear stack without exception:

```text
integration branch <- PR A branch <- PR B branch <- PR C branch
```

The following topology is forbidden because more than one PR uses the same base:

```text
integration branch <- PR A branch
integration branch <- PR B branch
```

- The first PR uses the integration branch as its base. Every later PR uses the immediately preceding PR branch as its base.
- Apply this rule regardless of logical independence, technical dependency, changed files, predicted merge conflicts, or confidence in conflict analysis. Conflict prediction must never be used to permit sibling PRs.
- No two PRs created from the same outcome-partition workflow may have the same `baseRefName`.
- Choose and announce the full parent-to-child order before changing Git state. Create each child branch from its declared parent branch after the parent outcome is committed.
- Push and create PRs in parent-to-child order. In each child PR, record the parent PR, current base branch, and complete merge order.
- Before every push, compare the planned base/head pairs and fail closed if they do not form one chain. After PR creation, fetch each PR's `baseRefName` and `headRefName` with `gh pr view`; do not finish while any sibling-base topology remains.
- After a parent PR is merged, retarget its immediate child to the integration branch, verify that the merged parent diff is no longer present, and repeat this process down the chain. Never merge a child before its parent.

## Prepare the change

1. Inspect the current branch, status, base commits, complete diff, and existing unpushed commits. Preserve unrelated and user-authored changes.
2. Apply the outcome partition gate and create an outcome map covering the planned PR(s), acceptance criteria, commits, technical dependencies, and the mandatory parent-to-child branch order.
3. For one PR, create its topic branch from the integration branch subject to the base and dirty-tree checks in `docs/development-workflow.md`. For multiple PRs, create the first topic branch from the integration branch and each later topic branch from the immediately preceding topic branch. Do not use a checked-out branch as a parent unless it is the declared previous branch in that chain. Never commit on or push to `main`.
4. Before mutating Git state, send a concise commentary update listing one acceptance outcome per planned PR, every base/head pair, commits, the complete merge order, and any technical dependency. Continue without a blocking question when the partition and order are unambiguous.
5. Stage explicit paths or hunks; do not use `git add .`. Keep implementation and its corresponding tests in the same commit. Do not rewrite user-authored or already-pushed commits without explicit authorization.
6. Write messages exactly as required by `docs/development-workflow.md`. A primary change and its tests use the approved `[primary, test]` form; never combine two primary prefixes.

When work becomes multiple PRs, always use the mandatory linear stack even when every outcome is technically independent. Never create sibling branches from `main` or from any other shared base. Preserve all local changes without destructive reset or an implicit stash.

## Validate and publish

1. Run focused tests while organizing commits, then run the full handoff checks from `AGENTS.md`.
2. For each PR branch, run `npm run commit-policy:check -- --base <its-declared-base> --head <its-head>`. Message errors must be fixed. Size and commit-count warnings require a split or an explanation in the PR body.
3. Re-read `git status`, every complete base diff, and every commit list, then repeat the outcome partition gate and mandatory stack check. Confirm there are no generated outputs, unrelated changes, secrets, independent outcomes combined in one PR, commits that cannot be mapped to that PR's outcome, duplicate bases, or broken links in the chain.
4. Push topic branches in parent-to-child order with upstreams. Do not force-push.
5. Fill `.github/pull_request_template.md` for each PR with one standalone acceptance outcome, concrete changes, exact verification results, commit count, production additions, related PRs, and commit cohesion. Every child PR must name its parent PR, current base branch, and complete merge order. `Why this cannot be split` must not be `None` for a multi-commit PR; explain why every commit is necessary for the same acceptance criterion or split the PR.
6. Create each PR in parent-to-child order, non-interactively with explicit `--base`, `--head`, `--title`, and `--body-file`. Pass the declared immediate parent branch to `--base`; only the first PR may use the integration branch. The title uses the primary prefix only and is at most 72 characters.
7. Verify every created PR using `gh pr view --json url,title,baseRefName,headRefName,commits,additions,deletions,files`. Compare all returned base/head pairs together and confirm they form exactly the announced linear chain before reporting completion.

Do not merge the PR unless the user separately and explicitly requests it. After verifying the PR, remain on its head branch for CI or review follow-up; do not switch to `main` or delete branches unless separately requested. Finish by reporting the PR URL, base/head, commits, checks, current branch, and any policy exception.
