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

## 初回時点の未実施と次の一手（下記の実機追記で更新）

1. A1X workerの接続を再確認する。今回の確認では最終接続は9月28日12:16:40 JST、約30時間経過し、queued/runningジョブなし。ユーザーへ起動・接続の必要を伝えている。オフラインのままジョブは投入していない。
2. 成功runのartifactを専用ディレクトリへ配置し、metadata・exeハッシュを再確認する。`KEYSTONE_CI_SAVE_DIR` に新しい隔離セーブを指定し、`--ci-smoke --ci-smoke-report <専用パス> --stage-id 13` で起動する。既存の実行中ゲームがあれば無断で終了させない。
3. [試遊計画](../../design/prototypes/stages-13-20-v1/PLAYTEST.md) とPR本文に従い、足場配置→乗降→高台→ゴール、停止／リセット→配置物消去／予算復元→再挑戦を確認する。実際の生成配置・実行コード・ログ・画面を記録する。CLI seedと製品seedを同一視しない。
4. ユーザーによる初見の理解・操作感の評価を得る。必要な修正を実施してから採用判断へ進む。その後に14以降を順次準備する。

今回のユーザー指示は、試遊準備と検証を順に進めること。マージ・公開配布・本番リリースは未実施。A1Xでの起動・ゲームプレイも未実施。17/18の製品コード、その他の面の実機受入、20面通しプレイ、HTTP解放実装は未完のまま。

本記録は統合ブランチに単独でコミット・pushする。Stage13専用ブランチのレビュー済みheadは変更しない。Sasara Hubへの登録は行わずGitを永続先とする。


## A1X実機追記（2026-09-29 22:17 JST）

- A1X workerは復旧済み。現在のworker IDは `d8341468f81a48459b50b95836a892cb`。ユーザーはworker PowerShellを開いたままにしている。古いworkerのモジュール参照エラーは新規PowerShellでUtility/ArchiveをGlobal importして解消。セキュリティ機能の無効化は行っていない。
- 上記レビュー済みexeをA1Xへ配置し、exe・metadata・RONのハッシュを確認して起動済み。キャッシュは `%LOCALAPPDATA%\Sasara\WindowsWorker\20260929-214305-5cc9bf78f4f8\source\verified-build\payload`。毎ジョブで隔離セーブを新設している。
- 実機で `place("up");` をF3実行し、青いブロックが石の上に1個出現、残りブロック0を画面で確認。ログも `Place(Top)` を確認した。証拠job `20260929-221159-3ae47616e211` の `placed.png`。結果zip SHA-256 `f2bc8058fb0b2157a8e8edf0e04c7516f5b7b176317a897503cacb653768e963` を検証済み。
- 同ジョブで停止ログは得たが、停止後の撮影時に前面が外れたため、リセット・再配置の一連の検証は未完。ジョブ自体は失敗扱いで、成功とは扱わない。
- 最後の診断job `20260929-221559-25b6958652b8` ではアプリ切り替え画面が開いていることを確認。結果zip SHA-256 `39b1901767b0512f276a071f0016928ea751ec71d704afa7fbd8d1283f1a3f4b` を検証済み。前面確認に失敗したため入力を中止し、自分が起動したゲームだけを終了した。ユーザーへ操作中か確認中。次のジョブ投入前に回答を確認する。
- ゲームを開いたまま結果回収するとファイルロックでworkerのzip生成が止まった。今後は1ジョブ内で起動・操作・撮影・自身のゲーム終了・ログ回収を完結させる。実行中ログはoutput外のruntimeへ置く。この方式で失敗時も結果が自動回収できることを確認済み。
- 一時スクリプト: `/tmp/keystone-stage13-focus-diagnostic.ps1`（配置・停止・再配置、前面確認付き）、`/tmp/keystone-stage13-route-trial.ps1`（移動試行用、未実行）。前面以外へ入力しない制御を保持する。生ログ・画面はqueueと/tmpに保存しGitへ入れない。
- 次: 操作競合がない状態で配置→停止→再配置を完了させ、実際のキー入力で足場乗降・高台・ゴールを確認する。無配置でもジャンプで突破できるかは未確認であり、CLI到達不可だけで製品実機の到達不可とは断定しない。
- Stage13の実機合格、14以降の準備、20面通しの受入は未完。PR #87はDraft・未マージのまま。製品コードやレビュー済みheadは変更していない。


## A1Xでの再開（2026-10-05 JST）

- 以降の作業ホストはOracleサーバーからA1X（WSL2）へ移した。worktreeは `/home/ubuntu/repos/meowtaverse/keystone_cc`（本ブランチ）。上記の `/home/ubuntu/repos/meowtaverse-games/...` と `/tmp/...` はOracle上のパスで、A1Xには存在しない。
- 再開時に `origin/feature/20-stage-release-handoff` = `1d2abc0`、`origin/feature/stage13-playtest` = `69530d9`、`origin/main` = `252efbe` を確認。#87・#82はともにOPEN/Draft、CI成功のまま変化なし。
- 試遊はworkerのジョブ投入ではなく、A1Xで直接起動する。前面が外れる問題を避け、本人の初見評価もそのまま行うため。実行物は引き続きrun `36550431386` のartifactを使い、上記ハッシュと照合する。Oracleの `/tmp` にあったartifact・PowerShellスクリプトは再開に必須ではない。
- A1XにはWSL・Windowsのどちらにもまだ Rust toolchainがない。CLIのstage_simを使うときはWSLへrustupを入れる。採用判断に使うビルドはCIのartifactとする。
- Oracleに残る作業: remoteにブランチがない言語pin・Stage 8試作のworktreeについて、未push・未コミットの変更がないか一度確認する。Stage13の続行には関係しない。
