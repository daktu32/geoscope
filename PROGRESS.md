# GeoScope - Progress

## Overview
GFD (地球流体力学) データ可視化デスクトップアプリケーション。

## Current Status: v0.3 P1+P2 — インタラクション強化 (Done)

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

### Done (v0.2 P2 — 操作性改善)

目標: Level 選択 + カラーマップレンジ制御

- [x] Level 選択スライダー (viewport 下部パネル, タイムスライダーと統一配置)
  - level 次元がある変数のみ表示
  - 全 load_field_at 呼び出しで level_index 反映 (時間再生, ベクトルオーバーレイ含む)
- [x] カラーマップレンジ制御 (3 モード: Slice / Global / Manual)
  - Slice: 表示中スライスの min/max で自動スケール
  - Global: NC ファイル全データ (全時刻・全レベル) の min/max でスケール (キャッシュ付き)
  - Manual: ユーザー指定 min/max (DragValue 入力)
  - Inspector に Slice / Global 両方の range 値を常時表示
- [x] upload_field_data_with_range (Globe/Map レンダラー, 明示的 min/max 指定)
- [x] compute_global_range (DataStore, 全値一括読み込み + キャッシュ)

- [x] UI ファイルオープン (Data Browser「Open」ボタン, rfd 複数選択対応)
- [x] マルチファイル切替 (変数クリックで active_file 切替, アクティブファイルハイライト)
- [x] カラーマップ刷新 (10種: Sequential 6 + Diverging 4, 9ストップ高精度化, カテゴリ付きUI)
- [x] ファイル閉じるボタン (Data Browser × ボタン, active_file 自動調整)
- [x] ロゴ SVG 作成 (Scope 強調型: ティール球体 + レンズ + 渦度双極子) — Issue #1
- [x] README.md 更新 (ロゴ埋込, ロードマップ・機能一覧を v0.2 P2 反映) — Issue #2
- [x] エクスポート UI ダイアログ (解像度 1x/2x/4x, カラーバー付き, タイトル入力) — Issue #3
- [x] パフォーマンス改善
  - LUT キャッシュ (全10種を起動時プリロード, 毎フレーム再生成を排除)
  - Vector overlay 座標キャッシュ (view 変更時のみ再計算)
- [x] UI 磨き込み
  - Inspector: セクションヘッダ統一 (teal+大文字), カラーバー min/max ラベル, 余白整理
  - Data Browser: × ボタンをヘッダ行内に統合
  - ステータスバー: 時刻・レベル情報追加 (t=3/1001, lev=0/26)
- [x] デザインシステム刷新 (モック準拠)
  - デザイントークン定義 (PRIMARY, BG_DARK/PANEL/WIDGET, TEXT_HEADING/BODY/SECONDARY/CAPTION, SP_*)
  - テーマ統一 (apply_theme でトークン適用, 12px Body, ボタン・ウィンドウシャドウ)
  - Data Browser: "+" ボタン, 変数ドットをタイプ別色分け (赤=diverging/緑=scalar/橙=velocity), 次元名表記 "(lon, lat, time)"
  - Inspector: Variable ComboBox (Inspector から変数切替), Projection 常時表示, Colormap 説明ラベル
  - Range UI 刷新: min/max 常時表示 + "to" セパレータ, Scale ボタン化, Symmetric (0-centered) チェックボックス
  - Tab bar: ピル型ボタン + 🌐 Globe アイコン, BG_DARK 背景
  - Status bar: 推論ベース表示 "💡 Detected: vorticity (RdBu_r, diverging, 0-centered)"
  - Time slider: max 値表示, Grid/Smooth ボタン化
  - Top bar: "/" セパレータ
  - Bug fix: Data Browser 複数ファイル展開時の重なり (allocate_rect → interact)
  - Level 次元検出拡張 (sig, depth, height, plev, pressure, eta, hybrid + 部分一致)

### Done (v0.3 P1 — インタラクション強化)

目標: 「触って理解できる」— Direct Manipulation の完成

- [x] Point Info (Map ホバーで緯度・経度・値をビューポート左下に表示)
  - Screen → NDC → UV → lat/lon 逆変換 (Equirectangular)
  - 最近傍格子点の値ルックアップ
  - 半透明ダーク背景ピル表示
- [x] キーボードショートカット
  - Space: 再生/停止, ←→: 時間ステップ, ↑↓: レベルステップ
  - 1-6: ビュー切替 (Globe/Map/Hovmoller/Spectrum/Profile/Section)
  - G: Grid/Smooth, C: 等値線, V: ストリームライン
- [x] Profile ビュー (鉛直プロファイル / 時系列折れ線グラフ)
  - 軸ティックマーク (X/Y 軸, 自動フォーマット: 科学的記法)
  - 薄いグリッド線, データ点マーカー (半径 2.0)
  - ホバークロスヘア + 最近傍点ツールチップ
  - タイトル (変数名 + 座標位置)
  - DataStore: load_profile_data (鉛直), load_time_series_data (時系列)

### Done (v0.3 P2 — 高度な可視化)

- [x] Polar Stereographic 投影
  - PolarNorth / PolarSouth 2 モード
  - 円盤メッシュ (ポール中心 + ラジアルリング, 三角形ファン + クワッドストリップ)
  - UV マッピング: 赤道投影と一致
- [x] ストリームライン描画
  - RK4 積分 (Euler → 4次ルンゲクッタ)
  - バイリニア補間 (最近傍 → 4点補間, 経度方向周期境界対応)
  - 矢じり描画 (20 点間隔, 4.5px)
  - 適応ステップ長 (速度場に応じた dt スケーリング)
