# 旧候補Stage 13〜20の検証

対象: `bcb599b` の `design/stages` と `design/solutions`。最新20面案の採用・完成判定ではない。結果と差分は[検証一覧](../README.md)を参照。

13〜16・20のJSONは100 seedを`check`で実行した要約とseed 0の詳細。全て実地形は1種類であり、ランダムチャンクの検証を通したという意味ではない。17〜19のログは既存コードとプレイヤーplanをseed 0で実行したもの。18のエラーはそのまま保存した。

リポジトリルートから再現（出力先は新規ディレクトリ）：

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
for stage_id in 13 14 15 16 20; do
  case "$stage_id" in
    13) budget=1 ;;
    14) budget=7 ;;
    15|20) budget=6 ;;
    16) budget=4 ;;
  esac
  sim check "$stage_id" --stages-dir design/stages \
    --plan "design/solutions/stage-${stage_id}-player.plan" \
    --place-limit "$budget" --seeds 100 --reject-blocked \
    --output "/tmp/old-candidate-${stage_id}-review"
done
```

17・19は成功、18は未到達で非0終了するのが今回の記録。下記を面ごとに実行して確認する（`stage_id`を17、18、19へ変える）：

```bash
stage_id=18
sim run "$stage_id" --stages-dir design/stages \
  --stone-script "design/solutions/stage-${stage_id}-stone-0.ks" \
  --stone-script "design/solutions/stage-${stage_id}-stone-1.ks" \
  --player-plan "design/solutions/stage-${stage_id}-player.plan" --max-rounds 100
```

`run`の言語版と時間の近似は製品Rhaiとは異なる。実機の同時実行を検証したものではない。
