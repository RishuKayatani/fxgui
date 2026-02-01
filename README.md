# fxgui

Rust + Tauri + React をベースに、`fxlib` を Core ライブラリとして利用するFXチャートアプリのGUIプロジェクトです。

## 概要
- デスクトップ向けFXチャートアプリ
- UI: React
- デスクトップ: Tauri
- Core: Rust ライブラリ `fxlib`

## リポジトリ構成（予定）
- `src/` : フロントエンド（React）
- `src-tauri/` : Tauriアプリ本体
- `core/` : `fxlib` 連携レイヤ

## 開発準備（予定）
- Rust / Node.js 環境を準備
- 依存関係をインストール
- `fxlib` をサブモジュールまたは依存として組み込み

## 計測（デバッグのみ）
- `npm run tauri dev` 実行中、以下の区間計測がコンソールに出力されます:
  - `dialog.open` / `dialog.normalize`
  - `ipc.ingest_csv` / `ipc.record_dataset_history`
  - `ipc.compute_indicators`
  - `ipc.resample_dataset`
  - `state.updatePane` / `state.applyTimeframe`
  - `render.chart`
- Rust側は `fxgui.log` に `ingest` / `resample` / `indicators` の計測が出ます（debugビルドのみ）

## 関連
- Core ライブラリ: `fxlib`（https://github.com/RishuKayatani/fxlib）

---

本READMEは初期ドラフトです。実装に合わせて更新してください。
