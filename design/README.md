# Stage design candidates

`stages/`は、20ステージ再設計のための固定マップ候補です。レビューとBevy上の操作確認が終わるまでは`assets/stages/`を置き換えません。

各ステージは次の3点をセットで管理します。

- `stages/stage-N.ron`: 固定ASCIIマップ
- `solutions/stage-N-stone-M.ks`: 石ごとのKeystone想定解
- `solutions/stage-N-player.plan`: プレイヤーの操作計画または検証条件

全体の学習曲線と製品化順序は`stages-01-20.md`にまとめています。

- 商品・価格・発売日・品質ゲート: `release-and-implementation-plan.md`
- 実装担当向けのファイル所有・依存・受入条件: `stage-implementation-handoff.md`

## 検証

全20面を一括検証できます。

```bash
./design/verify-all.sh
```

製品カタログの1〜20、製品RONの解析、昇格済みStage 1〜4と設計正本の一致を軽量確認できます。

```bash
./design/verify-product-stage-catalog.sh
```

個別の解析・実行例：

```bash
cargo run --manifest-path tools/stage_sim/Cargo.toml -- \
  analyze 2 --stages-dir design/stages --coordinates

cargo run --manifest-path tools/stage_sim/Cargo.toml -- \
  run 2 --stages-dir design/stages \
  --stone-script design/solutions/stage-2-stone-0.ks
```

設計候補の合格条件：

1. ステージ1以外は初期状態でゴールへ到達できない。
2. 想定解の実行後にゴールへ到達可能になる。
3. その面で未習得の命令を必須にしない。
4. 石の無効移動や無駄な`dig`を想定解に含めない。
5. Bevy上でジャンプ距離、石への乗り降り、ゴール判定を確認する。

1〜4面のCLI検証はまとめて実行できます。

```bash
./design/verify-stages-01-04.sh
```

5〜8面も同じ形で検証できます。

```bash
./design/verify-stages-05-08.sh
```

9〜12面（採掘章）の検証：

```bash
./design/verify-stages-09-12.sh
```

13〜16面（CLI提案仕様のplace章）の検証：

```bash
./design/verify-stages-13-16.sh
```

17〜20面（複数ストーン章）の検証：

```bash
./design/verify-stages-17-20.sh
```
