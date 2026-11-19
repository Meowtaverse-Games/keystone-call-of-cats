# PR #53 terrain reference

このディレクトリは、旧 [PR #53](https://github.com/Meowtaverse-Games/keystone-call-of-cats/pull/53) のhead [`928f9fd9852e5cb22e3f4241dbb03ac4abb93ff6`](https://github.com/Meowtaverse-Games/keystone-call-of-cats/commit/928f9fd9852e5cb22e3f4241dbb03ac4abb93ff6) から、地形資料だけをbyte-identicalに保存した参照です。

- [`stage-13.ron`](https://github.com/Meowtaverse-Games/keystone-call-of-cats/blob/928f9fd9852e5cb22e3f4241dbb03ac4abb93ff6/assets/stages/stage-13.ron) と [`stage-14.ron`](https://github.com/Meowtaverse-Games/keystone-call-of-cats/blob/928f9fd9852e5cb22e3f4241dbb03ac4abb93ff6/assets/stages/stage-14.ron) は同じblobで、Type4の28×18固定`start-1`マップ、`mid-none`、`goal-noop`から成る。
- [`stage-22.ron`](https://github.com/Meowtaverse-Games/keystone-call-of-cats/blob/928f9fd9852e5cb22e3f4241dbb03ac4abb93ff6/assets/stages/stage-22.ron) はType5の26×10マップで、開始chunk、11個のmiddle chunk、2個のgoal chunkを持つ。

現行の候補正本は`design/stages/`と`design/solutions/`のStage 1〜20である。この参照は候補へ採用した地形ではなく、比較や将来の設計検討に使う資料である。特に旧Stage 22のType5は現行製品で未対応のため、参照専用とする。採用する場合は、現行の能力、固定マップ、解法、製品実機確認へ適応する設計作業が必要である。

ここにあるRONは`assets/stages/`へ登録・配置しておらず、製品カタログからロードされない。Runtime機能はこの保存で追加しない。石種ごとの能力制限、Type4の`place`、HTTP外部入力はそれぞれmainへマージ済みのPR #83、PR #84、PR #85に属し、この参照のType5を実装するものではない。
