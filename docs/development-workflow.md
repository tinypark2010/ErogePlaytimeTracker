# Development workflow

この文書はbranch、commit、pull request運用のsource of truthです。repository固有のbuild/test方法はルートの [AGENTS.md](../AGENTS.md) と [README.md](../README.md) を参照してください。

## Protected integration branch

- Integration branchは`main`です。`master`ではありません。
- `main`上でcommitしません。`origin/main`へ直接pushしません。
- 通常の変更はtopic branchからpull requestとしてmergeします。
- PR作成の依頼はbranch作成、commit、検証、topic branchのpush、PR作成を許可しますが、mergeまでは許可しません。
- 単なる実装依頼はcommit、push、PR作成を暗黙に許可しません。

作業開始時は`main`をfast-forwardで更新してから、1 PRにつき1本のbranchを作ります。既にworking treeに変更がある場合は、変更を捨てずに現在のHEADからtopic branchを作成します。

```powershell
git switch main
git pull --ff-only
git switch -c update/short-description
```

branch名は`<type>/<short-kebab-description>`とします。typeは変更のprimary commit prefixに合わせます。

```text
update/add-library-filter
fix/prevent-update-during-tracking
docs/development-workflow
ci/pull-request-checks
```

## Commit scope

- 1 commitは1機能、1不具合、または1つの独立した保守目的だけを扱います。
- 同じprefixでも機能AとB、bug CとDは別commitにします。
- 実装と、その実装を直接検証するtestは同じcommitに含めます。
- test追加が現実的でないbehavior変更では、PRのVerificationに理由とmanual testを記録します。
- 必須のdocumentation、migration、lockfile、generated metadataは、それが属する変更と同じcommitに含めて構いません。これらは別の変更目的とは数えません。
- dependency変更と、それにより更新されたlockfileを分離しません。
- broad formattingは`[format]` commitへ分離し、behavior変更と混ぜません。触った行だけのincidental formattingはprimary変更に含めて構いません。
- 各commitは可能な限りbuild可能で、対応するcheckが通る状態にします。
- 最終PRに`WIP`、`fixup!`、一時debug commitを残しません。
- unrelatedなworking-tree変更や、ユーザーが別目的で作成した変更をstageしません。`git add .`ではなく明示的なpathまたはhunkをstageします。

### Size guidelines

行数はsoft guidelineであり、単一責務より優先しません。

- 1 commitあたりproduction code追加200行以内を目安にします。
- standalone test、docs、lockfile、generated file、純粋なformat変更は200行の集計から除外します。Rustのinline testはproduction fileと同じ行数へ含まれるため、表示値はあくまで目安です。
- 200行を超えたら、別の責務や別機能を含んでいないか再検討します。
- 分割するとbuild不能になる、migrationと利用codeが離れる、実装とtestが離れる場合は分割しません。
- 小さなdiffでも複数の成果を含むなら分割します。

`npm run commit-policy:check -- --base <base> --head <head>`はcommitごとのproduction additionsを表示し、200行超過をwarningにします。

## Commit messages

subjectは次の形式です。

```text
[prefix] Imperative summary
[prefix, test] Imperative summary
```

### Primary prefixes

- `[update]`: 機能追加、既存機能の改善、意図的なbehavior変更
- `[fix]`: 不具合修正
- `[refactor]`: 外部behaviorを変えない構造整理
- `[docs]`: documentationのみ
- `[build]`: dependency、build、packaging
- `[ci]`: CIまたは検証automation
- `[format]`: formattingのみの機械的変更
- `[release]`: version更新などrelease準備
- `[chore]`: 他のprefixに該当しない限定的なrepository保守
- `[test]`: product codeを変更しないtest追加・改善

primary prefixは1つだけです。対応testを同じcommitに含める場合に限り、`test`を唯一のsecondary prefixとして追加できます。許可する組み合わせは次だけです。

```text
[update, test]
[fix, test]
[refactor, test]
[build, test]
[ci, test]
```

`[update, fix]`、`[update, docs]`、3個以上のprefixなど、2つのprimary目的を表す組み合わせは禁止します。documentationを伴う機能実装のprimaryは`update`であり、`docs`をsecondaryにしません。