- [x] 等値線 (コンター) オーバーレイ
  - Marching Squares アルゴリズム (UV 空間線分抽出)
  - 主等値線 / 副等値線 (5 レベルごとに太線 1.2px, 他 0.6px)
  - コンターラベル (主レベル, 水平部分に配置, 100px 間隔)
  - Globe 投影対応 (正射影 + 裏面カリング)
  - Inspector: Enabled チェックボックス + Levels スライダー
- [x] Zonal Mean 計算 (DataStore::compute_zonal_mean)
  - 緯度帯ごとの経度方向平均

### Performance Improvements (v0.3)
- [x] compute_global_range をオンデマンド化 (Global モード時のみ計算, ファイルオープン 2s→瞬時)
- [x] Profile ビューアニメーション最適化 (field reload スキップ, カクカク→滑らか)
- [x] Profile プレイヘッドマーカー (黄色縦線 + ハイライトドット + 値ラベル)

### Done (v0.4 — Suggestion + Trajectory + Code Panel)

目標: 推論エンジン拡張、軌跡オーバーレイ、コード生成

- [x] Visualization Suggestion — Issue #7
  - `VisualizationSuggestion` 構造体 (view_mode, colormap, overlays, symmetric)
  - `suggest_visualization()`: 次元分析 + カテゴリ判定 → 推奨ビュー/カラーマップ/オーバーレイ
  - Inspector "Suggested" セクション: Apply ボタンで一括適用, × で dismiss
  - 変数変更時に suggestion_dismissed 自動リセット
- [x] Trajectory Overlay — Issue #8
  - `TrajectoryData` 構造体, `DataStore::load_trajectory_data()`
  - `detect_trajectory_pair()`: 1D lon/lat ペア自動検出 (名前パターンマッチング)
  - `TrajectoryOverlay`: Globe/Map 両対応 egui painter 描画
    - 過去軌跡: アルファフェード付きポリライン
    - 現在位置: filled circle + white stroke
    - 未来: dimmed dots
    - Globe: 3D 球面座標 → view_proj → スクリーン (裏面カリング)
  - Inspector "Trajectory" セクション: lon/lat 変数 ComboBox, Trail Length スライダー
  - キーボード `T` でトグル
  - 時間同期 (アニメーション連動)
- [x] Code Panel (Read-only) — Issue #9
  - `codegen/python.rs`: xarray + cartopy + matplotlib コード生成
    - ファイルパス, 変数名, time/level isel, 投影法 (Ortho/PlateCarree/Mollweide/Polar)
    - カラーマップ, vmin/vmax (Manual mode), contour/quiver オーバーレイ
  - `Tab::CodePanel`: monospace TextEdit (read-only) + Copy ボタン
  - Inspector 右パネル隣タブとして配置

### Next (v0.5 — スクリプト + 高品質出力)
- [ ] Code Panel 双方向 (Code → GUI 反映, Rhai スクリプトエンジン)
- [ ] GIF/MP4 エクスポート
- [ ] Publication export (タイトル・ラベル・凡例付き高品質出力)

### Next (並行)
- [ ] gtool-rs の CF standard_name 対応改善 — Issue #6

## Architecture
```
geoscope/
├── .cargo/config.toml        # HDF5_DIR 設定
├── Cargo.toml                # eframe 0.33 + wgpu 27 + egui_dock 0.18 + netcdf 0.12
├── docs/
│   ├── PRD.md                # プロダクト要求定義書 v0.4.0
│   ├── DESIGN_REVIEW.md      # 専門家パネルレビュー
│   └── MOCKUP_GUIDE.md       # モックアップ構成 + ペルソナ検証
├── PROGRESS.md               # このファイル
└── src/
    ├── main.rs               # エントリポイント (D&D有効, 1280x800)
    ├── app.rs                # GeoScopeApp (ダークテーマ, egui_dock 3カラム, D&D, GPU統合)
    ├── data/
    │   ├── mod.rs            # DataStore, NetCDF読み込み, Hovmoller/Profile/CrossSection データ
    │   └── inference.rs      # 3段階変数推論エンジン (L1 CF, L2 name, L3 stats)
    ├── renderer/
    │   ├── mod.rs            # Hub (モジュール re-export)
    │   ├── common.rs         # 共通型 (Vertex, CameraUniform, カラーマップ LUT)
    │   ├── globe.rs          # Globe レンダラー (wgpu, UV sphere, WGSL)
    │   ├── map.rs            # Map レンダラー (wgpu, Equirect/Mollweide/Polar, pan+zoom)
    │   ├── hovmoller.rs      # Hovmoller 図 (egui 2D, ColorImage)
    │   ├── spectrum.rs       # E(n) スペクトル (egui painter, log-log)
    │   ├── cross_section.rs  # 鉛直断面 (egui 2D)
    │   ├── profile.rs        # 1D 折れ線グラフ (鉛直プロファイル / 時系列)
    │   ├── contour.rs        # 等値線 (Marching Squares, Globe/Map 両対応)
    │   ├── streamline.rs     # ストリームライン (RK4 積分, 矢じり付き)
    │   ├── trajectory.rs     # 軌跡オーバーレイ (Globe/Map, アルファフェード)
    │   ├── vector_overlay.rs # ベクトル場矢印描画
    │   └── export.rs         # PNG エクスポート (image crate)
    ├── codegen/
    │   ├── mod.rs            # コード生成ハブ
    │   └── python.rs         # Python (xarray+cartopy+matplotlib) 生成
    └── ui/mod.rs             # TabViewer (DataBrowser, Viewport, Inspector, CodePanel)
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
