# セッション引き継ぎ: PR #99（変わらないステージ ID とカテゴリ）を Windows で確かめてからマージする

前の handoff: `docs/handoffs/2026-11-06-category-prs-and-stage-ids.md`（設計の決定、PR A〜G の順番、林君への依頼の経緯）。このファイルはその続きで、PR A（#99）の現状と次の一手を正とする。

## 対象

- リポジトリ: `/home/ubuntu/repos/meowtaverse/keystone_cc`（GitHub `Meowtaverse-Games/keystone-call-of-cats`）
- worktree:
  - `/srv/sasara/worktrees/keystone_cc/feature/stage-categories`（PR #99。clean）
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff`（PR #82。閉じる予定。この handoff の置き場所）
- 確認済みGit toplevel / branch / source code HEAD:
  - `/srv/sasara/worktrees/keystone_cc/feature/stage-categories` / `feature/stage-categories` / `c95cf37146fe169cc10135abadd8a5522a5a87c4`（PR #99 の head と一致）
  - main: `d9a48a43a474b601d8702f539a63cc1f27589f23`（#99 の base。2026-11-06 の時点で動いていない）
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff` / `feature/20-stage-release-handoff` / `95bd54f8aafea5c4e992712f34b91009825166a1`（handoff だけのコミット。origin と一致）
- worktree一覧の確認: 共有の checkout（`main` だが `4f31f85` のまま古い。触っていない）と、上の 2 つだけ。
- タスク: PR A（#99）を本人が Windows で確かめ、マージする。続けて PR B、C を Astra に渡す。
- 記録日時: 2026-11-07T10:07+09:00

## 現在地

- 結論:
  - PR #99 は open。独立レビューは **approved**（head `c95cf37`、P1 なし、コードの修正は不要）。CI（lint / verify）は SUCCESS。
  - 本人の決定（2026-11-07）: **Windows で確かめてからマージする**。マージは本人がする（`--admin` での Claude のマージは、本人の明示の指示があるときだけ）。
  - レビューの P2（`tools/stage_sim` の既定の読み込み先が壊れた件）は、本人の承認を得て PR 本文の「残ったリスク」に 1 行足した。コードは変えていない。
  - フォローアップを Hub の task に積んだ（下の「永続先」）。
- 根拠:
  - Astra（`gpt-6-astra`）が 5 コミットを作成（作者 `Tokusei Noborio`、trailer `Implemented-by: Codex gpt-6-astra`）。最初の実行は、環境の git に作者が未設定で `blocked`。設定ファイルは変えず、コミットごとに `-c user.name/user.email` を指定させて再開した（effort medium）。
  - 再開のときに、main の en-US / zh-Hans の説明文の 1 面ずれも直させた。main の `stage11` は日本語の Stage 12 の訳だった。今は `dig-04` に移し、`dig-03` には新しい訳を入れた。
  - 独立レビュー（会話の経緯を渡さない subagent）が head で再実行した結果:
    - fmt、clippy（既定と `--no-default-features`）、`cargo test` 138 件（両方の機能の組み合わせ）、release ビルド
    - headless の検証（stage-1 の動きのハッシュが README と一致、move-02 の地形のハッシュが base と一致）、`snapshot-baselines.py`（変化なし）
    - 実際の ron 0.12.2 で旧い保存ファイルを作り、新しい形には読めず旧い形として移行されることを確認
  - `tools/ftl_sheet_exporter` のテストは、この機械に OpenSSL の開発用ファイルが無くビルドできない（`sudo` が要るので入れていない）。レビューで、変更した関数を取り出して試し、通った。
- 未解決の不確実性:
  - 画面の見た目（カテゴリのタブ、公開前の印）、実機でのタッチ、Steam Cloud への実際の保存は、窓の無い環境では確かめられていない。→ Windows での確認で見る。
  - ron 0.12 より前の版が書いた保存ファイルの書式は確かめていない。
  - 英語・中国語の説明文には、日本語の最新の文と合わない面がほかにもある（例: 旧 Stage 10）。D〜F のカテゴリの PR で揃える。
