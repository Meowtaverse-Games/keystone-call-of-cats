# stage_sim — Bevyを起動しないステージ実験室

地形生成、プレイヤー操作、石の移動・採掘・配置、手順の保存と再生、複数seedの一括検証を行う、小さなCLIです。AIが生成した候補RONと解法も同じ手順でチェックできます。

**判定対象は1マス単位の近似モデルです。** Bevyの物理・タイミング・実機クリア・人が感じる面白さを保証するものではありません。候補を速く絞り込み、残ったものを実機で確認します。本体のビルドは不要です。

## まず試す

以下はリポジトリルートで実行します。以降の`sim`は同じシェル内で使う関数です。

```bash
sim() { cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- "$@"; }
sim play 13 --stage-file tools/stage_sim/examples/ai-candidate.ron
```

この実験用候補では、石に足場を1個置くと高所への経路が開きます。製品ステージとしては登録していません。

```text
show
status
assert unreachable
s 0 place up
route
walk-goal
assert goal
reset
```

`route`は**現在の地形・静止した石**に対するプレイヤーだけの経路を表示します。`walk-goal`はその操作を実際に再生し、ゴール到着を確認します。石のコードや全パズルを自動で解く機能ではありません。

操作ごとに`show`で状態を見られます。`quit`で終了します。

## 操作一覧

```text
p left | p right
p jump | p jump-left | p jump-right
p leap-left | p leap-right
s 0 move right
s 0 dig down
s 0 place up
reachable
reachmap
route
walk-goal
assert goal
assert reachable
assert unreachable
reset
```

石番号はRONのチャンク走査順で、複数石は画面上の`0`、`1`などで区別します。手動コマンドも石種の能力を守り、`dig`はType3、`place`はType4のみです。採掘残数は石ごと、配置残数は全石共有です。RONの`place_limit`を読み、`--place-limit N`で試験用に上書きできます。未指定は製品と同じく無制限です。

`@`=プレイヤー、`S/0-9`=石、`*`=岩、`#`=外周、`O`=障害物、`?`=動的地形、`+`=置いた足場、`G`=ゴール。

## 候補とseedの指定

```bash
sim render 3 --stages-dir assets/stages --seed 42 --coordinates
sim analyze 11 --stages-dir assets/stages
sim render 13 --stage-file tools/stage_sim/examples/ai-candidate.ron --seed 7
```

`--stage-file`は単独の候補ファイルを直接指定します。省略時は`--stages-dir`以下の`stage-N.ron`を使います。`assets/stages`は現在チェックアウトしているブランチの面です。設計用ブランチではmainと異なる候補を含むため、mainの完成面を確認する場合はmainのRONを別ディレクトリへ取り出し、`--stages-dir`または`--stage-file`で明示してください。候補を試すときは、コミット済みの`tools/stage_sim/examples/`か、明示した候補ファイルを使います。

## 操作を記録・再生する

```bash
sim play 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --seed 7 --record /tmp/my-stage.plan
sim simulate 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --seed 7 --plan /tmp/my-stage.plan --frames --coordinates --require-goal
```

記録ファイルは新規作成のみで、既存ファイルを上書きしません。受理した操作とリセット・assertを記録し、`walk-goal`は具体的なプレイヤー操作へ展開します。失敗した構文・能力違反コマンドは記録しません。ブロックされた移動などは受理された操作として残ります。

記録の再生には同じRON・seed・上書き設定が必要です。候補を後から編集する場合は、下記の`check --output`で元RONも保存してください。

`simulate`は従来互換で「最後にゴールへ行ける状態」も成功とします。**到着を確認するときは`--require-goal`、または手順末尾の`assert goal`を使います。**

## 多数のランダムチャンクを一括検証する

```bash
sim check 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --plan tools/stage_sim/examples/ai-candidate.plan \
  --seed 0 --seeds 100 --walk-to-goal \
  --require-initially-unreachable --reject-blocked \
  --output /tmp/stage-check-001
```

