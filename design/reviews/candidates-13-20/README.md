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

## 操作を省いた対照試験

seed 0で、既存planからプレイヤー操作を外して石の操作だけを抽出し、指定した石の操作も省いてから`--walk-to-goal`でプレイヤー経路を探索・実行した。配置上限は上記と同じ。結果は`role-controls`以下。

- 13〜15: 石操作を全て省くと経路なし。
- 16: 石操作なし、石0の操作だけ省略、石1の操作だけ省略の3試験とも経路なし。
- 20: 石操作なし、石0・1・2の操作をそれぞれ省略した4試験とも経路なし。

合計10試験はすべて`no player-only route after plan`で非0終了。無効な移動や資源不足で失敗したのではない。これは既存手順で各役割が効く証拠であり、別のコードや操作順でも1石で解けないという証明ではない。20は現在の3石用経路から1石を消すだけでは成立しないため、最新案の2石化には地形と役割を一緒に設計する必要がある。

再現例（到達しない対照試験なので非0終了が今回の観測結果）:

```bash
sim check 16 --stages-dir design/stages --place-limit 4 \
  --plan design/reviews/candidates-13-20/role-controls/16-omit-0/input.plan \
  --walk-to-goal --reject-blocked --output /tmp/stage16-omit0-review
```

## Stage 18の失敗原因: 石コードの割り当て

追加の比較で、現在の石番号（上からの走査順）と旧解法ファイルの割り当てが逆になっていることを確認した。

- 石0は上側(13,10)、石1は搭乗済みの下側(7,5)。
- 旧順序では上側だけが4回上昇し、下側はsleepで待つため、プレイヤーはround 5で足場から底へ落ちる。
- `.ks`の指定順だけを交換すると、同じRON・seed 0・プレイヤーplanでround 31に実到達。操作25、blocked 0。地形・コード本文・assertを弱めていない。

```bash
sim run 18 --stages-dir design/stages \
  --stone-script design/solutions/stage-18-stone-1.ks \
  --stone-script design/solutions/stage-18-stone-0.ks \
  --player-plan design/solutions/stage-18-player.plan --max-rounds 100
```

[成功ログ](stage-18-swapped-assignment.log)と既存順序の失敗ログを両方保持する。これは旧Type3候補での割り当て診断であり、最新案のType2接触乗り継ぎの完成ではない。旧コードには` sleep 15.0 `があり、sleepを持たないType2へ石種だけ変更することはできない。既存解法・検証スクリプトは変更していない。新案では石番号と役割を明示し、接触待ちへ設計し直す。
