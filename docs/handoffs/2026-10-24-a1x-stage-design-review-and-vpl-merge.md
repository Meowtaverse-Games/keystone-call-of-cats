# セッション引き継ぎ: 20面設計の確定作業（A1X）と PR #88 取り込み後の追従

**このタスクの再開時は本ファイルを先に読む。** 2026-09-29 の2本（[20面候補](2026-09-29-twenty-stage-candidates-and-validation.md)、[Stage 13 試遊ビルド](2026-09-29-stage13-reviewed-playtest-build.md)）は経緯の資料として残す。現在地・判断待ち・次の一手は本ファイルで更新した。

## 対象

- リポジトリ: `Meowtaverse-Games/keystone-call-of-cats`
- 作業ホスト: **A1X**（Windows 11 + WSL2、distro `Sasara`）。2026-10-05 に Oracle（`sasara-oracle`、ARM 4コア）から移した。Oracle は SSH（`ssh sasara-oracle`、Tailscale 名）で入れる。
- worktree: `/home/ubuntu/repos/meowtaverse/keystone_cc`（WSL）。Oracle でも同じパスに旧 checkout がある。
- 確認済み Git toplevel / branch / source code HEAD: `/home/ubuntu/repos/meowtaverse/keystone_cc` / `feature/20-stage-release-handoff` / `067e7bfdc3b94f4660330674403b43b1e49ef9ae`（本 handoff コミットの直前）
- worktree一覧の確認: A1X の WSL はこの1つだけ。Oracle 側の worktree は下記「Oracle」を参照。
- 記録時の remote: `origin/main` = `4f31f851706d305f86077a02b86e451aae75890c`（#88 マージ後）、`origin/feature/stage13-playtest` = `69530d9f83103432319b5020ef39a449c2badb27`（PR #87、未マージ）。
- タスク: main の Stage 1〜12 を生かし、13〜20 を設計・検証して、面ごとの小さな PR で main に入れる。ユーザーの直近の方針は「**まず20面の設計を終える。気になる点は TODO に残して進める**」「main に入った変更へこのブランチを追従させ、その上で設計を進める」。
- 記録日時: 2026-10-24（JST）

## 現在地

### 進め方（2026-10-23 にユーザー了承）

| 段階 | 内容 | 状態 |
| --- | --- | --- |
| 0 | 14〜20 の地図・石の役割・想定コードをまとめ、ユーザーが採否を判断 | 資料完成。**判断待ち** |
| 1 | 危ない所の確認: 接触待ちを製品の実行器で確かめる（Astra） | 指示書完成。**渡す承認待ち** |
| 2 | Stage 13（PR #87）を main へ。`place right` 表示の食い違いを直してからマージ | 未着手 |
| 3 | 14〜16 を面ごとの PR → 章として試遊 | 未着手 |
| 4 | 17〜19 を面ごとの PR | 未着手 |
| 5 | 20 ＋ 最終面クリア処理の修正 ＋ HTTP 解放 ＋ `list.ron` の 21〜23 枠の整理 | 未着手 |
| 6 | 20面通しプレイ | 未着手 |

### 資料（段階0）