同じ石の操作を全seedで試し、その後のプレイヤー経路を探索・再生します。石のコードは共通のまま、フレーバー部分の道中に違いがあっても到達するかを試せます。

- `--seed`が最初、`--seeds`が連続する件数。1〜10000件まで。
- `--walk-to-goal`: 手順の後でプレイヤー経路を再生する。石はそれ以上動かさない。
- `--require-initially-unreachable`: 初期の静止した石のまま抜けられる候補を落とす。操作不要のStage 1では付けない。
- `--reject-blocked`: 無効移動、空振り、資源不足などの時点でそのseedを失敗にする。
- `--expect goal`（既定）: 操作後に**実際にゴール到着**していること。
- `--expect reachable`: 現状態から静止した石で歩いて到達可能なこと。未到着は別フィールドに残す。
- `--expect unreachable`: 現状態からプレイヤーだけの経路がないこと。パズルが解けないという意味ではない。
- `--expect generated`: 地形生成とWorld構築だけ。プレイ可能と判定したことにはならない。

一部のseedが失敗しても残りを調べ、1件でも失敗なら終了コードを非0にします。生成探索には予算があり、使い切った場合も失敗として残します。予算超過は「解が存在しない」という証明ではありません。

`--output`は親が存在する**新しいディレクトリ**を指定します。過去の証跡を上書きしません。

```text
report.json      全seedの判定・選択チャンク・操作・到達状況・時間・制約
source.ron       検証した候補のコピー
input.plan       入力手順のコピー
seed-N.json      失敗したseedの詳しい状態・エラー
seed-N.txt       失敗時の初期地形（生成できた場合）
seed-N.plan      失敗までの操作・assert
```

`report.json`には初期／最終地形、実到着と到達可能性の別々の値、実行したプレイヤー経路も含まれます。構文不正や生成失敗もJSONのエラーにします。入力ファイルが読めない等の起動エラーはstderrに出します。

再現は保存したRONと入力手順に、reportのseed・オプションを付けて行います。

```bash
sim check 13 --stage-file /tmp/stage-check-001/source.ron \
  --plan /tmp/stage-check-001/input.plan --seed 7 --seeds 1 \
  --walk-to-goal --require-initially-unreachable --reject-blocked
```

`distinct_initial_layouts`は地形の実際の種類数です。「100 seed成功」を「100種類の地形成功」と読み替えず、固定面なら1になることを確認します。

## AIで生成してチェックする

[AI生成の作業手順](AI_WORKFLOW.md)に沿い、候補RONと操作計画を作り、`check`のJSONを読みながら修正します。CLI自体は外部AIサービスを呼びません。APIキーやBevyの起動は不要です。

## Keystoneコードの実行（既存機能）

`run`はKeystoneコードを石番号順に受け取り、プレイヤー計画を1ラウンド1行で再生します（待機は`wait`）。時間待ちは秒を設計ラウンドへ丸めた近似です。コード・計画は検証対象と同じコミットで管理されたものを明示的に指定してください。

**この独立ツールのCargo.lockは既存のkeystone-lang版を固定しており、製品の現在の言語版とは同一ではありません。** この版の`run`には`place`イベントの対応がなく、Rhaiも実行しません。配置面は手動コマンド／操作計画で検証します。`run`の言語評価・センサーの能力制限も、製品と同等の受入検証には使わないでください。

## 近似の境界

- 連続物理、ジャンプの時間、接触の瞬間、石の移動速度、複数プログラムのフレーム単位の競合は再現しません。
- `O`の時間消滅はなく、`?`は静的な障害物です。動的地形の抽選・小数の石位置補正は適用しません。
- 到達探索はプレイヤーだけです。失敗は、その手順／そのモデルで届かなかったという結果であり、別解の不存在を意味しません。
- CLI seedは製品seedではありません。同じツール版とRONで再現してください。

## 開発時の検証

```bash
cargo test --locked --manifest-path tools/stage_sim/Cargo.toml
cargo fmt --manifest-path tools/stage_sim/Cargo.toml -- --check
```
