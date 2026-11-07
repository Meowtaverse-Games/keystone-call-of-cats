# Stage 11: 2石を維持して実際の役割を見る

2026-09-29。main `252efbea`、ツール `bcb599b`。mainは入口Oで止まる。O消滅後の別RONでは石0のみで到達し、石1は静止したまま。石0の下を3回掘る経路も、右へ迂回して採掘しない経路も成立。石1だけを運搬する補助探索では経路は見つからなかったが、無価値・不要との判断はしない。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-11](baseline-11/summary.json) | 0 / 100 | 1 | unmodified main |
| [crossing-11](crossing-11/summary.json) | 100 / 100 | 1 | expired-obstacles sensitivity only |
| [dig-11](dig-11/summary.json) | 0 / 100 | 1 | unmodified main |
| [intended-11](intended-11/summary.json) | 0 / 1 | 1 | expired-obstacles sensitivity only |
| [intended-complete-11](intended-complete-11/summary.json) | 1 / 1 | 1 | expired-obstacles sensitivity only |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[intended-complete-11/seed-0-replay.plan](intended-complete-11/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 11 --stage-file design/reviews/stage-11/main-stage-11.ron --seed 0 --coordinates
sim simulate 11 --stage-file design/reviews/stage-11/expired-obstacles-only.ron \
  --plan design/reviews/stage-11/intended-complete-11/seed-0-replay.plan --seed 0 --require-goal
```

採掘なしの経路は`crossing-11/seed-0-replay.plan`。必ず別データ`expired-obstacles-only.ron`を使う。main原本では入口Oの時間消滅が未モデル化であり、同じ判定はできない。
