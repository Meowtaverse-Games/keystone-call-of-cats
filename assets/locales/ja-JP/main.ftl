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
stage-select-stage-header = ステージ{$number}
stage-select-state-ready = 開始可能
stage-select-state-locked = ロック中
stage-select-stage-description-ready = 熟練コーダー向けのスプリントレイアウト。
stage-select-stage-description-locked = 上層のキーストーンに到達してこのリミックスを解放しよう。
stage-select-play = プレイ >

stage-ui-back-to-title = タイトルに戻る
stage-ui-menu-load = ロード
stage-ui-menu-save = セーブ
stage-ui-menu-run = 実行
stage-ui-menu-stop = 停止
stage-ui-status-load = メニュー「ロード（F1）」を選択しました。
stage-ui-status-save = メニュー「セーブ（F2）」を選択しました。
stage-ui-status-run = メニュー「実行（F3）」を選択しました。
stage-ui-status-stop = メニュー「停止（F3）」を選択しました。
stage-ui-feedback-stopped = 実行を停止しました。
stage-ui-feedback-no-commands = 命令は返されませんでした。
stage-ui-feedback-commands = {$count}件の命令: {$summary}
stage-ui-commands-list = 命令: {$summary}
stage-ui-feedback-goal = ステージクリア！
stage-ui-feedback-advance = ステージ「{$stage}」へ進みます。
stage-ui-feedback-start = ステージ「{$stage}」が開始されました。
stage-ui-feedback-complete = 全てのステージをクリアしました！

stage-ui-tutorial-controls-hint = F1で例題ロード / F3で実行、矢印キーで移動、スペースでジャンプ。
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

stage-ui-error-empty-script = スクリプトが空です。
stage-ui-error-invalid-move-direction = move命令にはleft/top/right/downのいずれかを指定してください: {$direction}
stage-ui-error-invalid-sleep-duration = sleep命令の秒数は0以上である必要があります。
stage-ui-error-engine = スクリプト実行エラー: {$message}
stage-ui-error-unsupported-language = サポートされていないスクリプト言語です: {$language}

stage-ui-tutorial-stage1-title = プレイヤー: 第一階層の記録
stage-ui-tutorial-stage1-text =
    ここは、地上から遠く離れた地下の洞窟。
    あなたは、どこからか聞こえる「猫の鳴き声」を追って
    この暗い穴へと降りてきました。

    この世界では、石だけがプログラミングで動かせます。
    あなたは自分の体を動かしながら、
    石には「命令」を書いて動かしていきます。

    まずは自分の体を動かしてみましょう。

    左右移動ボタン … 左右に移動

    ジャンプボタン … 段差を飛び越える
    崩れた足場に気をつけながら、先へ進んでください。

    先へ進むには、石を動かして足場を作る必要があります。
    画面の「石のプログラム」を編集して、
    「右へ進む」命令を並べてから「実行ボタン」で動かしてみましょう。
    うまくいけば、石が道を作ってくれます。

stage-ui-tutorial-stage2-title = プレイヤー: 第二階層の記録
stage-ui-tutorial-stage2-text =
    洞窟の奥へ進むと、同じような小さな谷が
    いくつも続いている場所に出ました。
    このままでは足場が足りません。

    石を何度も「右へ1マス進ませる」ことで、
    谷に橋のような足場を作ることができます。
    今度は、同じ動きをいくつも並べて書くことを意識してみましょう。

    石のプログラムを開いて、
    「右へ進む」命令を何度か続けて書いてください。
    そのあと、実行ボタンを押すと
    石が命令の通りに、順番に動いていきます

    もし谷を越える前に石が止まってしまったら、
    命令の回数が足りていないのかもしれません。
    もう一度プログラムを開き、
    命令の数を変えて、何度か試してみましょう。

stage-ui-tutorial-stage3-title = プレイヤー: 第三階層の記録
stage-ui-tutorial-stage3-text =
    洞窟の奥に、重たい石が乗ると開く
    不思議な扉を見つけました。
    しかし、石を動かしすぎると扉はまた閉じてしまうようです。

    今度は、石を「ちょうどいい位置」で止めることが大切です。
    壁にぶつかったら止める、
    落ちそうになったら止める、など
    状況を考えながら命令を書くイメージを持ってみてください。

    まずは、これまでと同じように石のプログラムを直して、
    扉の近くまで石を運んでみましょう。

    石が行き過ぎる → 命令を減らす

    届かない → 命令を増やす
    というように、少しずつ調整してみてください。

    プログラムは、一度で正しく動かなくてもかまいません。
    うまくいかなかったら、「どこで思っていた動きと違ったか」を探して、
    その部分の命令を直して、もう一度実行してみましょう。
    それが、デバッグという大事な作業です。

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
