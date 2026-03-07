# GeoScope - Progress

## Overview
GFD (地球流体力学) データ可視化デスクトップアプリケーション。

## Current Status: v0.2 P1 — GFD 研究者必須機能 (Done)

### Done (設計・準備)
- [x] 既存ツール調査と課題分析 (Panoply, GrADS, ParaView, ncview 等)
- [x] PRD v0.3.0 作成 (`docs/PRD.md`)
- [x] 専門家パネル設計レビュー (`docs/DESIGN_REVIEW.md`)
- [x] Pencil モックアップ作成 (11 画面: L0-L3, A1-A7)
- [x] ペルソナ検証 (`docs/MOCKUP_GUIDE.md`)
- [x] GitHub リポジトリ作成 + Issues 登録 (#1-#6)
- [x] ロゴデザイン (案B: Scope 強調型)

### Done (v0.1a — 技術 PoC)
- [x] Cargo プロジェクト初期化 (eframe 0.33 + wgpu 27 + egui_dock 0.18 + netcdf 0.12)
- [x] egui_dock 3カラムレイアウト (Data Browser / Viewport / Inspector)
- [x] wgpu 球面レンダラー (UV sphere, WGSL shader, 正射影, カメラ回転・ズーム)
- [x] 30° 経緯線 (グラティキュール) 描画
- [x] カラーマップ LUT (viridis / RdBu_r, GPU テクスチャ差替)
- [x] データテクスチャ (R32Float, GPU アップロード)
- [x] NetCDF 読み込み (netcdf 0.12, 変数メタデータ, 2D スライス, 時間/鉛直スライス)
- [x] D&D ファイル読み込み + CLI 引数対応
- [x] Data Browser (変数ツリー, シングルクリック選択, 次元ラベル)
- [x] Inspector (変数情報, カラーマップ ComboBox + グラデーションプレビュー, 値範囲)
- [x] タイムスライダー (time 次元自動検出, データ再ロード → GPU 連動)
- [x] ステータスバー (変数名, サイズ, 値範囲)
- [x] ダークテーマ UI (モック準拠, カスタム Visuals + egui_dock Style)
- [x] トップバー (アプリ名 + アクティブファイル名)

### Done (ピボット判断)
- [x] **egui + wgpu で続行を決定** (Tauri ピボットしない)
  - Inspector / Data Browser: 問題なく実装可能 → PRD 判断基準クリア
  - パフォーマンス: 128×64×1001 データで 60fps、GPU アップロード瞬時
  - テーマ: Visuals カスタマイズでモック準拠のダーク UI 実現
  - Tauri ピボットのコスト (JS/TS 移行, ビルド変更) に見合わない
  - wgpu の WebAssembly 対応パスは維持される

### Done (v0.1b — 使える最小形)

目標: 「Panoply より快適に dcmodel 出力を見られる」

- [x] renderer リファクタリング (common.rs / globe.rs 分離, mod.rs ハブ化)
- [x] Map ビュー (Equirectangular 投影, wgpu, 128x64 grid, pan+zoom)
- [x] 自動推論強化 (3段階フォールバック: standard_name → 名前パターン → データ特性)
- [x] ホフメラー図 (時間-経度ヒートマップ, egui 2D 描画)
- [x] E(n) エネルギースペクトル図 (log-log, egui painter)
- [x] PNG エクスポート (image crate, ファイルダイアログ)
- [x] Inspector 推論結果表示 + Export ボタン
- [x] ViewMode 4種切替 (Globe / Map / Hovmoller / E(n))
- [x] 時間再生アニメーション (play/pause, 速度 1-60fps)
- [x] UI デザイン調整 (パネル幅, タイムスライダー配置, Globe 余白)
- [x] カラーバー高解像度化 (64頂点 Mesh, LUT ベース滑らかグラデーション)
- [x] Grid/Smooth 表示切替 (シェーダー内バイリニア補間)
- [x] 経緯線の細線化 (0.6→0.25度幅, 控えめなブレンド)
- [x] ビューポート背景演出 (ラジアルビネット, 宇宙風グラデーション)
- [x] Globe 大気グローエフェクト (リムライト + ハロー, ティール系)
- [x] Globe 上下左右パディング (5%, 最低8px)
- [x] ~~ベクトル場 (矢印) 重ね合わせ → v0.2 送り~~ → v0.2 P1 で実装
- [x] ~~Mollweide 投影 → v0.2 送り~~ → v0.2 P1 で実装

### Done (v0.2 P1 — GFD 研究者必須機能)

目標: Mollweide 投影 + 鉛直断面 + ベクトル場オーバーレイ

- [x] Mollweide 投影 (Newton 法 θ 求解, メッシュ再生成, Inspector ComboBox 切替)
- [x] 鉛直断面ビュー (level × lat/lon ヒートマップ, Hovmoller パターン踏襲)
  - Fix Lat / Fix Lon 切替 + インデックス slider
  - ViewMode::CrossSection + "Section" タブ
- [x] ベクトル場オーバーレイ (CPU egui painter 矢印描画)
  - Globe / Map 両対応 (球面接線ベクトル → view_proj → スクリーン座標)
  - u/v ペア自動検出 (L1: standard_name, L2: name pattern)
  - Inspector: On/Off, u/v 変数選択, Density/Scale slider
  - Rust row-major → WGSL column-major 転置対応
- [x] データ構造追加 (CrossSectionData, VectorFieldData, CrossSectionAxis)
- [x] DataStore 拡張 (load_cross_section, load_vector_field)
- [x] detect_wind_pair (inference.rs)

### Next (v0.2 P2)

### Next (デザイン)
- [ ] ロゴの最終調整・エクスポート (SVG/PNG) — Issue #1
- [ ] README にロゴ埋め込み — Issue #2
- [ ] エクスポート UI ダイアログのモックアップ — Issue #3
- [x] ~~ベクトル場オーバーレイの Inspector UI — Issue #4~~ → v0.2 P1 で実装

### Next (並行)
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
    ├── app.rs                # GeoScopeApp (ダークテーマ, egui_dock 3カラム, D&D, GPU統合)
    ├── data/
    │   ├── mod.rs            # DataStore, NetCDF読み込み, Hovmoller データ読み込み
    │   └── inference.rs      # 3段階変数推論エンジン (L1 CF, L2 name, L3 stats)
    ├── renderer/
    │   ├── mod.rs            # Hub (モジュール re-export)
    │   ├── common.rs         # 共通型 (Vertex, CameraUniform, カラーマップ LUT)
    │   ├── globe.rs          # Globe レンダラー (wgpu, UV sphere, WGSL)
    │   ├── map.rs            # Map レンダラー (wgpu, Equirectangular, pan+zoom)
    │   ├── hovmoller.rs      # Hovmoller 図 (egui 2D, ColorImage)
    │   ├── spectrum.rs       # E(n) スペクトル (egui painter, log-log)
    │   └── export.rs         # PNG エクスポート (image crate)
    └── ui/mod.rs             # TabViewer (DataBrowser, Viewport, Inspector)
```

## Tech Stack
| Layer | Choice | Version |
|-------|--------|---------|
| GUI | eframe + egui_dock | 0.33 / 0.18 |
| 3D | wgpu (via eframe) | 27 |
| NetCDF | netcdf-rs | 0.12 |
| PNG | image | 0.25 |
| File Dialog | rfd | 0.15 |
| HDF5 | hdf5@1.10 (Homebrew) | 1.10.11 |
