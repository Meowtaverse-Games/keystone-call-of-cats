# セッション引き継ぎ: 軽量CLIとAIステージ生成・検証

## 再開後のステージ確認

- main `43a8df9`のStage 1〜4を`/tmp/keystone-main-ascii-43a8df9/`へ抽出し、CLI seed 0で生成したASCIIをユーザーへ表示した。
- ユーザーは「stage 4まではこのままでいいかな」と評価。Stage 1〜4はmain版を維持し、再設計・フレーバー追加を必須にしない方針を[最終提案](../../design/stage-design-final-proposal.md)へ記録した。
- 次に構成を確認する対象はmainのStage 5以降。Stage 1〜4の実機クリア・全seed成立を確認済みとはしない。統合ブランチのassetsは旧固定候補のままであり、main版へ置換したと誤認しない。

## 対象

- リポジトリ / worktree / Git toplevel: `/home/ubuntu/repos/meowtaverse-games/keystone_cc`
- ブランチ / source code HEAD: `feature/20-stage-release-handoff` / `80024697cde0e01ba3527f2b163c8a0939da6634`
- worktree一覧: 対象のほか外部制御・言語pin・Rhai待機・stage_sim・Stage 8試作・能力制限・place等の専用worktreeを確認。別worktreeは変更していない。
- タスク: Bevyを起動せずCLIでプレイ・石操作・ゴール判定・操作再生を行い、AI生成の候補を多数のランダムチャンク配置で高速に検証する。
- 記録日時: `2026-09-28T20:13:33+09:00`

## 現在地

- 結論: 既存`tools/stage_sim`を拡張し、操作記録、実到達を要求する再生、player-onlyの経路探索と実行、多seedのcheck、AI向けJSON、失敗seedのRON・手順・地形の保存を実装した。
- AI生成の実証: `tools/stage_sim/examples/ai-candidate.*`。Type4×1、配置上限1、共通の石操作でseed 0〜99の100件成功。実際の地形は12種類で、全件初期到達不可・操作後の実到達・無効操作なし。開発ビルドで654ms（コンパイルを除く）。配置上限0の負の対照は3件全て失敗を検出した。
- 根拠: 独立ツールのテスト16件、clippy全target（警告エラー）、fmt、diff check成功。旧候補1〜16の検証スクリプト、17・19のコードとplayer plan、20の配置plan、製品カタログ検証も成功。Stage 18は従来どおり`player plan ended before reaching the goal`で失敗し、地形・解法・assertは変更していない。
- 追加の整合: 手動dig/placeの石能力制限、RONのplace_limitとCLI上書き、配置時の境界、探索と実操作の共通遷移、探索予算、RONの未知フィールド拒否を追加した。
- 未解決の不確実性: グリッドモデルでありBevyの連続物理・時間・乗車感・人の面白さを検証していない。障害物Oの時間消滅、動的地形の抽選、小数位置補正はモデル外。CLI seedは製品seedではない。
- 言語境界: ツールのkeystone-langは既存lockの`057ca26c`のまま。製品の言語版と同一ではなく、`run`はRhaiやplaceイベントに未対応。新しいcheckは逐次操作planの検証であり、製品コードの同時実行検証ではない。
- 未コミット変更: コードcommit後は、このhandoffの作成以外になし。ローカル証跡は`/tmp/keystone-ai-candidate-check-v1/report.json`、`/tmp/keystone-ai-candidate-negative-v1/report.json`、`/tmp/keystone-stage-lab-regression-{17,18,19,20}.log`。tmpは永続保証がない。候補・手順・測定要約はリポジトリへ保存済み。
- コードの既存コミット: `80024697cde0e01ba3527f2b163c8a0939da6634`

## 続行範囲

- ユーザーはブラウザ画面より軽量CLIを先に使うことを選択。AIでの候補生成とチェックも依頼した。
- 設計方針は[全20面の最終提案](../../design/stage-design-final-proposal.md)を維持。mainの完成面は原則継承、ランダムチャンクはフレーバー、HTTPは20面クリア後の解放案。実験用候補を製品Stage 13へ自動採用しない。
- 実施済み: CLIとAI生成ループの基盤実装・検証・実証。Bevy本体をビルドしていない。製品assets・製品コード・ロケールを変更していない。
- 次の正確な一手: Stage 1からの確認へ戻る場合、main `43a8df9`の対象RONを別ディレクトリへ取り出して`--stage-file`または`--stages-dir`で指定する。現在ブランチのassetsをmainと取り違えない。まずrender/play、操作不要のStage 1は`check --walk-to-goal`を使い、`--require-initially-unreachable`を付けない。失敗はモデル不足・生成問題・操作計画不足を分け、完成面の全面改造へ直結させない。

## 使用例

```bash
cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  play 13 --stage-file tools/stage_sim/examples/ai-candidate.ron
```

`s 0 place up`、`route`、`walk-goal`、`assert goal`、`reset`で試せる。

```bash
cargo run --quiet --locked --manifest-path tools/stage_sim/Cargo.toml -- \
  check 13 --stage-file tools/stage_sim/examples/ai-candidate.ron \
  --plan tools/stage_sim/examples/ai-candidate.plan \
  --walk-to-goal --require-initially-unreachable --reject-blocked \
  --seeds 100 --output /tmp/ai-stage-next-check
```

出力先は新しいディレクトリ。既存の証跡は上書きしない。

## 永続先

- 操作説明: [README](../../tools/stage_sim/README.md)、[AI生成の作業契約](../../tools/stage_sim/AI_WORKFLOW.md)、[実証結果](../../tools/stage_sim/examples/ai-candidate.md)。
- handoffコミットとpush: このファイルだけを別commitにし、既存の`origin/feature/20-stage-release-handoff`へ実装commitとともにpushする。成功状態は実行結果で報告する。
- PR: 既存Draft #82のブランチ上。新規PR、merge、製品への採用、CIの最新結果確認、独立エージェントレビューはこの作業では実施していない。
- Sasara Hub: リポジトリ内開発作業としてファイルへ記録。Hubへの登録は行っていない。
