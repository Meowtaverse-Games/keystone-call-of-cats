# Stage 7: 接触で動かす上昇足場

2026-09-29。main `252efbea`、ツール `bcb599b`。共通の搭乗手順と上移動8回で32地形・100 seedを通過。CLIでは搭乗後に手動で動かしたため、is_touchedの条件式やRhaiの連続実行を検証した結果ではない。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-7](baseline-7/summary.json) | 0 / 100 | 32 | unmodified main |
| [crossing-7](crossing-7/summary.json) | 100 / 100 | 32 | unmodified main |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[crossing-7/seed-0-replay.plan](crossing-7/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 7 --stage-file design/reviews/stage-07/main-stage-7.ron --seed 0 --coordinates
sim simulate 7 --stage-file design/reviews/stage-07/main-stage-7.ron \
  --plan design/reviews/stage-07/crossing-7/seed-0-replay.plan --seed 0 --require-goal
```

共通の搭乗・石操作とseedごとの出口探索を再実行：

```bash
sim check 7 --stage-file design/reviews/stage-07/main-stage-7.ron \
  --plan design/reviews/stage-07/crossing-7/input.plan --seed 0 --seeds 100 \
  --walk-to-goal --reject-blocked --output /tmp/stage07-review-repeat
```
