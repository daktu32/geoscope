# GeoScope - Progress

## Overview
GFD (地球流体力学) データ可視化デスクトップアプリケーション。

## Current Status: v0.1a Tech PoC (Skeleton Complete)

### Done
- [x] 既存ツール調査と課題分析 (Panoply, GrADS, ParaView, ncview 等)
- [x] PRD v0.2.0 作成 (`docs/PRD.md`)
  - ペルソナ定義 (院生/D2/准教授)
  - Progressive Disclosure (L0-L3) 設計
  - 3-stage fallback 変数推定
  - GUI Master モードのコード同期
  - レスポンシブレイアウト規約
  - MVP スコープ分割 (v0.1a/v0.1b/v0.2/v0.3)
- [x] 専門家パネル設計レビュー (`docs/DESIGN_REVIEW.md`)
  - テックスタック評価
  - UX デザイン改善提案
  - GFD ドメインギャップ分析
  - 20 件の具体的修正提案 (M1-M20)
- [x] レビュー結果の PRD 反映 (16+ 編集)
- [x] Pencil モックアップ作成 (9 画面)
  - Level 0: Welcome (ファイルドロップ)
  - Level 1: Basic View (地球儀 + サイドバー)
  - Level 2: Explorer (全パネル表示)
  - Level 3: Code Mode (コードエディタ)
  - A1: Onboarding Overlay (3 ヒントカード)
  - A2: Hovmoller View (時間-経度ヒートマップ)
  - A3: Spectrum View (E(n) log-log プロット)
  - A4: Fullscreen Mode (HUD オーバーレイ)
  - A5: Variable Label (変数メタデータ表示)
- [x] ペルソナ検証 (`docs/MOCKUP_GUIDE.md`)
- [x] PRD v0.3.0 — LLM Copilot 機能追加
  - チャットサイドバー (Code Panel とタブ同居)
  - コマンドパレット (Cmd+K)
  - ハイブリッド LLM (クラウド API + ローカル LLM)
  - コンテキスト階層設計 (Layer 0-3, メタデータのみ送信)
  - GFD ドメイン知識 (スペクトル法、典型パターン辞書)
  - 出力経路: LLM → Rhai コード → ユーザー確認 → 実行
  - v0.2 スコープ: Explain + Explore モード
  - v0.3 スコープ: Suggest モード + コンテキストメニュー + インライン注釈
- [x] LLM Copilot モックアップ (A6 Chat + A7 Command Palette)
- [x] README 作成 + スクリーンショット11枚埋め込み
- [x] GitHub リポジトリ作成 (private): https://github.com/daktu32/geoscope
- [x] ロゴデザイン (案B: Scope 強調型を採用)
- [x] GitHub Issues 登録 (#1-#6)
- [x] **v0.1a 技術 PoC — スケルトン実装完了**
  - Cargo プロジェクト初期化 (eframe 0.33 + wgpu 27 + egui_dock 0.18 + netcdf 0.12)
  - egui_dock 3カラムレイアウト (Data Browser / Viewport / Inspector)
  - wgpu 球面レンダラー (UV sphere mesh, WGSL shader, カメラ回転・ズーム)
  - カラーマップ LUT (viridis 256-entry, RdBu_r 準備済み)
  - データテクスチャ (R32Float, GPU アップロード)
  - NetCDF 読み込み (netcdf 0.12, 変数メタデータ, 2D スライス抽出)
  - D&D ファイル読み込み (.nc/.nc4/.netcdf フィルタリング)
  - Data Browser (変数ツリー, ツールチップ, ダブルクリックでロード)
  - Inspector (変数情報, カラーマップ ComboBox, 値範囲)
  - タイムスライダー (time 次元自動検出)
  - ビュータブ (Globe / Map 切替 UI)
  - ステータスバー (変数名, サイズ, 値範囲)
  - 時間/鉛直次元の自動検出 (time/t, level/lev/z/sigma)
  - 座標値の読み取り (lon/lat)
  - 変数推論の基礎 (infer_colormap: PRD 4.1 準拠)
  - Data Browser → GPU 自動アップロード統合
  - アプリ起動確認 (macOS Metal バックエンド)

### Next Steps (v0.1a 残タスク)
- [ ] サンプル NetCDF ファイルで Globe 描画の実地テスト
- [ ] タイムスライダー操作でデータ再ロード → GPU アップロード連動
- [ ] カラーマップ切替 (Inspector の ComboBox → GPU LUT 差替)
- [ ] ピボット判断: egui + wgpu の統合品質評価

### Next Steps (デザイン)
- [ ] ロゴの最終調整・エクスポート (SVG/PNG) — Issue #1
- [ ] README にロゴ埋め込み — Issue #2
- [ ] エクスポート UI ダイアログのモックアップ — Issue #3
- [ ] ベクトル場オーバーレイの Inspector UI — Issue #4

### Next Steps (並行)
- [ ] gtool-rs の CF standard_name 対応改善 — Issue #6

## Architecture
```
geoscope/
├── .cargo/config.toml        # HDF5_DIR 設定
├── Cargo.toml                # eframe 0.33 + wgpu 27 + egui_dock 0.18 + netcdf 0.12
├── docs/
│   ├── PRD.md                # プロダクト要求定義書 v0.3.0
│   ├── DESIGN_REVIEW.md      # 専門家パネルレビュー
│   └── MOCKUP_GUIDE.md       # モックアップ構成 + ペルソナ検証
├── PROGRESS.md               # このファイル
└── src/
    ├── main.rs               # エントリポイント (D&D有効, 1280x800)
    ├── app.rs                # GeoScopeApp (egui_dock 3カラム, D&D, GPU統合)
    ├── data/mod.rs           # DataStore, NetCDF読み込み, 変数推論
    ├── renderer/mod.rs       # GlobeRenderer (wgpu, UV sphere, WGSL shader)
    └── ui/mod.rs             # TabViewer (DataBrowser, Viewport, Inspector)
```

## Tech Stack
| Layer | Choice | Version |
|-------|--------|---------|
| GUI | eframe + egui_dock | 0.33 / 0.18 |
| 3D | wgpu (via eframe) | 27 |
| NetCDF | netcdf-rs | 0.12 |
| HDF5 | hdf5@1.10 (Homebrew) | 1.10.11 |