- Markdown: [design/stages-14-20-design-review.md](../../design/stages-14-20-design-review.md)（コミット済み）
- 閲覧用ページ（非公開の claude.ai Artifact。別セッションから更新するときは URL を `url` に渡す）:
  - Stage 14–20 設計レビュー: https://claude.ai/artifact/WT5eGiWbvmJaA8sa5Z2FU9
  - Stage 1–13 解法レビュー（main の 1〜12 と PR #87 の 13）: https://claude.ai/artifact/HcF4cdMLGvoKJrufnPz9UX
  - HTML の元ファイルの控え: `C:\Users\tnobo\keystone-playtest\handoff-2026-10-24\`（WSL からは `/mnt/c/Users/tnobo/keystone-playtest/handoff-2026-10-24/`）

### ユーザーの判断待ち

1. **Stage 19**: 2石が同じコード（右を2回掘って穴へ下りる）で、[最終提案](../../design/stage-design-final-proposal.md)の「2石に同じ掘削をさせるだけにしない」と食い違う。① このまま採用 ② 片方の役割を変える（地形の作り直しと CLI 再検証） ③ 掘る量や方向を2石で変える（小さな手直し）。
2. **Stage 17**: Type1 の2石目は `sleep` で待つ。秒数合わせを本編に入れてよいか（余裕を大きく取れば方針と矛盾しない見込み）。
3. **Stage 14 と 15** が似すぎていないか。**Stage 20** が締めくくりとして十分か。
4. **段階1の指示書を Astra に渡してよいか**（下記）。
5. **Oracle の checkout** の未コミットの追記を捨てて最新に揃えてよいか（内容は A1X から `a24ec8d` でコミット済み）。

### 段階1: 接触待ちの調査（Astra への指示書）

- 指示書: `C:\Users\tnobo\keystone-playtest\handoff-2026-10-24\astra-brief-contact-wait.md`（repo には入れない方針）。
- 中身: Type2 の石が「触れられるまで待つ → N 回動く → 終わる」を Keystone と Rhai で書けるかを、実行器の単体テストで固定する（600 ステップ待っても固まらない・エラーにならない、触れたら N 回、途中で離れても最後まで、Type2 の命令制限内）。製品の動作は変えない。書けない言語があれば止めて `needs_decision`。
- 設定: ブランチ `feature/contact-wait-idiom-tests`、base main `4f31f85`、worktree `/srv/sasara/worktrees/keystone_cc/feature/contact-wait-idiom-tests`（A1X の WSL に作る）、モデル `gpt-6.1-sol` effort high。
- 意義: Stage 18 だけでなく、main の Stage 7〜12（Type2・Type3）も「乗るまで接触で待つ」想定解になる。
- Keystone の第一候補（#88 で空本体が書けるようになった）:

```text
while not is_touched()
end
```

- Rhai 候補: `loop { if is_touched() { break; } }`。実行器の操作数上限（`set_max_operations`）に当たらないかが未確認。

### PR #88（VPL ブロック）の取り込み

- 2026-10-24 にユーザーが手動でマージ（main `4f31f85`、squash）。
- マージ前に独立レビュー（head `247f24c`、keystone-blocks `833d678`、keystone-lang `267d9cc...fc8e3f4`）: **approved**、P1/P2 なし。10/22 に出した4指摘（括弧の優先順位、壊れたコードの書き戻し、rev 固定、石ごとのパレット制限）は根拠付きで修正済み。
- keystone-lang `fc8e3f4` で `loop` / `while` / `if` の空本体が可能になり、空本体は1回ごとに `Event::Tick` を返す。ゲームは1フレームに1イベント処理するので、空の `while` で固まらない見込み（コード読解。実行器テストは段階1）。
- P3 のうち3件を Hub backlog に登録（下記）。P3-4（VPL 編集でコードの書式が整え直される）は仕様として受け入れ。
- 本ブランチに `origin/main` を取り込み済み（`1ae233a`、競合なし）。取り込み後に WSL で `cargo test --locked --no-default-features` 69件成功。

### Stage 1〜4 の CLI 試行（2026-10-23、main の地形）

- Stage 2: `move down` で 100/100（88地形）。
- Stage 3: プレイヤーが乗ってから右へ6〜7回で 52/100（83地形）。残りの原因は未調査。
- Stage 4: 上1〜9回 × 右6〜14回の組み合わせはすべて失敗。プレイヤーを石に乗せる手順が足りない可能性。未解決。
- 試行スクリプト: `handoff-2026-10-24/main-stages/`（スクリプト内のパスは旧 scratchpad を指すので、使うときは書き換える）。

### 試遊の TODO

- Stage 13 の試遊結果と保留 TODO 11件は [PLAYTEST.md](../../design/prototypes/stages-13-20-v1/PLAYTEST.md) の「Stage 13 A1X本人試遊（2026-10-12）」に記録済み。主なもの: 足場なしのジャンプ突破は未確認、Type4 共通説明の `place right` が面の説明と食い違う（#87 で直す）、最終面クリア時の表示が目立たない・F3 でリセットされない・存在しない面まで解放（Stage 20 と一緒に直す）。

## 環境（A1X）

- **WSL**: Rust 1.99.1（rustup、`~/.cargo`）、clippy、rustfmt。apt で `pkg-config libasound2-dev libudev-dev libwayland-dev libxkbcommon-dev` を導入済み（2026-10-23、ユーザー承認）。本体の初回テストビルド 10分46秒。codex は `~/.local/bin/codex`。
- **Windows ネイティブ**: Rust 1.98.1（MSVC）と VS Build Tools 2026 が入っている。ユーザー PATH に `%USERPROFILE%\.cargo\bin` が無いので、スクリプトで補っている。
- 試遊用の作業フォルダ: `C:\Users\tnobo\src\keystone_cc`。現在は**手元専用ブランチ `local/playtest-main-stage13`**（`d1047c2` = `origin/feature/stage13-playtest` に `origin/main` を手元でマージ。upstream なし、push しない）。debug ビルド済み（78秒、テスト73件成功）。`assets/fonts` と `assets/images` は CI と同じく `ext-assets` のコピーに置き換えてある（git 上は削除扱いで正常）。
- デスクトップのショートカット「Keystone Call of Cats (dev)」: 上記の `target\debug\keystone-cc.exe` を作業フォルダ指定で起動する。VPL 入り＋Stage 13。
- スクリプト: `C:\Users\tnobo\keystone-playtest\scripts\`（`native-build.ps1`、`launch-native-debug.ps1`、`update-local-playtest.ps1`、`create-desktop-shortcut.ps1`、`capture-game-window.ps1` など）。
- **注意: セーブの隔離が効いていない。** `KEYSTONE_CI_SAVE_DIR` は `--ci-smoke` のときだけ有効。`LOCALAPPDATA` の差し替えも効かなかった（原因未特定）。試遊は通常セーブ `%LOCALAPPDATA%\KeystoneCC` に書き込む。現在、通常セーブに試遊の状態（`unlocked_until:(14)`、Stage 13 の `place up`）が残っている。試遊前のバックアップは `C:\Users\tnobo\keystone-playtest\save-backups\`。戻すかはユーザー判断。

## Oracle（`sasara-oracle`）

- checkout は `feature/20-stage-release-handoff` の `1d2abc0` のまま、A1X 追記が未コミット（判断待ち 5）。
- 10/20〜10/22 の keystone_cc の Claude セッションは PR #88 のレビューで、ステージ作業は進んでいない。
- Oracle にだけ残るブランチ: `fix/preserve-original-stage-terrain`（9/27 の承認前の地形復元 WIP と A1X 検証記録、未 push）、`feature/stage8-playful-prototype`（Stage 8 試作、remote なし）。片付ける前に残すか決める。`chore/keystone-lang-pin-main` は main に取り込み済み、`feat/external-stone-control` は #85 として取り込み済み。

## 続行範囲

- 許可済みの範囲: docs・設計資料の編集とこのブランチへのコミット・push、CLI での検証、A1X での試遊ビルドと起動。
- 要承認: Astra への受け渡し（判断待ち 4）、PR のマージ、GitHub への投稿（コメント・承認レビューは**文面を先に見せる**）、Oracle の checkout の変更。
- 実施済み（繰り返さない）: A1X のビルド環境の準備、#88 の独立レビューと取り込み、P3 の Hub 登録、Stage 2〜4 の CLI 試行、2つの閲覧用ページの作成。
- 次の正確な一手:
  1. 本ブランチの HEAD・`origin/main` を確認する（main が進んでいれば取り込んでテスト）。
  2. 判断待ち 1〜5 をユーザーに確認する。2つのページを見て判断してもらう。
  3. 承認されたら、指示書の base SHA を最新 main に合わせて worktree を作り、`delegate-to-astra` の手順で Astra に渡す。
  4. Stage 19 の方針が決まったら、候補 RON と CLI 検証（`design/prototypes/stages-13-20-v1/verify.sh` の該当面）を更新し、両方の資料に反映する。

## 永続先

- handoff コミットと push: 本ファイルのみを `feature/20-stage-release-handoff` にコミットし、同じ remote ref へ push する（結果は最終応答で報告）。
- コミット: `a24ec8d`（Stage 13 A1X 追記）、`6babbf1`（試遊 TODO）、`7f78888`（14〜20 設計資料）、`1ae233a`（main 取り込み）、`067e7bf`（空の while への更新）。
- PR: #82（統合 Draft、一括マージしない）、#87（Stage 13、Draft・未マージ）、#88（VPL、マージ済み `4f31f85`）。
- Sasara Hub（backlog に登録）:
  - `7a810b59-799d-4eb8-a0a1-160d6a96f256` u32 を超える数値リテラルで panic（keystone-lang）
  - `c41d9b65-deae-41da-824d-0a693b514b70` VPL の数値欄で大きな値が巻き戻る（keystone-blocks）
  - `58d9fb85-d688-4286-9857-5862cf47864a` VPL の未保存編集が別ステージに持ち越される（keystone-blocks）
  - 既存: VPL を experimental で隠すか、翻訳対応、エディタ欄のはみ出し、全角文字の消失（10/22 登録）。Keystone の Steam 提出タスク（active）があり、VPL を製品に出すかの判断に関わる。
- このタスク自体の進行は Git（本ファイルと design/）を永続先とする。
