# セッション引き継ぎ: 全20面の設計・ASCII検証とStage 13〜20候補

**このタスクの再開時は本ファイルを先に読む。** 2026-09-28の2つのhandoffは経緯の資料であり、次作業・未解決状態は本ファイルで更新する。Stage 10の83/100失敗やStage 18の割り当て調査を最初からやり直さない。

**Stage 13の続行更新:** [レビュー済み試遊ビルド](2026-09-29-stage13-reviewed-playtest-build.md)を準備済み。Draft #87、head `69530d9`、Windowsビルド・73テスト・独立レビュー承認まで完了。下記の「ビルドを準備する」は完了し、次はA1X接続後の実機試遊。旧実行物を使わず、リンク先の成功runとハッシュを照合する。

## 対象

- リポジトリ: `Meowtaverse-Games/keystone-call-of-cats`
- worktree / Git toplevel: `/home/ubuntu/repos/meowtaverse-games/keystone_cc`
- ブランチ: `feature/20-stage-release-handoff`
- source code / 候補HEAD: `15fcce535abfd963648a8520613377917d34555e`
- main確認値: `252efbea156f674b6be819656a676592fff55eaf`
- 記録日時: `2026-09-29T16:54:49+09:00`。GitHubとリモートrefを保存時に照合。
- worktree一覧: この統合ブランチのほか、ASCII PR、外部石制御、言語pin、Rhai接触待ち、Stage 8試作、能力制限、place等の専用worktreeがある。他worktreeは変更していない。prunableな過去worktreeもあり、本作業で削除しない。
- タスク: mainの既存面を生かして20面を設計する。ランダムチャンクは主要課題を保つフレーバーとし、軽量CLIで候補を操作・再生・多seed検証してから実機評価へ進める。

## 現在地

### 方針

- Stage 1〜4はmainのASCII構成をユーザー了承済み。再設計・フレーバー追加を必須にしない。実機クリアや全seed合格の承認ではない。
- 5〜12もmainを原則継承。CLIに合わせて地形を平坦化しない。
- 13〜16はType4の道づくり、17〜20は2石の応用。1面は同一Type。HTTPは本編の石Typeに入れず、20面クリア後の解放方針（解放・進行連携は未実装）。
- 製品への採用はmainから面ごとの小さなPRで進める。Draft #82を一括マージしない。

### 完了した作業

