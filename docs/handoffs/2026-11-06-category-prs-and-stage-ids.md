# セッション引き継ぎ: カテゴリごとの PR への切り替えと、変わらないステージ ID（PR A）

前の handoff: `docs/handoffs/2026-11-01-stage-concept-and-signal-stone.md`。その後に設計が確定し、進め方も変わった。この先はこのファイルを正とする。

## 対象

- リポジトリ: `/home/ubuntu/repos/meowtaverse/keystone_cc`（GitHub `Meowtaverse-Games/keystone-call-of-cats`）
- worktree:
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff`（PR #82。閉じる予定。この handoff の置き場所）
  - `/srv/sasara/worktrees/keystone_cc/feature/stage-categories`（PR A の作業用。2026-11-06 に origin/main `d9a48a4` から作成。Astra に渡す）
- 確認済みGit toplevel / branch / source code HEAD:
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff` / `feature/20-stage-release-handoff` / `67483000feeba1696b76eb4f371534d59ee1e63b`（origin と一致、clean）
  - main: `d9a48a43a474b601d8702f539a63cc1f27589f23`
- worktree一覧の確認: 共有の checkout（`main` だが `4f31f85` のまま古い。触っていない）と、上の 2 つだけ。11-01 に一時領域へ置いた main の作業コピーは消えていたので、登録を `git worktree prune` で外した。
- タスク:
  - 1.0 は 5 カテゴリ 20 面（動かす 1〜6、見る 1〜2、ほる 1〜4、置く 1〜4、伝える 1〜4）。リリース後はカテゴリの末尾に面を足していく（回路の「組む」など）。
  - 各面は、お手本コードから逆算してマップを作り、headless エージェントで「コードなしでは通れず、お手本なら通れる」を確かめる。例外は動かす 1。
  - main がどの時点でも遊べるように、カテゴリごとの PR で進める。
- 記録日時: 2026-11-06T08:22+09:00

## 現在地

- 結論:
  - 設計は確定した（下の Hub の決定を参照）。正本はシート「keystone: cc: ステージ設計」のコンセプトタブ。まとめのページ: https://claude.ai/artifact/1gPLvukv59U83YrdZCgBYg
  - #82 にまとめて 20 面を入れる方式はやめ、次の順の PR に切り替えた（2026-11-06、本人の決定）。
    - A: 変わらないステージ ID、カテゴリ、公開の印、保存データの移行、カテゴリ別のステージ選択画面（Astra）
    - B: エージェントの改善（石に乗って待つ、触れて動かす）と、CI での全面の確認（Astra）
    - C: 置く。#82 の中身を引き継ぐ新しい PR。公開前で入れて試遊する（Astra）
    - D〜F: 動かす、見る、ほる の作り直し。各カテゴリを 1 つの PR でまるごと入れ替える（Astra）
    - G: 伝える。林君の Type5 の PR が入ったあと（Astra）
  - main を遊べる状態に保つ仕組み: 面ごとの公開の印（公開前の面はリリース版に出ない。開発用のビルドでは遊べる）、公開中の全面を CI で確かめる（マップの読み込み、3 言語の説明文、記録した解き方の再生）、既存の面を直すカテゴリは 1 PR でまるごと入れ替える。
  - 林君（GitHub `riyansu`）に、11-01 の返信と 11-02 の追加の依頼を本人が送った。林君の PR はまだ出ていない（`feature/script-system` は 10-31 の `e1d231d` のまま）。
- 根拠:
  - main の 1〜12 面で、たたき台のお手本コードはどれもクリアできなかった（2026-11-01、各面 30 seed）。Hub event `agent-api/event/8d51c192-d8a8-417e-b58e-5d8276dcfddf` に要点がある。
  - リリース用のビルドは `--no-default-features`（`scripts/build_macos.sh`、`.github/workflows/windows_build.yml`）。`experimental` が切れるので、公開前の面はリリース版に出ない。
  - 今の保存は通し番号に結び付いている: `src/resources/stage_progress.rs` の `unlocked_until: StageId`、`src/resources/stage_scripts.rs` の `HashMap<Language, HashMap<StageId, Vec<String>>>`。保存先はローカルと Steam Cloud（`src/resources/file_storage/`）。
- 未解決の不確実性:
  - 動かす 2（今の Stage 2）では、お手本の `move down` が 30 seed すべてで「動けない」になった。RON の石の位置 `(0.5, 0)` が関係していそう。実機では未確認。
  - 置く 2〜4 は、試遊で決める（7 マスで単調ではないか、置く 3 との違い、置く 4 の役割が読めるか）。
  - 林君が main に直接入れた keystone-lang `e07a36a` と keystone-blocks `43894cb`・`320cd8b`（整数の sleep）は、差分は読んだが、テストは流していない。
  - A1X は Windows がスリープすると止まる。長く回す作業は、止まったら再開する（本人の選択）。
