# セッション引き継ぎ: 20面のコンセプトの作り直しと合図の石（Type5）

前の handoff: `docs/handoffs/2026-10-26-headless-verification-and-agent.md`。その「次の一手」の 1〜3 は終わっていて、20面の設計の方針も変わった。この先はこのファイルを正とする。

## 対象

- リポジトリ: `/home/ubuntu/repos/meowtaverse/keystone_cc`（GitHub `Meowtaverse-Games/keystone-call-of-cats`）
- worktree: `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff`（PR #82。この handoff の置き場所）
- 確認済みGit toplevel / branch / source code HEAD: `/srv/sasara/worktrees/keystone_cc/feature/20-stage-release-handoff` / `feature/20-stage-release-handoff` / `b2b92a11a65f54b48a442413e46ed03792bc384b`（origin と一致、clean。無視対象の `target/` だけ）
- worktree一覧の確認:
  - 共有の checkout `/home/ubuntu/repos/meowtaverse/keystone_cc`：`main` だが `4f31f85` のまま古い。触っていない。
  - `/srv/sasara/worktrees/keystone_cc/feature/headless-agent`（`8ce0930`、#97 はマージ済み、clean）
  - `/srv/sasara/worktrees/keystone_cc/fix/agent-clearance`（`ecfd75a`、#98 はマージ済み、clean）
  - 上の 2 つは役目を終えた。削除してよいか、本人の返事を待っている。
- タスク:
  - 20面（第1部）と HTTP の拡張（第2部）から成る 1.0 を仕上げる。
  - 各面は、お手本コードから逆算してマップを作り、headless エージェントで「コードなしでは通れず、お手本コードなら通れる」ことを確かめる。例外として、Stage 1 はコードなしで進む面にする。
  - #82 には、ステージのコンセプト、実際のマップ、想定解だけを残す。
- 記録日時: 2026-11-01T10:03+09:00

## 現在地

- 結論:
  - main は `d9a48a4`（#98）。#97（自律エージェント）と #98（ふさがった跳び込みを経路にしない修正）はマージ済み。#98 は本人の指示で `--admin` の squash マージをした。
  - #82 は main `d9a48a4` まで取り込み済み（`b2b92a1`）。2回の取り込み（`e7bbb41`、`b2b92a1`）は、どちらも独立レビューで approved だった。
  - **#82 のマップは最新の設計と合っていない。** 1〜12 は main と違う 09-04 の古い固定配置の候補で、14〜20 も古い候補のまま。最新の設計と合っているのは 13 だけ。
  - **設計の方針が変わった**（2026-10-29〜30、本人の決定）:
    - 正本は元のスプレッドシート「keystone: cc: ステージ設計」https://docs.google.com/spreadsheets/d/1KTxJP7uDv_RYROq7DVa8fKQfi2MWSVsdBq5DgPFy540/edit 。ja タブに I〜N 列（石の数、上限、想定コード、状態、マップ、元の構想・備考）を足し、先頭に「コンセプト」タブを作った。
    - コンセプト:
      - 対象は、プログラミングに興味があり、Scratch に触ったことがあって、ゲーム作りに興味がある小学生。
      - ComputerCraft の発想を土台にする。石がタートル、石どうしの合図が rednet に当たる。
      - 二部構成。第1部はお手本コードに対応した 20 面、第2部は HTTP で外の世界の自分のプログラムから石を動かす。
    - 終盤と山場では、石どうしの合図（`send` / `receive`）を必須にする。たとえば、石を踏むとその石が別の石に合図を送り、合図を受けた石が動かないとクリアできない仕掛けにする。これは V2 の計画にあった「通信を必須にしない」を覆す決定。
    - 合図の石として Type5 を新しく作る（本人の選択 a）。
    - 17 面に入れやすい要素が見つかれば、「`sleep` での時間合わせの難しさを味わう面」は作らない。
  - コンセプトタブの「決めること」1〜4 は、2026-11-01 の時点で未決のまま（下の「次の正確な一手」）。
