# Stage implementation handoff — fixed 20-stage release

- 更新日: 2026-09-04
- 作業ブランチ: `feature/20-stage-release-handoff`
- 起点: `main` / `e49b1df`

## 1. 目的

CLIで解法成立を確認した固定20面を、Keystone: Call of Catsの製品ステージへ昇格する。面数やメカニクスを増やす作業ではない。2027-01-14のitch.io 1.0、2027-02-16のSteam 1.0を目標に、次を完成させる。

- 固定20面を製品上で開始、リセット、クリアできる。
- Stage 13〜16、20で必要な`place`を製品実装する。
- Stage 1〜6を30〜45分の無料デモにする。
- 各面に説明と3段階ヒントを用意し、外部テストで難易度を調整できる。

リリース判断、価格、日程、KPIは`design/release-and-implementation-plan.md`を参照する。商品計画の共有正本は次のGoogle Sheetsとする。

- https://docs.google.com/spreadsheets/d/1iXRJlJJ7kFDQ9uhlhcrkMdMKGxVCR6lGhB9WsJndCmw/edit

## 2. 変更してはいけない決定

- 1.0は固定20面。Stage 21以降を追加しない。
- ランダム、変数、石間通信を必須解法にしない。
- 章構成は4面ずつの5章とする。
  - 1〜4: 導入
  - 5〜8: 制御
  - 9〜12: 採掘
  - 13〜16: 道づくり
  - 17〜20: 複数石
- `place`の表記は`place up/down/left/right`。
- `place_limit`はステージ全体で共有する。占有マスへの失敗では消費せず、ステージリセットで配置物と残数を復元する。
- Type4は`move / is_touched / is_empty / place`を使える。
- このサーバではBevy本体をビルドしない。

これらを変える必要が生じた場合は、実装内で独自判断せず、V2シートとリリース計画の変更提案として切り出す。

## 3. 正本と役割

| 正本 | 役割 |
| --- | --- |
| `design/stages/stage-N.ron` | 固定地形と石配置 |
| `design/solutions/stage-N-*.ks` | 現行Keystone言語で実行できる石コード |
| `design/solutions/stage-N-player.plan` | プレイヤー操作とCLI再生手順 |
| `design/stages-01-20.md` | 全体カリキュラムと章構成 |
| `design/stages-XX-YY.md` | 面別の意図、想定解、Bevy確認点 |
| `./design/verify-all.sh` | 20面の初期到達不可と想定解到達を確認する回帰 |
| Google Sheets V2 | 難易度、目標時間、商品状態、実装計画、プレイテスト記録 |

Stage 13〜16、20は、`keystone-lang`に`place`がないため`.ks`ではなくCLI操作計画で検証している。これは製品実装済みという意味ではない。

## 実装進捗スナップショット

2026-09-04時点:

- 製品カタログと`assets/stages/list.ron`はStage 1〜20へ整合済み。
- 既存能力で解けるStage 1〜12・17〜19は固定設計RONを`assets/stages/`へ昇格済み。製品RONを使うCLI正解検証も成功済み。
- Stage 13〜20のRONはカタログからロード可能。ただし13〜16・20は`place`未実装のため製品クリア未達。
- 次の実装対象はWP2の`keystone-lang`への`place`追加。
- 全面のBevy物理・UI確認は別環境で未実施。

## 4. 現在の製品コードとの差分

### ステージ登録

- `assets/stages/`、`StageMeta::load_map`、`assets/stages/list.ron`はStage 1〜20へ整合済み。
- Stage 1〜12・17〜19は固定設計RONへ昇格済み。
- Stage 13〜20は固定設計RONを配置済み。ただし13〜16・20は`place`実装後に製品クリア確認が必要。
- `design/stages/`にはCLI検証済みの固定Stage 1〜20があり、製品RON昇格時の正本とする。

Stage 21〜23は一覧から削除済み。データ面の次の差分は`place_limit`追加後にStage 13〜16・20へ上限値を反映すること。

### `place`

- `src/resources/stone_type.rs`に`Type4`はあるが、能力登録がコメントアウトされている。
- `src/util/script_types.rs`の`ScriptCommand`は`Move / Sleep / Dig`だけ。
- `src/resources/script_engine/keystone_executor.rs`は`keystone_lang::Event`を`Move / Sleep / Dig`へ変換している。
- `src/scenes/stage/systems/stone.rs`の実行アクションにも配置処理はない。
- `ChunkGrammarConfig`、`Map`、石スポーン状態は`dig_limit`だけを持ち、`place_limit`を持たない。
- CLIの`tools/stage_sim`には設計検証用の配置処理がある。

