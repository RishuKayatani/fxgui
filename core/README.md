# core

`core/` は `fxlib` を中核ライブラリとして取り込むための層です。

## 方針
- `fxlib` は submodule として `core/fxlib` に配置する。
- `fxgui` 側で UI/操作/状態管理を担い、`fxlib` は分析/指標計算/バックテスト用の処理を担当する。
- 直接依存する境界は `core/` 配下に集約し、UI層からの直接参照は避ける。

## 構成
- `core/fxlib/` : fxlib submodule