- 根拠:
  - #82 の 20 面をコードなしでエージェントに回した結果（`e7bbb41` と `b2b92a1` の release ビルド、各面 1,000 seed）:
    - Stage 1・10・12・20 はコードなしでクリアでき、ほかの 16 面はクリアできなかった。
    - 地形が seed で変わるのは Stage 13（328 種類）だけで、19 面は固定の地形だった。
    - ただし、これは古いマップでの結果なので、マップを作り直したら回し直す。生データのあった一時領域は、もう残っていない。
  - #82 の Stage 2 は、`move("down");` を渡すと 20 seed すべてでエージェントの 1 回目の試行でクリアでき、コードなしではクリアできない（#98 の効果。マージ前に確かめた）。
  - コードの調査（main `d9a48a4`。keystone-lang は `07c12ed`）:
    - Type1〜4 の命令の制限は #83 で実装済み。Keystone では実行前に全体を検査し、Rhai では呼んだときにエラーにする。headless と HTTP も同じ制限を使う。
      - Type1: `move`、`sleep`
      - Type2: `move`、`is_touched`、`is_empty`
      - Type3: `move`、`sleep`、`is_touched`、`is_empty`、`dig`
      - Type4: `move`、`is_touched`、`is_empty`、`place`
    - Keystone の実行器には、石の間で名前付きの合図を共有する仕組みがあり、`send "open"` / `receive "open"` が動く。`receive` は合図が届くまで待ち、受け取ると合図は消える。
    - ただし、合図、`print`、`turn`、`rand` は制限の対象外で、どの石でも書ける。製品は `print`、`turn`、`rand` を無視する。
    - Rhai には `send` / `receive` が無い（`sleep` はある）。Rhai にしか無いのは、関数の定義、配列、`for`、`break`、`else`。
    - main に Type5 はまだ無い。
- 未解決の不確実性:
  - コードなしで「クリアできない」は、エージェントが経路を見つけられなかったという意味で、通れないことを証明したものではない。
  - 1〜16 面のお手本コード（コンセプトタブ）はたたき台で、製品で確かめたものは無い。
  - #82 の `design/solutions/`（`.ks` と `.plan`）は古いマップに合わせたもの。Stage 13〜16 と 20 には石のコードが無い。
  - 説明文は、3 言語とも 11〜12 か所が「・・・」や「...」のまま。日本語は Google Sheet から `./scripts/export_ftl.sh` で書き出す。13〜16 面は文を新しく作り、17〜20 面は書き直す（ja タブの N 列に印がある）。
  - 最終面をクリアした後に F3 ですぐゴールになる件は、まだ確かめていない。20 面に増えると最終面が変わるので、リリース前に headless で確かめる。
  - V2 のシート `1iXRJlJJ7kFDQ9uhlhcrkMdMKGxVCR6lGhB9WsJndCmw` は、つないでいるアカウントでは読めない。参考扱い。
  - A1X は Windows の WSL2 で、Windows がスリープすると止まる。長く回す作業は、対策なしで始め、止まったら再開する（本人の選択 b）。
- 未コミット変更・未追跡artifact:
  - コードの未コミット変更は無い。
  - `ext-assets/` はサブモジュールで、`fonts` と `images` はビルド用に置いたもの。コミットしない。
  - Windows の試遊環境は `local/playtest-main-stage13`（`d1047c2`）。2026-10-30 に、本人の依頼でデスクトップのショートカットから起動した。
- コードの既存コミット: `b2b92a1`（#82）、`d9a48a4`（main）

## 続行範囲

- 許可済みの範囲（本人の決定）:
  - 機能、基盤、製品の修正は、main への個別の PR で入れる。#82 は main を取り込むだけにし、ステージ設計に専念する。#82 は 20 面がすべて検証を通ったら、そのまま main に入れる（別の PR は作らない）。
  - 実装は Astra（`codex exec`）に渡し、独立レビューは新しいコンテキストの subagent で行う。マージは本人がするか、`--admin` を明示的に指示されたときだけ Claude が行う。
  - 正本のシートは、本人の承認があれば Claude が書き込んでよい（10/29 に I〜N 列、10/30 にコンセプトタブを書き込んだ）。書き込む前に変更一覧を見せる。
  - `design/` は「ステージのコンセプト、実際のマップ、想定解」だけを残す方針（10/27 に承認）。ただし、マップを揃えるまで削除は止めている。
- 実施済み（繰り返さない）:
  - #97 と #98 のマージ、#82 への main の取り込み（2回）と、それぞれの独立レビュー
  - 20 面をコードなしで回すこと（修正の前と後の 2 回）
  - 省く前の状態を、タグ `archive/20-stage-design-history`（`b2b92a1`）として push した
  - 正本のシートの照合、I〜N 列の追加、コンセプトタブの作成
