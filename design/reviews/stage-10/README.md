# Stage 10: 採掘と移動順、出口のチャンク差

2026-09-29。main `252efbea`、ツール `bcb599b`。石の左を1回掘って左へ動かし、上を1回掘る。下降するプレイヤーが搭乗し、石を上へ動かす手順で83/100に到達。失敗17 seedは同じ1地形。seed 1で同じ採掘後に石移動だけを追加する補助探索でも完全経路は得られなかったが、別解不存在は証明していない。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 追加検証: 地形変更なしで100/100到達

前回の83/100は、最初の搭乗後に石の操作を止めた手順の不足だった。`complete-crossing/input.plan`で13種類・seed 0〜99の全件に実到達し、無効操作0、採掘5回（残数0）を確認した。前回の失敗記録は比較のため保持する。不可能配置と判断したり、チャンクを削ったりしていない。

追加した流れ:

1. 左・上の2回採掘で最初に搭乗し、石を上へ1回動かす。
2. プレイヤーはいったん右の足場へ降りる。石だけを下へ1回、右へ9回、上へ2回送り、通路の先へ回す。
3. 残り3回の採掘で右側の岩を開け、石をプレイヤーが乗り直せる位置(14,4)へ戻す。
4. 再搭乗して右へ7回運び、(21,4)から下車して出口へ進む。

主要課題である採掘・移動順・プレイヤーの乗降が、この地形のままで成立する。5回が全解法における最小採掘数という証明ではない。二度の搭乗に合わせた待機や接触を含む製品コード、実際のジャンプ・運搬は未検証。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim check 10 --stage-file design/reviews/stage-10/main-stage-10.ron \
  --plan design/reviews/stage-10/complete-crossing/input.plan \
  --seeds 100 --walk-to-goal --require-initially-unreachable --reject-blocked \
  --output /tmp/stage10-complete-repeat
sim simulate 10 --stage-file design/reviews/stage-10/main-stage-10.ron \
  --plan design/reviews/stage-10/complete-crossing/seed-1-replay.plan \
  --seed 1 --require-goal
```

結果は[complete-crossing/summary.json](complete-crossing/summary.json)。seed 0・1の完全操作列は保存先から再実行して到達確認済み。

## 前回の結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-10](baseline-10/summary.json) | 0 / 100 | 13 | unmodified main |
| [crossing-10](crossing-10/summary.json) | 83 / 100 | 13 | unmodified main |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[crossing-10/seed-0-replay.plan](crossing-10/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 10 --stage-file design/reviews/stage-10/main-stage-10.ron --seed 0 --coordinates
sim simulate 10 --stage-file design/reviews/stage-10/main-stage-10.ron \
  --plan design/reviews/stage-10/crossing-10/seed-0-replay.plan --seed 0 --require-goal
```

共通の搭乗・石操作とseedごとの出口探索を再実行：

```bash
sim check 10 --stage-file design/reviews/stage-10/main-stage-10.ron \
  --plan design/reviews/stage-10/crossing-10/input.plan --seed 0 --seeds 100 \
  --walk-to-goal --reject-blocked --output /tmp/stage10-review-repeat
```

上記一括確認は17件失敗し非0終了するのが今回の記録。`crossing-10/seed-1.json`、`.txt`、`.plan`に失敗を保存。元のRONと`input.plan`を同じコマンドへ渡し、`--seed 1 --seeds 1`で再現する。地形やassertを弱めて通していない。
