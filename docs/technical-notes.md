# Technical notes

この文書は、開発・保守・リリースに必要な技術情報をまとめたものです。実装時の規約とdomain invariantは [AGENTS.md](../AGENTS.md)、branch・commit・pull requestの運用は [development-workflow.md](development-workflow.md) を参照してください。

## 技術スタック

Rust 2024、Tauri 2、Svelte 5、TypeScript、SQLite (`rusqlite`)、Windows API (`windows` crate)、reqwest、scraperを使用しています。serverやcloud accountはありません。

## 開発環境

- Windows 10 / 11 x64
- Rust stable（MSVC toolchain）とVisual Studio C++ Build Tools
- Node.js 20以降、npm
- Microsoft Edge WebView2（通常のWindows 10/11には導入済み）

```powershell
npm install
npm run tauri dev
```

lockfileどおりのclean installには`npm ci`を使用します。`npm run dev`はport 1420のfrontend-only previewであり、Tauri command、native tracking、local asset protocolは利用できません。

## テストと品質確認

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run format:check
npm run check
npm run commit-policy:test
npm run test
npm run build
```

TypeScript、Svelte、CSS、JSON、Markdownなどの整形には`npm run format`、Rustの整形には`cargo fmt --manifest-path src-tauri/Cargo.toml`を使用します。Rust testsにはgame単位のsession遷移、launcherを含む複数exe、複数game間のforeground遷移、集計・重複validation、fixtureベースのErogameScape parser検証が含まれます。

## Windows build / installer

```powershell
npm run tauri build
```

NSIS形式のinstallerが`src-tauri/target/release/bundle/nsis/`に生成されます。WebView2 download bootstrapperを使用するため、利用者がRust、Node.js、.NET、開発SDKを手動導入する必要はありません。初回install時にWebView2を取得するため、network接続が必要になる場合があります。

## 保存場所

dataは`%LOCALAPPDATA%\ErogePlaytimeTracker\`配下に保存します。

- `app.db`: SQLite database（WAL、foreign key有効、UTC RFC 3339 timestamp）
- `thumbnails\`: download済みpackage image cache
- `screenshots\`: 撮影したscreenshot
- `backups\`: import直前に自動作成される復旧用`.eptbackup`
- log: Tauri log pluginの標準app log directory

durationはDBへ重複保存せず、sessionとintervalのtimestampからquery時に算出します。
Screenshotの文字起こしは同梱したPP-OCRv5 mobileの検出・認識modelを`paddleocr_rs_onnx`とONNX Runtimeでon-demand実行します。modelは初回実行時にprocess内で初期化し、その後は再利用します。UIで指定した範囲はnormalized coordinatesとしてcommandへ渡し、Rust側で元画像からcropします。画像や認識結果をnetworkへ送信せず、認識結果はDBへ保存しません。

## Backup / restore

設定画面から作成する`.eptbackup`は、SQLiteの一貫したsnapshot、参照中のthumbnailと任意のscreenshot、format/schema version、各fileのSHA-256を含むZIP archiveです。screenshotを除外した場合はsnapshot側の`game_screenshots`も空にし、その選択をmanifestへ記録します。archive作成はfileを読みながらchecksum計算と圧縮を1 passで行い、処理済みbyte数をIPC channelでUIへ通知します。snapshot内のmedia pathはarchive相対pathへ変換し、import先を検証した後で新しい`%LOCALAPPDATA%`配下の絶対pathへ書き換えます。game executable pathは保持しますが、game本体のfileは含めません。

Importはmergeではなく全置換です。archive path、重複entry、symlink、size、checksum、SQLite integrity/foreign key、schema versionをactive dataへ触れる前にstaging領域で検証します。確定時に現在のdataを`backups\`へ自動exportし、pending markerを書いて再起動します。次回起動時のdata directory切替に失敗した場合や切替中に中断された場合はrollback directoryから元のdataへ戻します。対応済みの古いschemaはstaging内でmigrationし、新しいschema versionのbackupは対応versionへappを更新するまで拒否します。移行元の設定は復元しますが、`last_seen`とskip中のupdate versionは移行先向けにresetし、autostartは移行先のWindowsへ再適用します。

## Tracking方式

登録exeをWindows APIで列挙し、1本のgameに属するprocessが0から1以上になったときにsessionを開始し、1以上から0になったときに終了します。launcherからgame本体へ移行してもsessionは二重になりません。

foregroundは`SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`を主な通知経路とし、`GetForegroundWindow`、`GetWindowThreadProcessId`、process image pathで登録exeと照合します。windowのcreate、destroy、show、hide通知とvisible top-level windowの列挙も使用します。Backgroundは、関連processとvisible windowが存在する一方で、そのgameがforegroundではない状態として記録します。関連PIDはprocess handleでも監視し、終了通知の直後に再照合します。通知取りこぼしから復旧するため、3秒間隔のreconciliationも実行します。playtimeはsessionの起動時間からbackground intervalの合計を引いて算出します。

旧versionへのrollback compatibilityのため、従来の`focus_intervals` tableは削除せず、compatibility mirrorとして新しいrecordにも併記します。旧dataは初回起動時にfocus intervalの補集合をbackground intervalとしてmigrationし、移行前後のplaytime秒数が一致した場合だけ確定します。

起動直後など、関連processはあるがvisible windowがまだない期間はBackgroundとして扱いません。gameのvisible windowが消えた後も一時的なBackground recordを作らず、最後にwindowが消えたtimestampでsessionを閉じます。異常終了後はperiodic `last_seen`より後を加算せず、orphan recordを閉じて`needs_review`にします。要確認のsessionは詳細画面で復旧理由を表示し、記録を修正するか、内容に問題がなければ確認済みにできます。

headerのtracking statusはgame単位のphase（起動中、プレイ中、Background、画面切替・終了処理中）から複数chipを同時表示します。同じphaseが複数本ある場合は件数へ集約し、各titleはchipのtooltipで確認できます。

## ErogameScape連携

game IDまたはgame URLからtitle、brand、発売日、package imageを取得します。HTML selectorは`GameMetadataProvider`と`ErogameScapeProvider`内に隔離されています。thumbnailはcacheされ、networkやparser failureはtracking loopへ影響しません。siteへの自動accessは明示的な取得・更新操作時だけです。

本appはErogameScapeおよび各game makerの公式appではありません。取得したpackage imageは利用者のPC内にのみcacheし、repositoryやinstallerには同梱しません。各画像とgame情報に関する権利は、それぞれの権利者に帰属します。site運営者または権利者から要請があった場合は、連携方法を見直します。

## Release

versionを`package.json`、`package-lock.json`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock`、`src-tauri/tauri.conf.json`で一致させ、同じversionのannotated tagをpushするとGitHub ActionsがWindows x64 installerを生成・公開します。