- 未コミット変更・未追跡artifact:
  - コードの未コミット変更は無い。
  - Windows の保存データ: `C:\Users\tnobo\AppData\Local\KeystoneCC\`（`stage_progress.ron`、`stage_scripts.ron`、`game_settings.ron`。最終更新 2026-10-30）。旧い形の本物の保存なので、移行の確認に使える。
  - Windows の clone は `C:\Users\tnobo\src\keystone_cc`（試遊用の `local/playtest-main-stage13`、`d1047c2`）。ビルド用のスクリプトは `C:\Users\tnobo\keystone-playtest\scripts\native-build.ps1`（`--no-default-features`）。以前の保存のバックアップの置き場所は `C:\Users\tnobo\keystone-playtest\save-backups\`。
- コードの既存コミット: PR #99 の head `c95cf37`

## 続行範囲

- 許可済みの範囲（本人の決定）:
  - PR の順番は A → B → C → D〜F → G（Hub decision `9f97e6c2`）。Astra に渡し、独立レビューは新しいコンテキストの subagent で行う。
  - #82 は、C の PR を作ったあと、引き継ぎ先を書いたコメントを付けて閉じてよい。ブランチは消さない。
  - PR へのコメントや本文の変更は、文面を本人に見せてから投稿する。林君（`riyansu`）への依頼や指摘は「林君」と呼び、背景を伝え、提案を求める形で書く（Hub note `4359e579`）。
- 実施済み（繰り返さない）:
  - PR #99 の作成、独立レビュー、PR 本文への P2 の追記、Hub の task 2 件の登録
  - 林君が main に直接入れた keystone-lang `e07a36a` と keystone-blocks `43894cb`・`320cd8b` の事後確認（問題なし。Hub event `e8352cea`）
- 次の正確な一手:
  1. 本人が Windows で PR #99 を確かめる。手伝う場合の手順:
     - 先に `C:\Users\tnobo\AppData\Local\KeystoneCC\` を `C:\Users\tnobo\keystone-playtest\save-backups\KeystoneCC-<日付>-before-pr99\` にコピーする。
     - `C:\Users\tnobo\src\keystone_cc` で `git fetch origin feature/stage-categories` をし、`git switch --detach origin/feature/stage-categories` で切り替えてビルドする。試遊用のブランチは消さない。公開前の面は無いので、`native-build.ps1`（`--no-default-features`）でよい。
     - 確かめること: カテゴリのタブで面を選べる（マウス・キーボード・タッチ）。題名が「動かす 1」の形になる。今まで開いていた面が開いたまま。石ごとに書いたコードが残っている。保存フォルダに新しい形の保存ができ、壊れたファイルが無い。
     - 終わったら、Windows の clone を `local/playtest-main-stage13` に戻すか、本人に確認する。
  2. 確認が済んだら、本人がマージする（または `--admin` を指示する）。マージの前に `gh pr view 99 --json headRefOid` が `c95cf37...` のままかを確かめる。違えば、確認と独立レビューをやり直す。
  3. マージのあと: `/srv/sasara/worktrees/keystone_cc/feature/stage-categories` を本人の確認を得て片付ける。Hub に記録する。シートの ja タブの ID 列を新しい ID に直す（`ftl_sheet_exporter` は旧番号も読めるので急がない）。
  4. PR B の指示書を書いて Astra に渡す。中身: エージェントが石に乗って待つ・触れて動かす動き（今の Stage 8・11 で石が一度も動かなかった件）、公開中の全面を CI で確かめる仕組み（マップの読み込み、3 言語の説明文、記録した解き方を `--headless-verify` で再生）。
  5. 続けて PR C（置く）の指示書。#82 から引き継ぐもの: `assets/stages/stage-13.ron`（置く 1 の確定版）、`design/prototypes/stages-13-20-v1/` の stage-14〜16（置く 2〜4 の候補）、13 面以降の登録コード、`place up` のヘルプ、実行器のテスト、`design/solutions/` の 13〜16。公開前（`published: false`）で入れ、開発用のビルドで試遊する。C を作ったら #82 を閉じる。
- backlog（Hub）:
  - `3e326ae0`: stage_sim を新しい面の ID に対応させる（PR #99 の P2）
  - `198ccfb9`: PR #99 のレビューの軽い指摘 6 件
  - `c4124a21`: 高リフレッシュの画面で、倍率が変わった直後に移動中の石が止まる
  - `5768d5a9`: stage_sim の keystone-lang の rev を揃えるか、退役させる

## 永続先

- handoffコミットとpush: このファイルを `feature/20-stage-release-handoff` にコミットし、`origin/feature/20-stage-release-handoff` に push する。
- コミット / PR / 外部状態:
  - PR #99（open、approved、CI SUCCESS）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/99 （head `c95cf37`）
  - PR #82（draft、閉じる予定）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/82
  - 設計のまとめ（artifact）: https://claude.ai/artifact/1gPLvukv59U83YrdZCgBYg
  - 正本のシート: https://docs.google.com/spreadsheets/d/1KTxJP7uDv_RYROq7DVa8fKQfi2MWSVsdBq5DgPFy540/edit
  - 林君（`riyansu`）: 11-01 の返信と 11-02 の追加の依頼を送付済み。PR はまだ出ていない（`feature/script-system` は 10-31 の `e1d231d` のまま）。
- Sasara Hub:
  - task `d1d750ed`
  - decisions: `9f97e6c2`（公開の印と CI、PR の順番）、`0e611601`、`f97ee367`、`2873949e`、`26b65225`、`4741633b`、`55d8131e`（詳細は前の handoff）
  - events:
    - `agent-api/event/fde8b98f-cb4a-47c8-bab6-0b05f9c342de`（#99 の独立レビュー approved）
    - `agent-api/event/1854e2b9-8cfa-456d-83cd-be928a22fd95`（#99 の作成）
    - `agent-api/event/f3bcbfd2-41e1-49fd-9f87-b5286459b669`（A を Astra に渡した）
    - `agent-api/event/e8352cea-f4cf-4876-8f14-cab583c7f89a`（林君の直接 push の事後確認）
  - note `agent-api/note/4359e579-a6e8-4539-ade9-0f7a96c17e51`（林君）
