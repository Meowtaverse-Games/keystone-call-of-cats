# Stage 12: 5回の縦掘削と採掘なしの別解

2026-09-29。main `252efbea`、ツール `bcb599b`。最初に石を左1回動かすとCLIで搭乗できる。右1回で元の列へ戻し、空洞では移動だけ、岩ではdig→moveを使い5回採掘する。その後石を右へ1回、下へ1回動かして出口まで到達する。一方、元の列の右を下降すれば採掘0回でも到達した。CLI上の別解として記録し、実機で成立するか確認してから変更を判断する。

製品地形を変更していない。これは離散グリッドの確認であり実機合格ではない。Stage 8・11のO除去は別データでの仮定であり、mainの成功結果に数えない。

## 結果

| 試験 | 成功 / 件数 | 地形数 | ソース |
| --- | ---: | ---: | --- |
| [baseline-12](baseline-12/summary.json) | 0 / 100 | 1 | unmodified main |
| [crossing-12](crossing-12/summary.json) | 100 / 100 | 1 | unmodified main |
| [dig-12](dig-12/summary.json) | 0 / 100 | 1 | unmodified main |
| [intended-12](intended-12/summary.json) | 0 / 1 | 1 | unmodified main |
| [intended-complete-12](intended-complete-12/summary.json) | 1 / 1 | 1 | unmodified main |

全観測地形の代表seedは[ASCII一覧](ascii-layouts.txt)。主要な再生手順は[intended-complete-12/seed-0-replay.plan](intended-complete-12/seed-0-replay.plan)。失敗した試験は到達不能の証明ではなく、その入力手順の結果である。

## 再現

リポジトリルートから実行する。新しい出力先を指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 12 --stage-file design/reviews/stage-12/main-stage-12.ron --seed 0 --coordinates
sim simulate 12 --stage-file design/reviews/stage-12/main-stage-12.ron \
  --plan design/reviews/stage-12/intended-complete-12/seed-0-replay.plan --seed 0 --require-goal
```

採掘なしの再生は`crossing-12/seed-0-replay.plan`。原本RONと`simulate --require-goal`で再現できる。5回掘った直後に止めた`intended-12`ではまだ出口へ届かず、石の右・下移動を追加して`intended-complete-12`で到達した。