`place`の言語構文とイベント追加は外部依存`keystone-lang`側の変更が先に必要になる。依存は現在Gitリビジョン`057ca26c...`を参照している。

### 複数石

- 複数石の製品基盤は既に存在する。
- Stage 17〜19は既存の`move / sleep / dig`だけでCLI実行済み。
- Stage 16と20だけが複数石＋`place`を必要とする。

## 5. 実装ワークパッケージ

依存関係を守り、1パッケージをレビュー可能な1〜数コミットにする。

### WP1 — 既存能力15面の昇格

対象: Stage 1〜12、17〜19

主な所有ファイル:

- `assets/stages/stage-1.ron`〜`stage-12.ron`
- 新規`assets/stages/stage-17.ron`〜`stage-19.ron`
- `assets/stages/list.ron`
- `src/resources/stage_catalog.rs`

作業:

1. `design/stages/`の該当RONを製品側へ移す。
2. `StageMeta::load_map`をStage 20まで扱える形にする。
3. `list.ron`を1〜20へ揃える。初期アンロック方針は現在どおりStage 1〜3を維持し、クリア進行を確認する。
4. Stage 17〜19で石が2個生成され、石番号とエディタが一致することを確認する。

受入条件:

- カタログ件数と製品RONが20面分一致する。
- 既存能力の15面について、設計RONと製品RONの意味的差分がない。
- Stage 1〜12の既存セーブを読み込んでもパニックしない。
- Stage 21〜23がステージ選択へ出ない。

### WP2 — `keystone-lang`への`place`追加

対象リポジトリ: `Meowtaverse-Games/keystone-lang`

作業:

1. `place <direction>`を構文として追加する。
2. 実行イベントに方向付き`Place`を追加する。
3. `up`と既存内部表記`top`の扱いを`move`と一致させる。
4. 正常系4方向、構文エラー、無限ループ安全制限のテストを追加する。
5. keystone_cc側の依存リビジョンをレビュー済みコミットへ更新する。

受入条件:

- `place down`が1つの`Place(Down)`イベントになる。
- 許可能力に`place`がない石では実行できない。
- 既存の`move / sleep / dig`テストが回帰しない。

### WP3 — 製品側`place`基盤

主な所有ファイル:

- `src/util/script_types.rs`
- `src/resources/script_engine/keystone_executor.rs`
- `src/resources/script_engine/rhai_executor.rs`
- `src/resources/stone_type.rs`
- `src/resources/chunk_grammar_map.rs`
- `src/scenes/stage/components.rs`
- `src/scenes/stage/systems/mod.rs`
- `src/scenes/stage/systems/stone.rs`
- `src/scenes/stage/systems/tiles.rs`
- `src/scenes/stage/systems/ui.rs`

実装仕様:

- `ScriptCommand::Place(MoveDirection)`を追加する。
- `ChunkGrammarConfig`と`Map`に`#[serde(default)] place_limit: Option<u32>`を追加する。旧RONは未指定でも読み込めること。
- 残数は石ごとではなくステージ共有リソース／コンポーネントとして管理する。
- 配置先は石の隣接1マス。通常地形、プレイヤー、他の石、既配置ブロック、ステージ外なら失敗する。
- 成功時だけ残数を1減らす。上限0なら短い非破壊アクションとして終了する。
- 配置物は通常地形と同じ衝突対象で、ASCIIの`+`に相当する視覚区別を持たせる。
- リセット時は配置物をすべて削除し、初期残数へ戻す。
- Type4能力は`move / is_touched / is_empty / place`。`dig`と`sleep`は付与しない。
- UIに残り配置数と`place`の命令例を表示する。

受入条件:

- 4方向へ配置できる。
- 成功、占有失敗、上限0、複数石の同時要求、リセットを単体またはECSテストで確認する。
- 同一フレームで2石が同じマスを要求した場合、確定した石番号順で1個だけ成功する。
- プレイヤーが配置物の上を歩ける。
- Stage 16と20で共有上限を超えない。

### WP4 — `place`5面の昇格

対象: Stage 13〜16、20

主な所有ファイル:

- 新規`assets/stages/stage-13.ron`〜`stage-16.ron`
- 新規`assets/stages/stage-20.ron`

作業:

- `design/stages/`から製品へ昇格する。
- Stage 13〜16、20へそれぞれ`place_limit`を設定する。
  - Stage 13: 1
  - Stage 14: 7
  - Stage 15: 6
  - Stage 16: 4（2石共有）
  - Stage 20: 6（3石共有）
