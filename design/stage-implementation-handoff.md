# Stage implementation handoff — fixed 20-stage release

- 更新日: 2026-09-27
- 作業ブランチ: `feature/20-stage-release-handoff`
- 起点: `main` / `e49b1df`

## 1. 目的

CLIで解法成立を確認した20面の候補を、Keystone: Call of Catsの製品ステージとして検討する。CLIの成立は製品クリアや遊びの完成を意味しない。特に既存面の地形を平坦な教材へそろえる案は再設計中であり、既存の地形・高低差・複数石が担う役割を個別に評価する。面数やメカニクスを増やす作業ではない。2027-01-14のitch.io 1.0、2027-02-16のSteam 1.0を目標に、次を完成させる。

- 固定20面を製品上で開始、リセット、クリアできる。
- Stage 13〜16、20へ`place_limit`を設定し、製品実機で解法を受入確認する。
- Stage 1〜6を30〜45分の無料デモにする。
- 各面に説明と3段階ヒントを用意し、外部テストで難易度を調整できる。

リリース判断、価格、日程、KPIは`design/release-and-implementation-plan.md`を参照する。商品計画の共有正本は次のGoogle Sheetsとする。

- https://docs.google.com/spreadsheets/d/1iXRJlJJ7kFDQ9uhlhcrkMdMKGxVCR6lGhB9WsJndCmw/edit

## 2. 変更してはいけない決定

次の章構成と`place`仕様は既存の候補計画である。製品地形の面白さを犠牲にして機械的に合わせる基準ではなく、実装・採用前に設計レビューを要する。

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
| `design/stages/stage-N.ron` | CLI研究用の固定地形候補と石配置。無条件の製品正本ではない。 |
| `design/solutions/stage-N-*.ks` | 現行Keystone言語で実行できる石コード |
| `design/solutions/stage-N-player.plan` | プレイヤー操作とCLI再生手順 |
| `design/stages-01-20.md` | 全体カリキュラムと章構成 |
| `design/stages-XX-YY.md` | 面別の意図、想定解、Bevy確認点 |
| `./design/verify-all.sh` | 20面候補の初期到達不可と想定解到達を確認する回帰。製品クリアの証明ではない。 |
| Google Sheets V2 | 難易度、目標時間、商品状態、実装計画、プレイテスト記録 |

Stage 13〜16、20は、製品側の`place`基盤を利用できる。ただし候補のCLI操作計画は製品クリアの証明ではなく、各面の上限設定・実機操作・解法確認が必要である。

## 実装進捗スナップショット

2026-09-27時点:

- 製品カタログと`assets/stages/list.ron`はStage 1〜20へ整合済み。
- 既存能力で解けるStage 1〜12・17〜19は候補RONを`assets/stages/`へ反映している。ただしCLIの正解検証は製品でのゴール、操作感、既存面の持ち味を保証しない。
- Stage 13〜20のRONはカタログからロード可能。ただし13〜16・20は`place_limit`設定と製品クリア確認が未完である。
- `keystone-lang`の`feature/place-command`（`fe41c163`、履歴上は`a655aca`から参照可能）には`place <direction>`と`Place(Direction)`の準備がある。一方、製品側には`Place`の変換・能力・地形処理がないため、このリポジトリの依存はmainの既存リビジョンへ戻している。依存更新と製品`place`基盤は将来の独立PRとして扱う。
- 次の実装対象は、まず候補地形の再設計と実機確認である。WP3の製品側`place`基盤は未着手であり、この文書の詳細仕様は実装開始時に再確認する。
- 全面のBevy物理・UI確認は別環境で未実施。

2026-09-27時点のA1X実機確認追記:

