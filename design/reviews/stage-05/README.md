# Stage 5: main継承候補のASCII検証

2026-09-29。地形の更新は行わず、mainを維持する提案。これはCLI上の検証結果であり、実機のクリア承認ではない。

## 対象と判断

- main `252efbea156f674b6be819656a676592fff55eaf` の `assets/stages/stage-5.ron` を `main-stage-5.ron` に無変更で保存。統合ブランチの製品assetsとは別物。
- ツール: `bcb599b` の独立stage_sim。seed 0〜99。
- Type1×1。開始地形と石のある必須チャンクは共通で、主な変化は石より先の短い通路と出口。100 seedで実地形6種類を観測した。全組合せの網羅を意味しない。
- 主要体験は「離れた石まで行く間に待ってもらい、搭乗して渡る」。ランダム性を追加するより、現在ある変化を残す方針を推奨する。

## 結果

| 確認 | 結果 | 意味 |
| --- | --- | --- |
| 石を操作せずwalk-to-goal | 0/100到達 | 静止した石でのプレイヤー単独経路はCLIで見つからない |
| 共通搭乗手順→石を右4回→walk-to-goal | 100/100到達、無効操作0 | 同じ搭乗・石操作で6種類の地形を通過できた |
| 石を右4回先に動かす→同じ搭乗手順→walk-to-goal | 0/100到達 | この早発手順では乗り遅れる。別解不存在の証明ではない |
| seed 0の完全操作列をsimulate --require-goal | 成功 | 探索結果を実際のコマンドで再生できた |

共通手順は `boarding-and-crossing.plan`。入口(1,14)から `p leap-right` 7回で石の上(15,8)へ到着し、石(15,7)を右へ4回動かして(19,7)へ運ぶ。その後の出口経路はseedごとに探索・実行した。共通の完全プレイヤー経路や最短解を保証した結果ではない。

代表seed 0では石から右へ降り、その先を跳び越えてゴールへ到着する。`seed-0-replay.plan`は到着assertを含む完全な再生手順。全6地形の代表seedは0、1、2、6、7、23で、`ascii-layouts.txt`に保存した。判定と各seedの最終状態は`summary.json`に保存した。

## 実機で残す確認

1. CLIの搭乗経路は上側の岩を越えて石へ降りる。想定した下降ルートと異なるため、実機のジャンプ幅・落下・搭乗で成立するかを確認する。地形をCLIへ合わせて変えない。
2. 石のコード案は「sleepで待機→右へmoveを4回」。4回はCLIで成立した候補であり、製品の確定値ではない。sleepの秒数は実機で入口からの所要時間と余裕を測って決める。CLIの逐次手動操作は実時間の待機・同時実行を検証していない。
3. 搭乗前に出発すると何が起こるか、失敗後のリセット、乗車中の安定性、下車しやすさを確認する。
4. CLI seedは製品seedではない。実機では出口3候補と通路の違いを識別できる地形記録を残す。

この結果だけでStage 5を製品合格にせず、「構造は維持、実機で待機・搭乗を確認」とする。次のASCII設計確認対象はStage 6。

## 再現

リポジトリルートで実行。出力先は毎回新しいディレクトリを指定する。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim render 5 --stage-file design/reviews/stage-05/main-stage-5.ron --seed 0 --coordinates
sim check 5 --stage-file design/reviews/stage-05/main-stage-5.ron \
  --plan design/reviews/stage-05/boarding-and-crossing.plan \
  --seeds 100 --walk-to-goal --require-initially-unreachable --reject-blocked \
  --output /tmp/stage05-crossing-review
sim simulate 5 --stage-file design/reviews/stage-05/main-stage-5.ron \
  --plan design/reviews/stage-05/seed-0-replay.plan --seed 0 --require-goal
```

負の対照（いずれも非0終了が今回の観測結果）：

```bash
sim check 5 --stage-file design/reviews/stage-05/main-stage-5.ron \
  --seeds 100 --walk-to-goal --output /tmp/stage05-static-review
sim check 5 --stage-file design/reviews/stage-05/main-stage-5.ron \
  --plan design/reviews/stage-05/early-departure.plan \
  --seeds 100 --walk-to-goal --output /tmp/stage05-early-review
```