subjectはprefixを含め72文字以内を目安ではなく必須とし、末尾にperiodを付けません。repository履歴との一貫性のため、簡潔な英語の命令形を使用します。

詳細が必要な場合は空行を1行挟み、1〜3行のbulletでwhat/whyを記載します。subjectだけで十分なら本文を省略します。

```text
[fix, test] Prevent updates while games are running

- Block installation from both update entry points
- Cover the active-tracking guard
```

message形式は`npm run commit-policy:check`で検証します。公開済みcommitは、ユーザーの明示的な許可なしにamend、rebase、force-pushしません。

この規約より前の`main`履歴は遡及修正しません。CIとvalidatorはpull requestのbaseからheadまでに追加されたcommitだけを検査します。

## Pull request scope

- 1 PRは1つの成果または問題だけを扱います。
- test、documentation、migration、必要なpreparatory refactorは、同じ成果に不可欠なら同一PRに含められます。
- 原則として5 commits以内、production code追加500行以内とします。
- 目安を超えたら分割を再検討します。分割不能ならPR本文の`Why this cannot be split`に理由を記録します。
- failed checkがある状態でready PRを作りません。未完了状態の共有を明示的に依頼された場合だけdraft PRを使います。
- unrelatedなlocal変更はPRに含めません。

### Title

PR titleはrelease notesで単独でも理解できる成果を、primary prefix付き・72文字以内で表します。`test` secondary prefixはPR titleに付けません。

```text
[fix] Prevent updates while games are running
[update] Add play-status filtering
```

### Body

`.github/pull_request_template.md`に従い、最低限次を記録します。

- Purpose: 何を解決するか
- Changes: 何を変えたか
- Verification: 実行したcommandと結果、必要なmanual test
- Scope: commit数とproduction additions
- Related PRs: dependencyとmerge順
- Why this cannot be split: size/scope例外の理由。例外がなければ`None`

CodexはPR作成後、URL、base/head、commit一覧、検証結果、例外を報告します。PR作成の依頼だけではmergeしません。

## Multiple and stacked pull requests

複数の変更がlocalに存在していても、論理的に独立していればそれぞれ`main`からbranchします。conflictの可能性だけを理由にstackしません。単独管理で即時mergeする通常運用では、先行PRをmergeし、`main`を更新してから次のPRを作る方法を優先します。

BがAのcode、schema、APIを技術的に前提とし、Aのmerge前にBもPRとして公開する必要がある場合だけstackします。

```text
main <- update/a
update/a <- update/b
```

後続PRの本文には次を記録します。

```markdown
## Related PRs

- Depends on #123
- Merge order: #123, then this PR
- Current base: `update/a`
```

先行PRをmergeした後、後続PRのbaseを`main`へ変更し、先行PRのdiffが重複しないことを確認してからmergeします。stacked PRは先行順にmergeし、後続を先にmergeしません。

## Merge policy

- GitHubの`Create a merge commit`を使用し、review済みのcommit境界とmessageを保持します。
- Squash mergeは使用しません。commit規則で作った境界を失うためです。
- Rebase mergeはstacked branchのrewrite/force-pushを必要にしやすいため使用しません。
- topic branch内へ`main`のmerge commitを取り込まず、必要ならPR作成前に安全に更新します。
- branch deletionは、そのbranchをbaseにするstacked PRを`main`へretargetした後に行います。

## Repository enforcement

GitHub rulesetは`main`を対象にし、pull requestを必須とします。通常運用のbypass actorとapproval requirementは設定せず、required PR checksを通過したownerがmergeします。repository管理画面ではmerge commitを許可し、squash/rebase mergeを無効にします。

`.github/workflows/ci.yml`はstacked PRを含むすべてのpull requestでcommit policy、frontend/Rust checks、tests、build、dependency license生成を実行します。既存のrelease workflowはtag publish専用です。

このworkflowが一度成功した後、GitHubのrepository settingsで次を設定します。

1. `main`を対象にするactive rulesetを作成する。
2. `Require a pull request before merging`を有効にし、required approvalsは0にする。
3. bypass actorを追加しない。
4. `Audit Rust dependency licenses`と`Verify pull request`をrequired status checksにする。
5. repositoryのpull request merge methodは`Create a merge commit`だけを有効にする。
6. `main`とactive stacked PRのbase branchを削除対象から保護する。
