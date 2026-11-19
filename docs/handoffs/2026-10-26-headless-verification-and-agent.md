# セッション引き継ぎ: 窓なしのステージ検証基盤と自律エージェント（20面の実装ブランチとの分担）

## 対象

- リポジトリ: `/home/ubuntu/repos/meowtaverse/keystone_cc`（GitHub `Meowtaverse-Games/keystone-call-of-cats`）
- worktree:
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff`（この handoff の置き場所。PR #82）
  - `/srv/sasara/worktrees/keystone_cc/feature/headless-agent`（PR #97。マージ待ち）
- 確認済みGit toplevel / branch / source code HEAD:
  - `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff` / `feature/20-stage-release-handoff` / `bda36f6546feb59d2cd4bf4c91009f86055d4e64`（origin と一致）
  - `/srv/sasara/worktrees/keystone_cc/feature/headless-agent` / `feature/headless-agent` / `8ce0930e7522ec8f1a0d16d38614c6946a8480a5`（origin と一致、clean）
- worktree一覧の確認: 上の2つと、共有の checkout `/home/ubuntu/repos/meowtaverse/keystone_cc`（`main` だが `4f31f85` のまま古い。触っていない）。ほかの作業用 worktree は、マージ後にすべて削除済み。
- タスク:
  - 製品と同じ処理を窓なしで早送りし、ランダムな地形（chunk）のステージを CLI と AI で検証できる基盤を作る。地形を固定せずに済ませ、ゲームの奥行きを保つ。
  - その上に、地形を見てゴールへ向かう自律エージェントを作る。条件①コードなし、②想定のコード。
  - 20面の実装ブランチ #82 は、ステージ設計だけに専念させる。
- 記録日時: 2026-10-26T07:50+09:00

## 現在地

- 結論:
  - 基盤は main に入った（PR #89〜#96）。自律エージェント v1（PR #97）は独立レビューで approved、マージ待ち。
  - #82 は main `fd3f8cd` まで取り込み済み（`bda36f6`）。#96 と #97 はまだ取り込んでいない。
- main に入った PR（新しい順）:
  - #96 `cc1b193`: 1フレームずつ進める `headless::checkpoint::Session` と、保存・復元の正確さの基準。復元は再生方式（O(k)）で、速くはない。
  - #95 `fd3f8cd`: 基準のテストを製品のステージから切り離し、`examples/headless/fixtures/` の固定の地形を使うようにした。
  - #94 `6d24d0f`: プレイヤーの動きのパラメータ化（`assets/config/player_physics.ron`、`PlayerPhysicsParams`）。headless の `stage_id` をカタログから決める。登録の一致テストを強化。
  - #93 `14d8617`: 窓の倍率が変わった瞬間に、速さ・石の移動距離・降下中の重力を換算する。石の衝突判定は古い倍率で行う。
  - #92 `469109a`: Keystone のヘルプの `sleep 1.0`。左の pane が担当した。
  - #91 `121e932`: `--headless-verify`、`--headless-measure-reach`、`--replay`、レポートの形式 v2、地形の seed。
  - #89 `f1562d3`: `list.ron` と埋め込みの整合テスト。12 クリア後の panic を直した。
- 根拠（主な検証値）:
  - 基準のハッシュ: Stage 1 `fnv1a64-v1:8f5d6c37b6efc4fe`、Stage 2 `fnv1a64-v1:eb1f870862eed1bc`。ほか `examples/headless/motion-baselines.json` の6件。
  - Windows のネイティブ release ビルドで、headless とゲーム画面のリプレイの両方が、Linux の headless と軌跡のハッシュで一致した（#91、4回確認）。
  - 到達範囲: 既定の値では、跳べる段は1マス、越えられる穴は2マス。窓の倍率（0.5〜2.0）と map_size によらない。
  - #97: 固定の Stage 1 を 1,000 seed で 85.7 秒、全件 cleared（ただし実際の地形は28種類）。書き出した時刻表は `--headless-verify` で全件ハッシュが一致した。
- 未解決の不確実性:
  - エージェントを、製品のステージや #82 の 20面で回した結果はまだ無い。
  - 条件②（想定のコード）で使う Stage 1〜12 の想定解は、確かめ済みのものが1つも無い。左の pane（keystone-cc-15）が一覧を作り、Hub にも記録すると報告した（ID は未確認）。一時ファイルは `/tmp/claude-1000/-home-ubuntu-repos-meowtaverse-keystone-cc/0cc9bf05-b9c8-41b3-9598-0cc4fac06dfd/scratchpad/stage-1-12-intended-solutions.md`。#82 の地形は main の 1〜12 から変わっているので、#82 の面に合わせて見直す必要がある。
  - Stage 13 の「足場なしでクリアした」は誤認だった。毎回 `place up` を使っていた。ただし、抜け道が無いことはまだ確かめていない。
  - この A1X は Windows 上の WSL2 で、Windows がスリープすると止まる（2026-10-25 22:41〜翌 07:0x に約8時間半停止）。長い作業の前に、スリープ対策を本人と決める必要がある。
- 未コミット変更・未追跡artifact:
  - コードの未コミット変更は無い。
  - #82 の worktree の `ext-assets/{fonts,images}` は、ビルド用に元の checkout からコピーしたもの。コミットしない。
  - リプレイビューア（Artifact）: https://claude.ai/artifact/7zZnQ6pPEp9wfrRf9e29RM（レポートの形式 v2 に対応。ソースは一時的な scratchpad）
  - Windows での確認用スクリプト: `C:\Users\tnobo\keystone-playtest\scripts\replay-check-{build,run,restore}.ps1`。結果は `C:\Users\tnobo\keystone-playtest\replay-check\`。Windows の clone は試遊用の `local/playtest-main-stage13`（`d1047c2`）に戻してある。
- コードの既存コミット: 上の各 PR の merge commit。#97 は `8ce0930`。

## 続行範囲

- 許可済みの範囲（本人の決定）:
  - 機能・基盤・製品の修正は、main への個別の PR で入れる。#82 は main を取り込むだけにし、ステージ設計に専念する。#82 は、20面がすべて検証を通ったらまとめて main に入れる（方式 a）。
  - 実装は Astra（`codex exec`）、独立レビューは新しいコンテキストの subagent で行う。マージは本人（main のルールで承認レビューが1件必要。本人がマージするか、`--admin` を明示的に指示されたときだけ Claude が行う）。
  - 検証の正しさの基準は headless（60Hz 固定）。1面あたり 1,000 地形を 5 分以内が目標。決め打ち度は合否に使わず、可視化して後で改善する。
  - 速い巻き戻しは作らない。エージェントは「計画 → 物理で確認 → 計画し直し」。
- 実施済み（繰り返さない）:
  - #87 は #82 に統合し、コメントを付けて閉じた。
  - #82 の題名を `Implement the 20-stage release` にし、本文を書き直した。
  - 空中で倍率を変える不具合の修正（#93）、Windows の実画面での確認（#91）。
- 次の正確な一手:
  1. `gh pr view 97 --json state,headRefOid` で、#97 がマージされたかと、head が `8ce0930` のままかを確かめる。未マージなら、本人にマージを依頼する。
  2. マージ後、#82 の worktree で `git fetch origin main` → `git merge --no-ff origin/main`。競合を解消し、`cargo fmt --all -- --check` / `cargo clippy -j 8 -- -D warnings` / `cargo test -j 8` を通す。push して独立レビューにかける。
  3. #82 の各面を、エージェントの条件①（コードなし）で回す。抜け道（`cleared` になった面）を洗い出し、本人に報告する。
  4. 次の main への PR: 決め打ち度の集計と表示。#97 のレビューの P3 も一緒に扱う。
     - unsolved を「計画のモデル上で経路がない」と「物理で失敗した」に分ける。
     - 再実行したとき、古い `solutions/seed-N.json` を残さない。
     - 地形の実際の種類数をレポートに目立たせる。
  5. 条件②の想定解を、#82 の面に合わせて整える。これはステージ設計の資料なので、#82 側に置く。
- backlog（Hub）:
  - `c4124a21`: 高リフレッシュの画面で、倍率が変わった直後に移動中の石が誤って止まる（既存の問題）。
  - `5768d5a9`: stage_sim の keystone-lang の rev を揃える。headless の結果を見て、退役させるかを決める。
  - 最終面のクリア後に F3 で即ゴールになる件は、headless で確かめてから積む（未確認）。

## 永続先

- handoffコミットとpush: このファイルを `feature/20-stage-release-handoff` にコミットし、`origin/feature/20-stage-release-handoff` に push する（結果は下のコミットで確認する）。
- コミット / PR / 外部状態:
  - PR #82（draft、20面の実装ブランチ）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/82（head `bda36f6`）
  - PR #97（open、approved、CI 成功）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/97（head `8ce0930`）
  - main の最新: `cc1b193`
- Sasara Hub:
  - task `d1d750ed`（基盤づくり、In-Progress）
  - decisions:
    - `agent-api/decision/f28b49ca-3d62-490e-9d0e-bf0a288604d2`（headless・60Hz・1,000 地形を5分）
    - `agent-api/decision/f562556c-02ee-4865-be58-d701b988d057`（機能は main への PR、#82 はステージ設計）
    - `agent-api/decision/925cf886-731c-4208-b3bb-9de8a3ebacba`（計画 → 物理で確認 → 計画し直し）
  - 最新の event: `agent-api/event/e27ebf35-6fd0-43de-9023-2e94c4c9fa2e`（#97 approved）
