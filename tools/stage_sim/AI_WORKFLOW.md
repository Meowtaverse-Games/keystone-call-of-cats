# AI生成 → 軽量チェック → 修正の作業契約

このCLIはAIが生成した候補を検証する道具です。AIモデルの呼び出しや製品への自動採用はしません。候補を作り、コマンドの結果を読み、修正を繰り返すために使います。

## AIへの依頼例

> Type4が1個の配置導入面を、別の候補ファイルとして作ってください。
> 必須課題は足場を1個置いて高所へ進むこと。入口から石へ行き、置いた足場へ乗り、出口へ進む遊びを残してください。
> ランダムチャンクは課題を解いた後の道中のフレーバーに限定します。
> 同じ石の解法で100 seedを検証し、実際のゴール到達、初期状態の抜け道、無効操作、配置上限をチェックしてください。
> 失敗したseedはRON・計画・JSONを保存して原因を説明し、必要な箇所だけ直してください。
> 製品assetsや既存の完成面を変更しないでください。

## 1. 先に固定するブリーフ

- 面の遊びと新しく使う能力。既存面の改善なら、残す地形と完成済みの解法。
- 石種、個数、採掘上限（石別）、配置上限（全石共有）。
- 入口→石→乗降／地形変更→出口の動線。
- ランダムに変えてよい部分と、変えてはいけない主要課題。
- 合格条件。操作不要の面に「初期到達不可」を課さない。

AIはブリーフを満たす候補RONと`.plan`を作ります。必要なら製品用の`.rhai`/`.ks`も別ファイルで添えますが、軽量チェックした操作計画を製品言語の実行証跡に読み替えません。

## 2. RONと計画

例は`examples/ai-candidate.ron`、`examples/ai-candidate.plan`、`examples/ai-candidate.rhai`です。いずれも製品未登録のコミット済み候補です。

RONは既存の`start_chunks`、`middle_chunks`、`goal_chunks`を使います。記号は`@`、`S`、`#`、`G`、接続の`I`/`E`など。`map_size`は30×20以内。石種は1面1種類です。

必須の石操作を共通のplanに書き、道中のプレイヤー操作は`--walk-to-goal`で探索・再生できます。石に乗ったまま移動するなど、石の操作とプレイヤー操作が交互に必要な面では、それもplanへ明示します。自動歩行は石を追加操作しません。

## 3. 検証と修正

```bash
cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  check 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --plan tools/stage_sim/examples/ai-candidate.plan \
  --walk-to-goal --require-initially-unreachable --reject-blocked \
  --seed 0 --seeds 100 --output /tmp/ai-stage-attempt-001
```

AIは終了コードだけでなく`report.json`を読みます。

1. RON構文・生成・world構築エラーか、手順の失敗かを分ける。
2. `initially_reachable`で意図した課題を飛ばせないか確認する。ただし静止障害物モデルの範囲。
3. `goal_reached`を確認する。`goal_reachable`だけで到着済みとしない。
4. `metrics.blocked_actions`、残数、失敗行、初期／最終地形を読む。
5. seedと選択チャンクを使って1件だけ再現する。
6. 失敗理由に対応する最小修正を行い、新しい出力ディレクトリで同じseed群を再チェックする。
7. `distinct_initial_layouts`を確認する。フレーバーの変化が本当に生成されているかを見る。

制約に合わない候補を通すために、上限を勝手に増やす、石種を変える、必須課題を削る、assertを弱めることはしません。予算超過・近似モデルの不足・解法計画の不足は区別して記録します。

## 4. 負の対照

同じ例で`--place-limit 0`を指定すれば配置できず、失敗するはずです。候補に合わせて検査が常に成功していないことを確認できます。

```bash
cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  check 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --plan tools/stage_sim/examples/ai-candidate.plan \
  --walk-to-goal --require-initially-unreachable --reject-blocked \
  --place-limit 0 --seeds 3 --output /tmp/ai-stage-negative-001
```

このコマンドの期待値は非0終了、3件の失敗記録です。

## 5. AIが返す成果物

- 候補RON、共通の石操作と必要なプレイヤー操作、製品用コード案。
- 何が楽しいと考えるか、既存面から何を残したか。
- 試したseed範囲・実際の地形種類数・到着結果・所要時間。
- 失敗例と修正理由、保存した証跡へのパス。
- 未確認の製品物理、タイミング、実機クリア、人の面白さ。

軽量モデルで通った候補だけ実機試遊へ進めます。採用は面ごとに行い、既存完成面をこの例のような単純な実験地形へ置き換えません。
