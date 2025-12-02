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
stage-select-stage-header = ステージ{$number}
stage-select-state-ready = 開始可能
stage-select-state-locked = ロック中
stage-select-stage-description-ready = 熟練コーダー向けのスプリントレイアウト。
stage-select-stage-description-locked = 上層のキーストーンに到達してこのリミックスを解放しよう。
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

stage-ui-tutorial-stage1-title = チュートリアル1
stage-ui-tutorial-stage1-text =
    ここは、ある森の中にある地下の洞窟。
    あなたは、どこからかともなく聞こえる、かすかな猫の呼び声を追ってここへ導かれたどり着きました。

    自分の体を動かし、下の階層へ進みましょう。
    ____
    左右移動ボタン … 左右に移動
    ジャンプボタン … 段差を飛び越える
    崩れた足場に気をつけながら、先へ進んでください。

    F3(実行)キーを押すとスタートです。

stage-ui-tutorial-stage2-title = チュートリアル2
stage-ui-tutorial-stage2-text =
    地下へ進むと、途中で大きな石にふさがれていました。
    このままでは、あなたは先へ進めません。

    けれど、この世界のあなたは、「特殊な石」に対して「どう動くか」を伝え、
    自分の道を切り開くことができます。

    左画面で、「どう動くか」の命令を書くことができます。
    「右へ進む」命令を書いてください。
    どうやって書けば良いか？は、F4(説明)キーを押すと分かります。
    そのあと、F3(実行)キーを押し、石を命令の通りに、動かしてみましょう。

    思うように石が動かなければ、説明をしっかり読んで何度も試してみましょう。

stage-ui-tutorial-stage3-title = チュートリアル3
stage-ui-tutorial-stage3-text =
    底の見えない大きな穴に行く手をさえぎられました。

    この石の上に乗れば、向こう側まで運んでもらえるかもしれません。
    石は、右へ進む命令を続けて書くことができます。

    石の上に乗った状態で実行すると、石と一緒にあなたの体も運ばれていきます。
    穴を越えて反対側の足場まで届くように、move を必要な回数だけ並べてみましょう。

stage-name-1 = チュートリアル1
stage-name-2 = チュートリアル2
stage-name-3 = チュートリアル3
stage-name-4 = ステージ1
stage-name-5 = ステージ2
stage-name-6 = ステージ3
stage-name-7 = ステージ4
stage-name-8 = ステージ5
stage-name-9 = ステージ6
stage-name-10 = ステージ7
stage-name-11 = ステージ8
stage-name-12 = ステージ9
stage-name-13 = ステージ10
stage-name-14 = ステージ11
stage-name-15 = ステージ12
stage-name-16 = ステージ13
stage-name-17 = ステージ14
stage-name-18 = ステージ15
stage-name-19 = ステージ16
stage-name-20 = ステージ17
stage-name-21 = ステージ18
stage-name-22 = ステージ19
stage-name-23 = ステージ20
