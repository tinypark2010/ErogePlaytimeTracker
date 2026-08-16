# Eroge Playtime Tracker

Windows上で、登録したVisual Novel / エロゲが最前面にある時間を記録するローカル完結型デスクトップアプリです。1回の起動を `PlaySession`、その中の最前面期間を `FocusInterval` としてSQLiteへ保存します。

## 技術スタック

Rust、Tauri 2、Svelte 5、TypeScript、SQLite (`rusqlite`)、Windows API (`windows` crate)、reqwest、scraperを使用しています。

## 開発環境

- Windows 10 / 11 x64
- Rust stable（MSVC toolchain）とVisual Studio C++ Build Tools
- Node.js 20以降、npm
- Microsoft Edge WebView2（通常のWindows 10/11には導入済み）

```powershell
npm install
npm run tauri dev
```

フロントエンドだけを確認する場合は `npm run dev` を使用できますが、Tauri commandは利用できません。

## テストと品質確認

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
npm run check
npm run test
npm run build
```

Rustテストにはgame単位のsession遷移、launcherを含む複数exe、複数ゲーム間のforeground遷移、集計・重複validation、fixtureベースのErogameScape parser検証が含まれます。

## Windows build / installer

```powershell
npm run tauri build
```

NSIS形式のinstaller exeが `src-tauri/target/release/bundle/nsis/` に生成されます。設定はWebView2 download bootstrapperを使用するため、利用者がRust、Node.js、.NET、開発SDKを手動導入する必要はありません。初回インストール時にWebView2取得のためネットワークが必要になる場合があります。

## 保存場所

`%LOCALAPPDATA%\ErogePlaytimeTracker\` 配下に保存します。

- `app.db`: SQLite database（WAL、foreign key有効、UTCのRFC 3339 timestamp）
- `thumbnails\`: download済みpackage image cache
- log: Tauri log pluginの標準app log directory

durationはDBへ重複保存せず、Session/Intervalのtimestampからquery時に算出します。

## Tracking方式

登録exeをWindows APIで列挙し、1本のゲームに属するprocessが0→1以上でSessionを開始、1以上→0で終了します。そのためlauncher.exeからgame.exeへ移行してもSessionは二重になりません。

foregroundは `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` を主な通知経路とし、`GetForegroundWindow` / `GetWindowThreadProcessId` / process image pathで登録exeと照合します。通知取りこぼし対策として既定3秒（2〜30秒）のreconciliationも実行します。ゲーム間の直接切替や非ゲームへのAlt+Tabでは、同じSession内のFocusIntervalを適切に閉じて再開します。

終了時はopen recordを現在時刻で閉じます。異常終了後はperiodic `last_seen` より後を加算せず、orphan recordを閉じて `needs_review` にします。

## ErogameScape連携

game IDまたはgame URLからtitle、brand、発売日、package imageを取得します。HTML selectorは `GameMetadataProvider` / `ErogameScapeProvider` 内に隔離されています。thumbnailは一度だけcacheされ、networkやparser failureはtracking loopへ影響しません。サイトへの自動アクセスは明示的な取得・更新操作時だけです。

本アプリはErogameScapeおよび各ゲームメーカーの公式アプリではありません。取得したpackage imageは利用者のPC内にのみcacheし、repositoryやinstallerには同梱しません。各画像・ゲーム情報に関する権利は、それぞれの権利者に帰属します。サイト運営者または権利者から要請があった場合は、連携方法を見直します。

## Release

バージョンを `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` の3箇所で一致させ、同じバージョンのtagをpushするとGitHub ActionsがWindows x64 installerを自動生成します。

```powershell
git tag v0.1.0
git push origin v0.1.0
```

workflowはfrontend/Rustのcheck・test、依存ライセンス監査、第三者ライセンス一覧生成を通過した場合だけGitHub Releaseを公開します。生成されたNSIS installerには `LICENSE` と `THIRD_PARTY_LICENSES.txt` が同梱されます。現時点ではコード署名を行っていないため、Windows SmartScreenの警告が表示される場合があります。

## License

アプリ本体は [MIT License](LICENSE) で提供します。Rustおよびnpm依存関係にはそれぞれのライセンスが適用され、配布時の一覧は `npm run licenses` で生成します。

## 既知の制約

- Windows専用です。管理者processなど、OSがimage path取得を拒否するprocessは検出できない場合があります。
- 実行ファイルは現状フルパスを入力して登録します。全disk自動探索は行いません。
- ErogameScapeのHTML構造変更時はprovider selectorの更新が必要です。
- reconciliation間隔の変更はtracker再起動後に反映されます。
- crash recoveryは安全側に倒してlast-seenで閉じ、該当Sessionを要確認にします。失われた数秒は履歴編集画面で修正できます。
