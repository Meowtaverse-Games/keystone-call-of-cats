# セッション引き継ぎ: Keystone 1.0 の年内 Steam リリースに向けた進行

## 対象

- リポジトリ: `Meowtaverse-Games/keystone-call-of-cats`（関連: `keystone-lang`, `keystone-blocks`）
- worktree: `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff`
- 確認済みGit toplevel / branch / source code HEAD: 上記 / `feature/20-stage-release-handoff`（PR #82, draft） / `2748223`（旧 handoff 10 件の削除コミット。origin と一致）
- worktree一覧の確認: 本 worktree、`/home/ubuntu/repos/meowtaverse/keystone_cc`（main, ローカルは `4f31f85` で origin/main `c931320` より古い）、`/srv/sasara/worktrees/keystone_cc/feature/stage-categories`（#99, `c95cf37`）
- タスク: 12/28 に Steam で 1.0 をリリースする。そのためのオーナー側タスクと 林君 側タスクを期限どおり進める
- 記録日時: 2026-11-23

## 現在地

- 結論:
  - 林君 の PR は lang #7・blocks #5（merge commit）と cc #102（`c931320`）までマージ済み。マージ済みブランチは削除済み。
  - cc main の pin は lang `275eaaa`、blocks `57a4f8d`。#99 / #103 の pin も各 main から到達できることを確認済み。
  - リリース作業は、Slack トラッカー `F08517J9U3F`（オーナー行は【リリース前】）と Sasara Hub のタスク（タグ `release-1.0`）に、同じ期限で 1 対 1 に登録済み。
  - 旧 handoff 10 件は内容をトラッカー・Hub・マスターシート・git に移したうえで削除済み（`2748223`）。
- スケジュール（Hub decision `aa45c24e`）:

  | 期限 | 内容 | 担当 | Hub |
  |---|---|---|---|
  | 11/24 | #99 を Windows で確認してマージ | オーナー | `1c509aa6` |
  | 11/27 | 1.0 の範囲を決める（VPL・#103 の Rhai ブロック・HTTP・間に合わない面） | オーナー | `142c679f` |
  | 11/30 | PR B: 公開中の全面を CI で確認、エージェント修正 | 林君 | `b0457c53` |
  | 12/3 | Steam ストアページを作って審査に出す | オーナー | `fbc30f6e` |
  | 12/5 | PR C: 置く 1〜4 | 林君 | `be94440f` |
  | 12/10 | PR D〜F: 動かす・見る・ほる | 林君 | `853fac0e` |
  | 12/12 | PR G: 伝える 1〜4 | 林君 | `a0be4c1b` |
  | 12/12 | 最終面のクリア処理 / 3 言語の説明文 / Type5 画像 / 高リフレッシュで石が止まる不具合 | オーナー | `2871ff2f` / `4b9d1994` / `5f7f5663` / `c4124a21` |
  | 12/15 | 20 面通しプレイ、ビルドを Steam 審査へ | オーナー | `2881b622` |
  | 12/28 | Steam リリース（手動リリース） | オーナー | `c989131e` |

- 根拠: Steamworks の規定（ストアページ審査と Coming Soon 2 週間以上、ビルド審査はストアページ提出後、各審査は 7 営業日見込み）から逆算。登録料・税・口座は登録済み。
- 未解決の不確実性:
  - 林君 の作業日は土曜（11/28, 12/5, 12/12）のみで、PR B〜G を約 3 週間で終えるのは余裕がない。11/27 の範囲決定で削る面を決める。
  - cc #103（draft, `feature/rhai-blocks`, head `a693d55`）は Keystone 限定の VPL ゲートを外し Rhai ブロックを許す。1.0 に入れるかは 11/27 に決める。#99 とは競合しない。
  - VPL 関連 3 件（`fbce852a` 翻訳, `67d51f6d` はみ出し, `3781c6b3` 全角）は backlog・期限なし。VPL を 1.0 に出すと決めたら期限を入れる。
  - 林君 の blocks ループ PR と #103 は Review になったらレビューする。
- 未コミット変更・未追跡artifact: なし。この handoff だけを追加コミットする。
- コードの既存コミット: cc main `c931320`、#99 `c95cf37`、#103 `a693d55`

## 続行範囲

- 許可済みの範囲: 状態確認、レビュー、トラッカー・Hub の更新、林君 への文面の下書き提示。マージ・ビルド・Steam への提出・リリースは、その都度オーナーの明示的な承認が要る（以前の承認は引き継がない）。
- 実施済み（繰り返さない）: lang #7 / blocks #5 / cc #102 のマージ、ブランチ削除、トラッカーと Hub への登録、旧 handoff の削除、blocks #5 の LGTM 画像の差し替え（vetted `133276`）。
- 次の正確な一手: #99 の head が `c95cf37` のままか、main との競合がないかを確かめる。そのうえでオーナーに Windows での確認結果を聞き、承認があればマージして Hub `1c509aa6` とトラッカーの行を Done にする。

## 永続先

- handoffコミットとpush: `origin/feature/20-stage-release-handoff`（PR #82）へ push
- コミット / PR / 外部状態: cc main `c931320`、PR #99 / #103 / #82 open、Slack トラッカー `F08517J9U3F`
- Sasara Hub: decision `aa45c24e`、event `af475bbc`（マージ）・`b18e11fd`（ブランチ削除）、上表のタスク（タグ `release-1.0`）
