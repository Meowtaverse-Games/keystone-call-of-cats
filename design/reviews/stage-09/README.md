# Stage 9: 採掘して石を取り出す

2026-09-29。main `252efbea`、ツール `bcb599b`。石の左を掘って左へ動く操作を2回、さらに左へ1回動かして入口側へ出す。プレイヤー搭乗後に上へ1回動かす共通手順で2地形・100 seedを通過。採掘は2回、残数3。これは資料の下向き採掘とは別の解法候補。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-9](baseline-9/summary.json) | 0 / 100 | 2 | unmodified main |
| [crossing-9](crossing-9/summary.json) | 100 / 100 | 2 | unmodified main |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[crossing-9/seed-0-replay.plan](crossing-9/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 9 --stage-file design/reviews/stage-09/main-stage-9.ron --seed 0 --coordinates
sim simulate 9 --stage-file design/reviews/stage-09/main-stage-9.ron \
  --plan design/reviews/stage-09/crossing-9/seed-0-replay.plan --seed 0 --require-goal
```

共通の搭乗・石操作とseedごとの出口探索を再実行：

```bash
sim check 9 --stage-file design/reviews/stage-09/main-stage-9.ron \
  --plan design/reviews/stage-09/crossing-9/input.plan --seed 0 --seeds 100 \
  --walk-to-goal --reject-blocked --output /tmp/stage09-review-repeat
```