- Stage 1〜12・17〜19の全15面で、隔離セーブ領域からのロードと正本コードのF3実行・停止・再実行を確認した。詳細は[実機確認記録](verification-2026-09-27-a1x.md)を参照する。
- この確認は短時間の実行経路までであり、全15面のゴール到達、全コマンド完了、全リセット完全性は未確認である。WP1を完了扱いにしない。
- Stage 1〜3で `Goal reached!` と次面遷移を確認済み。Stage 3→4では隔離した `stage_progress.ron` の `unlocked_until: (4)` により進行保存も確認した。残るゴール確認はStage 4〜12・17〜19の12面である。
- Stage 8の有限`loop 6`を前提にした旧候補の記録を、再設計後の製品成立証跡へ流用しない。このブランチではRhaiの`loop { if is_touched() { move("right"); } }`を用いる「石の待つ谷へ」試作を扱う。地形・解法・試し方は`prototypes/stage-8-playful-v1.md`を参照する。Keystone言語の有限`loop`は対象外で、A1Xでの操作感、クリア、リセットは未確認である。
- stage_simを製品に合わせて石番号順・石ごとの掘削残数へ整合した後、Stage 18の既存プレイヤー計画はゴール前で終了することが分かった。このブランチでは地形や解法を変更せず、候補の再検証課題として残す。そのため`verify-all.sh`はStage 18で失敗する。Stage 19の既存解法とStage 20のCLI配置計画は個別に通過したが、いずれも製品クリアの証明ではない。
- Stage 13の`place`縦切り（WP3）は未着手であり、既存能力15面の確認結果を`place`実装の受入証跡へ流用しない。

## 4. 現在の製品コードとの差分

### ステージ登録

- `assets/stages/`、`StageMeta::load_map`、`assets/stages/list.ron`はStage 1〜20へ整合済み。
- Stage 1〜12・17〜19は候補RONを製品側へ反映している。旧mainの地形を一括復元する案は未採用で、別ブランチのローカル退避にのみ保存されている。
- Stage 13〜20は候補RONを配置済み。ただし13〜16・20は`place`実装後に製品クリア確認が必要である。
- `design/stages/`にはCLI検証済みの候補Stage 1〜20がある。製品RONへの採用は、面ごとの設計・実機確認を経て判断する。

Stage 21〜23は一覧から削除済み。データ面の次の差分は`place_limit`追加後にStage 13〜16・20へ上限値を反映すること。

### `place`

- `src/resources/stone_type.rs`に`Type4`はあるが、能力登録がコメントアウトされている。
- `keystone-lang`側には`place <direction>`と`Place(Direction)`を追加済み。
- `src/util/script_types.rs`の`ScriptCommand`はまだ`Move / Sleep / Dig`だけ。
- `src/resources/script_engine/keystone_executor.rs`はまだ`keystone_lang::Event`を`Move / Sleep / Dig`だけへ変換している。
- `src/scenes/stage/systems/stone.rs`の実行アクションにも配置処理はない。
- `ChunkGrammarConfig`、`Map`、石スポーン状態は`dig_limit`だけを持ち、`place_limit`を持たない。
- CLIの`tools/stage_sim`には設計検証用の配置処理がある。

`keystone-lang`の`feature/place-command`は`fe41c163e2795f5a39173517d1f441be5efbfc73`にあるが、製品が対応するまで依存固定しない。製品側の能力制限、`up`から`MoveDirection::Top`への変換、配置処理はWP3を独立して実装・検証する。

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

進捗: 実装・単体テスト・ブランチ公開・依存固定まで完了。レビューとmainへのマージ待ち。

作業:

1. `place <direction>`を構文として追加する。
2. 実行イベントに方向付き`Place`を追加する。
3. `up`と既存内部表記`top`の扱いを`move`と一致させる。
4. 正常系4方向、構文エラー、無限ループ安全制限のテストを追加する。
5. keystone_cc側の依存リビジョンをレビュー済みコミットへ更新する。

受入条件:

- `place down`が1つの`Place(Down)`イベントになる。
- 4方向、型エラー、構文エラー、遅延実行をテストできる。
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

#### 推奨実装順 — Stage 13の縦切りを先に完成させる

5面を同時に作り込まず、最初にStage 13だけで「開始→配置→移動→クリア→リセット」を最後まで成立させる。Stage 13が別環境のBevy確認を通るまで、Stage 14〜16・20の製品調整へ広げない。