- 未コミット変更・未追跡artifact:
  - コードの未コミット変更は無い。
  - ダウンロードフォルダの林君あての文面（本人が送付済み）: `C:\Users\tnobo\Downloads\keystone-外注返信-2026-11-01.txt`、`C:\Users\tnobo\Downloads\keystone-外注追加依頼-2026-11-02.txt`
- コードの既存コミット: main `d9a48a4`、#82 `b2b92a1`（handoff を除く source code の head）

## 続行範囲

- 許可済みの範囲（本人の決定）:
  - A〜G の順で、Astra に main への PR として渡す。独立レビューは新しいコンテキストの subagent。マージは本人がするか、`--admin` を明示的に指示されたときだけ Claude が行う。
  - #82 は、C の PR を作ったあと、引き継ぎ先を書いたコメントを付けて閉じてよい。ブランチは消さない。古い資料はタグ `archive/20-stage-design-history` に残っている。
  - 林君への依頼や指摘は「林君」と呼び、背景と理由を伝え、土曜日 1 回で収まる大きさにし、提案を求める形で書く（Hub note `4359e579`）。下書きは送る前に本人に見せる。
  - 正本のシートは、変更を見せたうえで Claude が書き込んでよい。
- 実施済み（繰り返さない）:
  - 設計の決定一式と Hub への記録、シートのコンセプトタブの更新、artifact の作成と更新
  - #98 と #97 用の worktree とローカルのブランチの削除、origin の URL の綴りの修正
  - 林君あての返信と追加の依頼の文面づくり（送付は本人）
- 次の正確な一手:
  1. PR A の指示書を `delegate-to-astra` の手順で書き、`/srv/sasara/worktrees/keystone_cc/feature/stage-categories` で Astra（`gpt-6-astra`、effort high。保存データの移行があり、影響範囲が広いため）に渡す。
  2. A の PR を受け入れ条件で確かめ、独立レビューにかけ、本人にマージを依頼する。
  3. 続けて B、C の指示書を作る。C を作ったら #82 を閉じる。
  4. 待ち時間に、林君が main に直接入れた keystone-lang と keystone-blocks の変更を、テストまで流して確かめる。
  5. シートの ja タブの 17〜20 行（古い構想のまま）と ID 列を、A の新しい ID に合わせて直す。
- backlog（Hub）:
  - `c4124a21`: 高リフレッシュの画面で、倍率が変わった直後に移動中の石が止まる。
  - `5768d5a9`: stage_sim の keystone-lang の rev を揃えるか、退役させる。
  - 決め打ち度の集計と表示、#98 のレビューで出た P3 の 3 件。

## 永続先

- handoffコミットとpush: このファイルを `feature/20-stage-release-handoff` にコミットし、`origin/feature/20-stage-release-handoff` に push する。#82 を閉じても、ブランチとこのファイルは残る。
- コミット / PR / 外部状態:
  - PR #82（draft、閉じる予定）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/82
  - main: `d9a48a4`
  - artifact: https://claude.ai/artifact/1gPLvukv59U83YrdZCgBYg
  - 正本のシート: https://docs.google.com/spreadsheets/d/1KTxJP7uDv_RYROq7DVa8fKQfi2MWSVsdBq5DgPFy540/edit
- Sasara Hub:
  - task `d1d750ed`
  - decisions（新しい順）:
    - `agent-api/decision/9f97e6c2-03aa-40c7-b0bf-b33318e34b8b`（公開の印と CI、PR の順番 A〜G、#82 を閉じる）
    - `agent-api/decision/0e611601-6f14-4a5c-977d-8d585cab35ce`（カテゴリごとの PR、置く 2〜4 は試遊、ID は Astra、林君への追加の依頼）
    - `agent-api/decision/f97ee367-05c5-47be-98f7-a564ff486c23`（カテゴリと番号、面を足すルール）
    - `agent-api/decision/2873949e-76da-4897-864b-86bfd701282b`（合図の原則、パスワードの仕組み）
    - `agent-api/decision/26b65225-f196-417e-805f-6640f9bcca3f`（else は全部の石、パスワードの面を採用）
    - `agent-api/decision/4741633b-1526-41de-abd1-ee4fd4edbb6c`（Type5 の命令、ブロックでの Rhai 対応を続ける）
    - `agent-api/decision/55d8131e-eb34-44bd-87de-9504c000b144`（お手本コードから逆算、山場で合図）
  - note `agent-api/note/4359e579-a6e8-4539-ade9-0f7a96c17e51`（林君）
  - events: `agent-api/event/573d4df5-1626-4a48-b491-4d899b60f31c`（林君へ送付）、`agent-api/event/8d51c192-d8a8-417e-b58e-5d8276dcfddf`（1〜12 面の検証）
