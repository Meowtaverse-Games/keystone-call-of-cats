# stage_sim

Bevyを起動せず、既存の`assets/stages/stage-N.ron`を固定シードで組み立てて確認するための設計用CLIです。

これはピクセル単位の物理検証ではなく、ステージ設計を速く反復するための1マス単位のモデルです。最終的なジャンプ感、接触判定、石に乗ったときの挙動はBevy版でも確認します。

## 表示

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- render 11 --coordinates
```

生成型ステージは`--seed`で同じチャンク配置を再現できます。

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- render 3 --seed 42
```

## 構造分析

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- analyze 11 --seed 0
```

石を動かさない初期状態での概算到達範囲、石・障害物・ゴール数などを表示します。

## 対話シミュレーション

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- play 11 --place-limit 3
```

主なコマンド：

```text
show
status
reachable
reachmap
p left
p right
p jump-right
p leap-right
s 0 move right
s 0 dig down
s 0 place right
assert goal
assert reachable
reset
quit
```

複数ストーンはASCII上で`0`、`1`、`2`として表示されます。

## 操作計画の再生

1行1操作のテキストを作り、検証可能な想定解として保存できます。

```text
# plans/stage-11.txt
s 0 move right
p right
reachable
assert reachable
```

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- simulate 11 \
  --plan plans/stage-11.txt --frames --coordinates
```

## Keystoneプログラムの実行

ゲーム本体と同じ`keystone-lang`を使い、石ごとのプログラムをグリッドモデル上で実行できます。

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- run 11 \
  --stone-script plans/stage-11-stone-0.ks \
  --stone-script plans/stage-11-stone-1.ks \
  --player-plan plans/stage-11-player.txt \
  --frames --coordinates
```

プレイヤー計画は1ラウンド1行です。何もしないラウンドは`wait`と書きます。現在の`keystone-lang`が扱う`move`、`sleep`、`dig`、条件判定、`send`、`receive`を実行できます。`place`はまだ言語側に存在しないため、対話モードと操作計画モードで先行検証します。

## 記号

- `@`: プレイヤー
- `S`または`0`〜`9`: 石
- `*`: 通常の地形
- `#`: ステージ外周
- `O`: 時間で消える障害物
- `?`: 動的地形
- `+`: `place`で置いた足場
- `G`: ゴール

## 設計上の利用

各ステージについて、RONと一緒に次を管理すると設計と検証を接続できます。

- 固定シード
- 想定解の操作計画
- 想定操作数
- `dig`・`place`の使用数
- 初期状態でゴールへ到達できないこと
- 想定解の終了後にゴールへ到達できること
