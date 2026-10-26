# Stage 8: 時間で開く道と接触待ち

2026-09-29。main `252efbea`、ツール `bcb599b`。mainのOがCLIでは消えず、入口側の到達範囲が閉じている。別RONでOだけを除いた感度確認では搭乗・上昇・右への運搬・出口の一連の操作が成立。固定地形を維持し、実機で時間消滅と接触制御を確認する。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-8](baseline-8/summary.json) | 0 / 100 | 1 | unmodified main |
| [crossing-8](crossing-8/summary.json) | 100 / 100 | 1 | expired-obstacles sensitivity only |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[crossing-8/seed-0-replay.plan](crossing-8/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 8 --stage-file design/reviews/stage-08/main-stage-8.ron --seed 0 --coordinates
sim simulate 8 --stage-file design/reviews/stage-08/expired-obstacles-only.ron \
  --plan design/reviews/stage-08/crossing-8/seed-0-replay.plan --seed 0 --require-goal
```

共通の搭乗・石操作とseedごとの出口探索を再実行：

```bash
sim check 8 --stage-file design/reviews/stage-08/expired-obstacles-only.ron \
  --plan design/reviews/stage-08/crossing-8/input.plan --seed 0 --seeds 100 \
  --walk-to-goal --reject-blocked --output /tmp/stage08-review-repeat
```
