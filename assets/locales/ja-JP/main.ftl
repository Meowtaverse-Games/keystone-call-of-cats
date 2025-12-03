#game-title-full = キーストーン：コール・オブ・キャッツ
#game-title-inline = キーストーン・コール・オブ・キャッツ

stage-select-badge-experimental = エクスペリメンタル版
stage-select-stats-unlocked = 解放済
stage-select-stats-locked = ロック
stage-select-stats-slots = スロット
stage-select-featured-label = ピックアップステージ
stage-select-featured-description = 猫たちより先にキーストーン戦略を磨こう。
stage-select-highlight-placeholder = 近日公開
stage-select-highlight-pages = ページ {$current}/{$total}
stage-select-highlight-mode = ストーリーモード
stage-select-back = 終了
stage-select-options = オプション
stage-select-stage-header = #{$number}
stage-select-state-ready = 開始可能
stage-select-state-locked = ロック中
stage-select-play = プレイ >

options-title = オプション
options-volume-master = マスターボリューム
options-volume-sfx = 効果音ボリューム
options-volume-music = BGMボリューム
options-fullscreen-label = フルスクリーン起動
options-fullscreen-on = ON
options-fullscreen-off = OFF
options-language-label = スクリプト言語
options-language-rhai = Rhai
options-language-keystone = Keystone
options-button-back = 戻る

stage-ui-back-to-title = ステージ選択に戻る
stage-ui-menu-run = 実行
stage-ui-menu-stop = 停止
stage-ui-menu-font-decrease = -
stage-ui-menu-font-increase = +
stage-ui-status-command-help-open = メニュー「コマンド説明」を開きました。
stage-ui-status-command-help-close = メニュー「コマンド説明」を閉じました。
stage-ui-feedback-stopped = 実行を停止しました。
stage-ui-feedback-no-commands = 命令は返されませんでした。
stage-ui-feedback-commands = {$count}件の命令: {$summary}
stage-ui-commands-list = 命令: {$summary}
stage-ui-feedback-goal = ステージクリア！
stage-ui-feedback-advance = ステージ「{$stage}」へ進みます。
stage-ui-feedback-start = ステージ「{$stage}」が開始されました。
stage-ui-feedback-complete = 全てのステージをクリアしました！

stage-ui-tutorial-controls-hint = F1で実行、F2/F3で文字サイズ変更、矢印キーで移動、スペースでジャンプ。
stage-ui-tutorial-ok = 了解!
stage-ui-tutorial-next-hint = ［Enter］で次を表示
stage-ui-clear-window-title = ステージクリア!
stage-ui-clear-heading = ゴールに到達しました。
stage-ui-clear-body = 次の挑戦へ進む前に少し休憩しましょう。
stage-ui-clear-ok = OK

stage-ui-command-help-button = コマンド説明
stage-ui-command-help-title = コマンド説明
stage-ui-command-help-intro = このシーンで使える主な命令です。石を動かす作戦を考えるときに参照してください。
stage-ui-command-help-move-title = move(direction)
stage-ui-command-help-move-body = 石を指定した方向へ1マス動かします。left/right/up/top/down の文字列を渡してください。
stage-ui-command-help-sleep-title = sleep(seconds)
stage-ui-command-help-sleep-body = 指定した秒数だけ待ってから次の命令へ進みます。小数も使えます。
stage-ui-command-help-close = 閉じる

stage-ui-error-invalid-move-direction = move命令にはleft/top/right/downのいずれかを指定してください: {$direction}
stage-ui-error-invalid-sleep-duration = sleep命令の秒数は0以上である必要があります。
stage-ui-error-engine = スクリプト実行エラー: {$message}
stage-ui-error-unsupported-language = サポートされていないスクリプト言語です: {$language}
