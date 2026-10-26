# Stage 6: 高さを確保して反復移動

2026-09-29。main `252efbea`、ツール `bcb599b`。右2回で入口の床下にある石を搭乗位置へ出し、プレイヤーが搭乗。上3回で高さを確保し、右18回で渡る。低い位置の候補はチャンク差で38/100だったが、高い共通経路は100/100。準備操作・搭乗時機と、製品でのloop停止・下車は実機確認が必要。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-6](baseline-6/summary.json) | 0 / 100 | 5 | unmodified main |
| [crossing-6](crossing-6/summary.json) | 38 / 100 | 5 | unmodified main |
| [high-route-6](high-route-6/summary.json) | 100 / 100 | 5 | unmodified main |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[high-route-6/seed-0-replay.plan](high-route-6/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 6 --stage-file design/reviews/stage-06/main-stage-6.ron --seed 0 --coordinates
sim simulate 6 --stage-file design/reviews/stage-06/main-stage-6.ron \
  --plan design/reviews/stage-06/high-route-6/seed-0-replay.plan --seed 0 --require-goal
```

共通の搭乗・石操作とseedごとの出口探索を再実行：

```bash
sim check 6 --stage-file design/reviews/stage-06/main-stage-6.ron \
  --plan design/reviews/stage-06/high-route-6/input.plan --seed 0 --seeds 100 \
  --walk-to-goal --reject-blocked --output /tmp/stage06-review-repeat
```