```powershell
git tag -a v0.1.0 -m "Eroge Playtime Tracker v0.1.0"
git push origin v0.1.0
```

workflowはfrontendとRustのcheck・test、dependency license audit、third-party license一覧生成を通過した場合だけGitHub Releaseを公開します。生成されたNSIS installerには`LICENSE`と`THIRD_PARTY_LICENSES.txt`が同梱されます。現在はcode signingを行っていないため、Windows SmartScreenの警告が表示される場合があります。

## License

app本体は [MIT License](../LICENSE) で提供します。Rustとnpmのdependency、同梱model、native runtimeにはそれぞれのlicenseが適用されます。`third-party/assets.json`ではCargo/npm外の配布物についてsource revision、SHA-256、legal textを管理し、`npm run licenses`で整合性を検証して一覧を生成します。`THIRD_PARTY_LICENSES.txt`はgenerated fileのためGit管理せず、Tauri buildとGitHub Actionsがinstaller作成前に生成します。

## 既知の制約

- Windows専用です。管理者processなど、OSがimage path取得を拒否するprocessは検出できない場合があります。
- 実行ファイルはfile pickerで個別に登録します。disk全体の自動探索は行いません。
- ErogameScapeのHTML構造変更時はprovider selectorの更新が必要です。
- crash recoveryは安全側に倒して`last_seen`で閉じ、該当sessionを要確認にします。失われた数秒は履歴編集画面で修正できます。