1. ASCII検証基盤の[PR #86](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/86)はユーザーがmainへマージ済み。記録・再生・RON直接入力・多seedチェック・失敗証跡を利用できる。独立Astraレビュー承認はこのツールPRに対するもので、今回の全ステージ候補への承認ではない。
2. origin/mainの取り込み競合を解消し、`bcb599b`で統合。ユーザーが開始したmergeを完了し、レビュー済み記録処理を保持した。
3. mainのStage 5〜12を原本RONから検証し、[検証一覧](../../design/reviews/README.md)へ保存。製品assetsは未変更。
4. [Stage 13〜20のチャンク付き第1候補](../../design/prototypes/stages-13-20-v1/README.md)を作成。RON、操作plan、代表ASCII、seed別地形対応、結果、再実行スクリプトをコミット済み。
5. source HEADまで同じremoteブランチへpush済み。保存時のremote HEADはローカルと一致。

### main Stage 5〜12の結論

| 面 | 現在の確認結果 | 未確認・扱い |
| --- | --- | --- |
| 5 | 6地形、100/100到達。搭乗後に右4回。早発手順は届かない | 実機で待機秒数・搭乗・下車 |
| 6 | 5地形、上がってから運搬する共通手順で100/100 | 低い初案38/100は履歴。準備・乗車・高さ調整も必要 |
| 7 | 32地形、搭乗後上8回で100/100 | Type2の接触待ちと実機運搬は未検証 |
| 8 | 原本は時間消滅しないCLIのOで入口が閉じる。O除去の別データなら到達 | 消滅後を仮定した感度確認をmain合格に数えない |
| 9 | 2地形、左2回採掘→入口側へ石を出す共通手順で100/100 | 下向き採掘だけが正解ではない。実機乗降が残る |
| 10 | **解決済み**。13地形、5回採掘と乗り直しで100/100 | 初回83/100は手順不足。地形変更なし。2回の乗降の実機確認が残る |
| 11 | O除去の別データなら上側の石0だけで到達。3回採掘・採掘なし双方を再現 | 石1の役割を断定せず、2石とも残す。実機の時間消滅・別解確認 |
| 12 | 5回の縦掘削経路と採掘0回の迂回を両方再現 | 迂回が実機で成立するかを確認してから局所修正の要否を判断 |

原本は`design/reviews/stage-NN/main-stage-N.ron`。統合ブランチの`assets/stages`は旧候補を含み、main原本や新候補とは異なる。**実機起動時も、どのRONを読むかを必ず明示・照合する。**

### Stage 13〜20の第1候補

正本ディレクトリは`design/prototypes/stages-13-20-v1/`。旧`design/stages`や製品assetsを置換していない。

| 面 | 石・役割 | 100 seed中の地形数 |
| --- | --- | ---: |
| 13 | Type4×1、足場1個で高い足場へ | 75 |
| 14 | Type4×1、7個の橋を作り石を退避 | 72 |
| 15 | Type4×1、天然足場を挟んだ2区間に計6個を配分 | 16 |
| 16 | Type4×2、入口1個と橋3個。共有上限4 | 18 |
| 17 | Type1×2、中間足場を挟む水平運搬 | 4 |
| 18 | Type2×2、下側の上昇→上側の水平運搬 | 16 |
| 19 | Type3×2、別々の区間を各2回掘る。上限は各5 | 8 |
| 20 | Type4×2、低い橋2個→上り段差→高い橋3個。共有上限5 | 8 |

- 全8面で各100 seed、計800件の初期到達不可・操作後実到達・無効操作0を確認。
- 操作省略・配置不足の対照は26条件×100 seed。指定手順の失敗を確認したもので、別解不存在や最小資源の証明ではない。
- 保存したseed 0の完全操作列8本を正規CLIで再生し、ゴール到達を再確認。
- 20は初案で下側の石が静止足場になっていたため、候補内で初期位置を上げた。最終版は両石の操作が今回の手順で必要。
- 18の石0は上側、石1は下側。旧Type3候補はコード割り当てが逆で、指定順だけ交換すると到達した。この診断を踏まえて新候補は番号と役割を正しく対応。旧コードのsleepをType2に流用していない。
- `.plan`は逐次手動操作。特に17の待機・18の接触待ちを含む製品Rhaiコードは未作成・未実機検証。手動で通ったことを同時実行の成立と取り違えない。

### 検証・外部状態

- source HEAD `15fcce5`で[Draft PR #82](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/82)はOPEN/Draft。保存時にlint、Stage simulator、Windows verificationすべて成功を確認（fork noticeは正常skip）。handoff追加後のHEADのCIまで承認した意味ではない。
- 成功run: lint `36528809826`、Stage simulator `36528809918`、Windows verification `36528809934`。
- 本体のサーバー上でのビルド、実機起動、配布、マージ、HTTP解放の実装は今回行っていない。
- 独立ツールのみをビルド・実行。Bevyの連続物理、ジャンプ、接触、時間消滅、Rhai、実機の面白さは未確認。CLI seedは製品seedではない。
- `run`のkeystone-lang pinは`057ca26c`で製品言語と異なる。Rhaiやplaceの受入検証に流用しない。

## 続行範囲と次の正確な一手

### 再開後の追記（2026-09-29）

ユーザーから実機検証と優先テストプレイ面の提示を依頼された。HEAD `23c0a51`、同一remote ref、clean状態を確認して再開した。Stage 13の代表seed 0・1・2表示、seed 0完全plan再生、seed 2のクリア→reset→再クリアをCLIで確認済み。詳細と人の試遊手順は[PLAYTEST.md](../../design/prototypes/stages-13-20-v1/PLAYTEST.md)。

実機は未実施。A1Xの最終接続は9月28日12:16:40 JST、確認時点で約29時間経過し、稼働中ジョブなし。ユーザーへ起動・接続できるか質問済み。さらに製品RONは `StageMeta::load_map` の `include_bytes!` で埋め込まれるため、成功済みrun `36528809934` の実行物は新候補の試遊には使えない。外部RONの置換だけでは不十分。

次はA1X接続を再確認し、隔離checkoutでStage 13候補を製品assetsへ組み込んだ検証用ビルドを準備する。source SHA・候補RON hash・隔離セーブを記録し、PLAYTESTの乗降・ゴール・リセットを実施する。全seed検証のやり直しは不要。公開配布・マージは行っていない。

ユーザーは順次の設計・検証・候補作成を依頼済み。今回はセッション終了のための保存依頼であり、新しい採用・マージ・配布を指示したものではない。

1. 本worktreeでbranch・HEAD・dirty状態を確認し、必要なら同じremoteブランチをfetchして比較する。他worktreeを切り替えたりresetしたりしない。
2. [第1候補README](../../design/prototypes/stages-13-20-v1/README.md)と[全体設計](../../design/stage-design-final-proposal.md)を読む。
3. **Stage 13の代表ASCIIを2〜3配置表示して、入口→配置→乗り降り→出口の体験を確認する。** 以下のrenderで候補RONを直接指定する。既存の検証結果を全面再実行する必要はない。
4. 13から順に候補の構成を評価し、製品コードと実機確認へ進む。実機は稼働状態・実行物のsource SHA・RON・隔離セーブを確認してから試す。過去のA1X稼働状況や旧候補でのクリア記録を現在の候補へ流用しない。
5. 17はType1の待機余裕、18はType2の接触待ちを作る必要がある。新しい能力や秒単位の厳密同期を本編の必須条件にしない。
6. 実機で採用できる面を個別PRに切り出す際は、地形・説明・予算・カタログ・進行を揃え、`develop-and-review-pr`の実装と独立レビュー手順に従う。#82全体のマージはしない。
7. 1〜12の残る実機項目、20面の通しプレイ、20面クリア後のHTTP解放は未完タスクとして残す。

```bash
cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  render 13 --stage-file design/prototypes/stages-13-20-v1/stage-13.ron \
  --seed 0 --coordinates

cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  play 13 --stage-file design/prototypes/stages-13-20-v1/stage-13.ron --seed 0
```

対話CLIでは`show`、`s 0 place up`、`walk-goal`、`assert goal`、`reset`、`quit`。完全再生は同ディレクトリの`evidence/stage-13-seed-0.plan`を`simulate --plan ... --require-goal`で実行。全8面の再検証が必要なら`bash design/prototypes/stages-13-20-v1/verify.sh`（新しい出力ディレクトリを使う）。

## 永続先・保存状態

- sourceコミット: `bcb599b` main取り込み、`aed511c` 既存面・旧候補の検証、`0d57857` Stage 10解決・18割り当て診断、`15fcce5` チャンク付き13〜20候補。
- これらは`origin/feature/20-stage-release-handoff`へpush済み。remoteとローカルのsource HEAD一致を保存時に確認。
- 本handoffはこの開発ブランチへ単独コミットし、同じremote/refへpushする。handoffコミットIDとpush成功は最終応答で報告する。
- 作成開始時の未コミット・未追跡変更なし。必要な候補・plan・ASCII・要約は全てGit管理済み。`/tmp/placement-chunks-v1`等の全レポートや補助探索は一時物であり、再開に必須ではない。
- Sasara Hub: このリポジトリ開発タスクはGit内を永続先とした。このセッションではHubへの登録は行っていない。
