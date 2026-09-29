# Stage 13: レビュー済み試遊ビルド

20面対応の続行記録。Stage 13のビルド準備は完了し、次は実機試遊。全体方針は [20面handoff](2026-09-29-twenty-stage-candidates-and-validation.md) を継承する。

## 対象と保存場所

- 記録日: 2026-09-29（JST）。
- 統合worktree: `/home/ubuntu/repos/meowtaverse-games/keystone_cc`、branch `feature/20-stage-release-handoff`、記録前HEAD `cd59e1672db26b201783b6dcc7631d61d1e0f687`。本記録を保存する場所。製品コードは変更していない。
- Stage 13専用worktree: `/tmp/keystone-stage13-playtest`、branch `feature/stage13-playtest`。
- base: `252efbea156f674b6be819656a676592fff55eaf`。
- 実装head: `69530d9f83103432319b5020ef39a449c2badb27`。
- [Draft PR #87](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/87)。未マージ。統合Draft #82は一括採用しない。
- worktree一覧を確認済み。他のASCII、外部制御、言語pin、接触待ち、Stage8等のworktreeは変更していない。

## 実施済み

- 最新mainからStage13だけを抽出。候補RONを完全一致で `assets/stages/stage-13.ron` に登録し、カタログ・リスト・3言語説明を揃えた。1〜12の地形は不変。
- Type4×1、共有配置上限1、12→13と13終端を確認。製品用サンプルはRhai `place("up");`、Keystone `place up`。
- CLI: 100 seedすべてで初期到達不可・配置後の実到達・blocked 0、75地形。地形変更なしのため、説明修正時には再実行していない。
- コミット順: `6b1d4c0` Stage13登録、`d84b5f8` 説明の構文誤表示修正、`69530d9` テストのCRLF対応。全てremoteへpush済み。
- [lint run 36550431178](https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/36550431178) 成功。
- [Windows verification run 36550431386](https://github.com/Meowtaverse-Games/keystone-call-of-cats/actions/runs/36550431386) ビルド成功、本体テスト73件成功・失敗0。
- 独立Astra再レビュー: **approved**。対象base/headは上記と一致。3言語の両言語ラベルに同一変数を渡す誤表示を修正済み。途中CIのLF専用テスト失敗はCRLF対応で解消した。
- この承認は試遊候補PRのコードレビューであり、実機の合格ではない。

## 正しい試遊用実行物

- GitHub artifact名: `windows-verification-a209adaa3b2197f17b4905291ce32c693b10da3d`（上記成功run）。
- 取得先: `/tmp/keystone-stage13-artifact-36550431386`。一時ファイルなので消失時は同runから再取得する。
- exe: `ci-results/keystone-cc.exe`。
- exe SHA-256: `f7945473953ea1ed37328d61c4488b52f4e4c30abafd25bc2c42b87d56834d26`。
- metadata source SHA: `a209adaa3b2197f17b4905291ce32c693b10da3d`（GitHub合成merge）。両親が上記base/headであること、レビューheadとtree `725b050dd50c2c202d299671957849b2af7187a8` が一致することをAPIで確認済み。
- 候補ソースRON SHA-256: `2bb8a8f9417067f1e706040b6c5abfd231b4125550d24ba818fa43ee1af42899`。
- artifact内RONの生バイトSHA-256: `5dd951eb6b6477f53716b663ed3a5d4638fa5e4a20e08b99ccab35dc6662646c`。WindowsのCRLFをLFへ正規化すると候補ソースと完全一致。生バイト一致と記載しない。
- 最初の成功run `36548884639` は説明修正前。`36549780516` はテスト失敗。どちらも最終試遊用に選ばない。

## 未実施と次の正確な一手

1. A1X workerの接続を再確認する。今回の確認では最終接続は9月28日12:16:40 JST、約30時間経過し、queued/runningジョブなし。ユーザーへ起動・接続の必要を伝えている。オフラインのままジョブは投入していない。
2. 成功runのartifactを専用ディレクトリへ配置し、metadata・exeハッシュを再確認する。`KEYSTONE_CI_SAVE_DIR` に新しい隔離セーブを指定し、`--ci-smoke --ci-smoke-report <専用パス> --stage-id 13` で起動する。既存の実行中ゲームがあれば無断で終了させない。
3. [試遊計画](../../design/prototypes/stages-13-20-v1/PLAYTEST.md) とPR本文に従い、足場配置→乗降→高台→ゴール、停止／リセット→配置物消去／予算復元→再挑戦を確認する。実際の生成配置・実行コード・ログ・画面を記録する。CLI seedと製品seedを同一視しない。
4. ユーザーによる初見の理解・操作感の評価を得る。必要な修正を実施してから採用判断へ進む。その後に14以降を順次準備する。

今回のユーザー指示は、試遊準備と検証を順に進めること。マージ・公開配布・本番リリースは未実施。A1Xでの起動・ゲームプレイも未実施。17/18の製品コード、その他の面の実機受入、20面通しプレイ、HTTP解放実装は未完のまま。

本記録は統合ブランチに単独でコミット・pushする。Stage13専用ブランチのレビュー済みheadは変更しない。Sasara Hubへの登録は行わずGitを永続先とする。
