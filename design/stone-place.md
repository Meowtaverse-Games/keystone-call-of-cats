# 石の Place 機能

Type 4 の石は `move`、`is_touched`、`is_empty`、`place` を使える。`dig` と `sleep` は使えない。

`place <direction>`（Keystone）または `place("<direction>");`（Rhai）は、石の隣のグリッドセルに固体ブロックを置く。ステージ RON の `place_limit` はステージ全体で共有する成功回数で、未指定なら互換性のため無制限、`0` なら配置できない。拒否された配置は回数を消費しない。

配置先は外周、ゴール、既存の terrain、プレイヤー、石、既に置いたブロックと重なる場合に拒否する。判定は配置時点の collider を使い、同じフレームの複数石の要求は StoneIndex の昇順で一つずつ決定する。

配置したブロックは通常 terrain と同じ static collider を持つ。石の移動・`is_empty`・将来その能力を持つ石の `dig` が同じ collider/tile 経路で扱う。F3 の停止または reset で全配置ブロックを消し、共有回数を初期値に戻す。

この機能は既存マップや解法を変更しない。Windows CI の ECS テストは命令、制限、競合、reset の状態を検証する。プレイヤーが実際に上に立てること、操作感、既存マップでの利用は A1X 実機確認で別途検証する。
