---
name: release
description: Prepare, publish, monitor, or diagnose this repository's Windows releases. Use when a user asks to bump or verify a release version, prepare a release change, create and push a release tag, publish a GitHub Release, or investigate the release workflow. Do not use for ordinary development builds or unrelated pull requests.
---

# Release

Handle release preparation separately from publication so that changing versions never implicitly publishes an installer.

## Establish the release state

Read the Release section of `docs/technical-notes.md` and `.github/workflows/release.yml`. Also read the root `AGENTS.md` and `docs/development-workflow.md` when they exist. Treat the files and workflow as the current source of truth rather than copying old commands from prior releases.

Inspect the current branch, working tree, `origin/main`, existing tags, and the five version-bearing files:

- `package.json`
- `package-lock.json` (top-level version and root package version)
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock` (the application package entry)
- `src-tauri/tauri.conf.json`

Run `scripts/check-release-version.ps1` from this skill with `-ExpectedVersion <version>` whenever a target version is known. Use stable `MAJOR.MINOR.PATCH` versions unless the user explicitly requests prerelease support; the current workflow always publishes with `prerelease: false`, so do not silently publish a prerelease version through it.

If the user requests only an audit, plan, or status report, keep the work read-only. Do not infer permission to edit files, commit, push, tag, rerun a workflow, or create a release.

## Prepare a release

When the user requests a version bump or release preparation:

1. Resolve the exact target version. Ask when choosing major, minor, or patch would materially change the user's intent; do not guess from commit history alone.
2. Confirm the target is greater than the current version and that neither a local nor remote `v<version>` tag already exists.
3. Follow the repository branch policy. Keep unrelated working-tree changes intact and put release preparation on a focused topic branch.
4. Update all five version-bearing files. Change only the application version and the corresponding lockfile entries; do not update dependencies as a side effect.
5. Run the version checker, inspect the complete diff, and confirm it contains no generated installer, `THIRD_PARTY_LICENSES.txt`, secret, dependency change, or unrelated edit.
6. Run the handoff checks required by `AGENTS.md`. A release-preparation commit uses the repository's `[release]` convention when the user has also authorized a commit.

Preparation does not authorize a commit, push, pull request, tag, or publication. If the user asks to open the preparation PR, follow the repository's PR workflow. Do not tag the topic-branch commit; publication begins only after the preparation change is merged to `main`.

## Publish a release

Only publish when the user explicitly asks to release or publish the exact version. Immediately before the tag push, report that the push will trigger a public, non-draft GitHub Release.

Before creating the tag:

1. Fetch `origin/main` and tags, then verify the working tree is clean, `HEAD` is the intended commit on local `main`, and local `main` equals `origin/main`.
2. Verify all five version values equal the requested version.
3. Verify `v<version>` is absent locally and on the remote. Never move, replace, force-push, or delete an existing release tag.
4. Confirm the release-preparation change is merged and the required checks for that `main` commit passed. Do not release directly from an unmerged topic branch.

Preserve the repository's annotated-tag convention: create `v<version>` at the verified `main` commit with the message `Eroge Playtime Tracker v<version>`, and push only that explicit tag ref. Never push all tags or push `main` as part of publication. Do not read, print, copy, or locally request `TAURI_SIGNING_PRIVATE_KEY`; GitHub Actions supplies it.

After pushing the tag, monitor the matching release workflow to completion. If it succeeds, verify the GitHub Release exists, is public, targets the expected tag/commit, and contains `latest.json`, updater signature data, and the expected Windows NSIS installer assets. If it fails, collect the failed job/step and relevant non-secret log excerpt, then stop. Do not delete and recreate the tag, rerun jobs, change secrets, or make a compensating release without explicit authorization.

## Finish the handoff

Report the version, commit SHA, tag, preparation checks, workflow result and URL, release URL, and published asset names. Clearly distinguish completed work from any manual or blocked step.