- 次の正確な一手:
  1. コンセプトタブの「決めること」を本人に決めてもらう。
     - (1) 17 面に入れる要素。案は「変数で数える」（`n = n + 1` で進んだ数を数え、同じ数だけ上がる）。
     - (2) Type5 で使える命令。合図だけにするか、掘る・置くも使えるようにするか。
     - (3) クライマックスで、1 面の石は同じ種類という制限を外すか。
     - あわせて、Keystone の `rand` / `turn` を第1部で使えないようにするか、石の吹き出し（`print`）を入れるかも決める。
  2. 決まったら、コンセプトタブと ja タブの 17〜20 行を更新する（本人の確認後に書き込む）。
  3. main への PR を Astra に渡す: Type5 の追加と合図の命令。
     - `StoneType` に Type5 を足し、RON で指定できるようにする。
     - Rhai に `send(...)` / `receive(...)` を足し、`receive` は Keystone と同じく届くまで待つ。
     - Keystone と Rhai の両方で、合図を Type5 だけに制限する（keystone-lang の修正を含む）。
     - ヘルプ、テスト、headless の対応も含める。石の画像は別の作業。
  4. #82 で、1〜20 面のお手本コードを確定し、それが必要で十分になるマップを作る。1〜12 は main のマップを出発点にする。そのあと、エージェントでコードなし・お手本ありの両方を回し、`design/solutions/` を書き直してから、`design/` を整理する。
  5. 本人の返事を待っているもの:
     - `feature/headless-agent` と `fix/agent-clearance` の worktree を削除してよいか
     - origin の URL の綴り（`Meotaverse-Games`、w が抜けている）を `git remote set-url` で直すか
- backlog（main 側。リリースには必須でない）:
  - 決め打ち度の集計と表示。unsolved を「計画のモデル上で経路がない」と「物理で失敗した」に分ける。古い `solutions/seed-N.json` を残さない。地形の実際の種類数を目立たせる。
  - #98 のレビューで出た P3 の 3 件（Hub に記録済み）: 天井の低い 1 段上りを候補から外しすぎる。低い位置へ跳ぶとき、途中の列で出発の高さより下を見ていない。調べなくてよいマスのテストが少ない。
  - Hub `c4124a21`: 高リフレッシュの画面で、倍率が変わった直後に移動中の石が止まる。
  - Hub `5768d5a9`: stage_sim の keystone-lang の rev を揃えるか、退役させる。

## 永続先

- handoffコミットとpush: このファイルを `feature/20-stage-release-handoff` にコミットし、`origin/feature/20-stage-release-handoff` に push する（結果はコミット後に確認する）。
- コミット / PR / 外部状態:
  - PR #82（draft）: https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/82 （source code head `b2b92a1`）
  - PR #97（merged、`89536ed`）、PR #98（merged、`d9a48a4`）
  - タグ `archive/20-stage-design-history` → `b2b92a1`
  - 正本のシート: https://docs.google.com/spreadsheets/d/1KTxJP7uDv_RYROq7DVa8fKQfi2MWSVsdBq5DgPFy540/edit （タブはコンセプト、ja）
- Sasara Hub:
  - task `d1d750ed`
  - decisions:
    - `agent-api/decision/55d8131e-eb34-44bd-87de-9504c000b144`（お手本コードから逆算し、山場で合図を必須にする）
    - `agent-api/decision/f562556c-02ee-4865-be58-d701b988d057`（機能は main への PR、#82 はステージ設計）
    - `agent-api/decision/f28b49ca-3d62-490e-9d0e-bf0a288604d2`（headless、60Hz、1,000 地形を 5 分以内）
    - `agent-api/decision/925cf886-731c-4208-b3bb-9de8a3ebacba`（計画 → 物理で確認 → 計画し直し）
  - events（新しい順）:
    - `agent-api/event/c27a1726-f0ab-4b24-8c84-e3ad64a312e9`（コンセプトタブ、keystone-lang の調査）
    - `agent-api/event/f26c7473-c7a9-4234-8264-d8e0e72290c3`（元のシートを正本にした、#82 のマップが古い）
    - `agent-api/event/eb4f1fe6-a1cf-4192-9239-2b212cb33a54`（#98 の後で 20 面を回し直した）
    - `agent-api/event/3e34864c-ae83-482a-83a3-c6f1825aa946`（#98 のマージと #82 への取り込み）
    - `agent-api/event/97f6611d-6d91-41a4-80ee-be3662148c82`（#98 の独立レビュー）
    - `agent-api/event/2592b310-12f9-46f7-b980-b26fdee51e73`（20 面をコードなしで回した）
    - `agent-api/event/8b669881-0e86-4163-9247-e80ac4ec52dd`（#82 への取り込み `e7bbb41` のレビュー）