- CLI操作計画を製品言語の`.ks`正解例へ置き換え、`design/solutions/`へ追加する。

受入条件:

- 5面すべてで、製品言語の正解コードとプレイヤー操作によりゴールへ到達できる。
- 配置前にはプレイヤー単独でゴールへ到達できない。
- リセット後に再度同じ正解を実行できる。

### WP5 — 学習UI、ヒント、3言語

主な所有ファイル:

- `assets/locales/ja-JP/stages.ftl`
- `assets/locales/en-US/stages.ftl`
- `assets/locales/zh-Hans/stages.ftl`
- `src/scenes/stage/systems/ui.rs`

各面に次を用意する。

1. 開始時の目的説明。
2. 新しい命令の説明と短い例。
3. ヒント1: 目的の言い換え。
4. ヒント2: 使う命令。
5. ヒント3: 最初の1〜2行またはコード骨格。
6. クリア後の短い振り返り。

受入条件:

- 20面×3言語でキー欠けがない。
- 章の導入面1、5、9、13、17では新概念以外を説明しすぎない。
- Stage 1〜6を初見で遊んだ中央値が30〜45分に収まる。

### WP6 — デモ、セーブ、リリースゲート

作業:

- デモはStage 1〜6だけ選択できるビルド設定にする。
- 製品版がデモの進行ファイルを引き継げることを確認する。
- Windows/Linux、キーボード／コントローラー表示、3言語を確認する。
- プレイテスト結果をGoogle Sheets V2の`プレイテスト`タブへ1人×1面で記録する。

発売判定:

- 外部テスター15人以上。
- Stage 1〜4の初見到達率90%以上。
- 20分以上進展なしが25%を超える面がない。
- 操作開始者の全20面完走率50%以上。
- クラッシュなしセッション99%以上。

## 6. 低負荷検証

このサーバで許可する検証:

```bash
cargo test --manifest-path tools/stage_sim/Cargo.toml
./design/verify-all.sh
./design/verify-product-stage-catalog.sh
cargo fmt --all -- --check
```

`./design/verify-all.sh`は2026-09-04時点で20/20成功、約3.7秒、最大RSS約35MB。

このサーバで実行しないもの:

```text
cargo build
cargo check（ルートパッケージ）
cargo run（Bevy本体）
cargo test（ルートパッケージ全体）
```

Bevy本体のビルド、物理挙動、乗車、入力猶予、演出、UIは別PCまたはCPU／メモリ上限を設定したCIで確認する。

## 7. Bevy実機確認チェックリスト

- 石の移動中にプレイヤーが滑り落ちない。
- `sleep`の待機時間が通常操作に対して短すぎない／長すぎない。
- 横・上からの`is_touched()`判定と説明が一致する。
- 掘削／配置演出の完了前に次命令が不自然に始まらない。
- 複数石の同時実行順がフレームレートで変わらない。
- 選択中の石番号と編集対象が視覚的に一致する。
- 配置物がプレイヤーと石の双方に正しく衝突する。
- リセット、次面、ステージ選択への遷移で動的状態が残らない。
- 想定外の短縮解、詰み、石を使わない抜け道がない。

## 8. リリース工程との接続

| 日付 | ゲート | 必要な状態 |
| --- | --- | --- |
| 2026-10-09 | 既存能力15面完了 | WP1完了、別環境Bevy確認 |
| 2026-11-06 | 固定20面機能完成 | WP2〜WP4完了 |
| 2026-12-02 | デモ・製品UI完成 | WP5、WP6のデモ範囲完了 |
| 2026-12-03 | Steam Coming Soon・デモ | ストア素材と配布物を公開可能 |
| 2027-01-08 | 難易度凍結 | 外部テストKPI通過 |
| 2027-01-14 | itch.io 1.0 | US$7.99／980円、初週10%オフ |
| 2027-02-16 | Steam 1.0 | itch修正反映、初週10%オフ |

日付より品質ゲートを優先する。ゲート未達時は発売日を延期し、メカニクスや面数を増やさない。

## 9. 最初の着手単位

最初の実装はWP1のStage 1〜4とする。`design/stages/stage-1.ron`〜`stage-4.ron`を製品側へ昇格し、カタログを20面対応へ広げる変更を分けてレビューする。その後Stage 5〜8、9〜12、17〜19の順に進める。

最初から`place`へ着手しない。既存能力15面を先に製品へ通すことで、地形座標、乗車、クリア、セーブの差分を早く把握し、`place`実装後の原因切り分けを容易にする。