1. 低負荷の配線とデータモデル
   - `ScriptCommand::Place(MoveDirection)`を追加する。
   - `keystone_lang::Event::Place`を製品コマンドへ変換し、`Direction::Up`を既存どおり`MoveDirection::Top`へ対応させる。
   - Keystone実行でも`allowed_commands`を適用し、`place`を持たない石ではコンパイル時に拒否する。
   - Type4の能力を`move / is_touched / is_empty / place`へ確定する。
   - `ChunkGrammarConfig`と`Map`へ`#[serde(default)] place_limit: Option<u32>`を通し、旧RONの読み込み互換を保つ。
2. Bevyから分離した配置判定
   - 石番号、要求方向、配置先、残数、占有状態から成功／失敗を返す純粋な判定単位を作る。
   - 地形、プレイヤー、石、既配置物、範囲外、残数0を拒否する。
   - 同一フレームの要求は石番号順で確定し、成功時だけ共有残数を減らす。
   - この層はルートBevyビルドを使わず単体検証できる形を優先する。
3. Stage 13の製品縦切り
   - `place_limit: Some(1)`を設定し、製品Keystoneコードの正解例を用意する。
   - 成功した配置を、通常地形と同じ衝突を持つ視覚的に区別可能な配置物として生成する。
   - 残数UI、上限0／失敗時の非破壊フィードバック、実行停止とリセット時の完全復元を接続する。
   - CLIで初期到達不可、想定解後到達、再リセット後の再現を確認する。
4. 別環境Bevyゲート
   - Stage 13の開始、4方向配置、配置物への乗降、クリア保存、リセットを実機確認する。
   - 当サーバではBevyをビルドせず、別PCまたはCPU／メモリ制限付きCIで行う。
   - このゲートを通過したら同じ基盤をStage 14〜16・20へ展開する。

推奨コミット境界:

1. コマンド変換、能力制限、`place_limit`データモデル、後方互換テスト。
2. 共有残数と決定的な配置判定、成功／失敗／競合／リセットのテスト。
3. Stage 13の地形生成、衝突、UI、正解コード、縦切り受入確認。
4. Stage 14〜16・20のデータと正解コード。面ごとの検証結果を混ぜず追跡可能にする。

### WP4 — `place`5面の昇格

対象: Stage 13〜16、20

主な所有ファイル:

- `assets/stages/stage-13.ron`〜`stage-16.ron`
- `assets/stages/stage-20.ron`

作業:

- `design/stages/`から製品へ昇格する。
- Stage 13〜16、20へそれぞれ`place_limit`を設定する。
  - Stage 13: 1
  - Stage 14: 7
  - Stage 15: 6
  - Stage 16: 4（2石共有）
  - Stage 20: 6（3石共有）
- CLI操作計画を製品言語の`.ks`正解例へ置き換え、`design/solutions/`へ追加する。

面別の展開ゲート:

- Stage 13: 上へ1個置く基本配置。まずこの面だけで製品縦切りを完成させる。
- Stage 14: 下へ7個を連続配置し、一本の橋と最後の退避を確認する。
- Stage 15: 2つの穴へ合計6個を過不足なく振り分け、配置上限の判断を確認する。
- Stage 16: 2石が共有上限4を分担し、石番号順の決定性と同時要求を確認する。
- Stage 20: 3石が共有上限6で橋を分担し、複数石と`place`の総合問題として確認する。

占有拒否、失敗時に残数を消費しないこと、上限0、範囲外は特定ステージの学習要素にせず、WP3の自動テストで常時保証する。

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

検証は次の三層に分ける。

1. CLI／純粋ロジック: 正本一致、初期到達不可、想定解、残数、競合、リセット。
2. Bevy別環境: 開始、物理、衝突、表示、入力、保存、画面遷移。
3. 人間のプレイテスト: 初見時間、再試行、ヒント、離脱、仕様誤解。

自動検証が成功してもBevy確認前は「確認待ち」、Bevy確認が成功しても外部テスト前は難易度を確定扱いにしない。

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
