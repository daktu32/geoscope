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
    - 全軌跡描画 (t=0〜現在): 直近部分は明るく、古い部分は薄く表示
    - 開始点マーカー (hollow circle, t=0 固定)
    - 現在位置: filled circle + white stroke
    - Globe: 3D 球面座標 → view_proj → スクリーン (裏面カリング, 緯度反転対応)
    - radians 単位自動検出 + degrees 変換
  - Inspector "Trajectory" セクション: lon/lat 変数 ComboBox, Trail Length スライダー (10-2000, 対数)
  - キーボード `T` でトグル
  - 時間同期 (アニメーション連動)
- [x] Code Panel (Read-only) — Issue #9
  - `codegen/python.rs`: xarray + cartopy + matplotlib コード生成
    - ファイルパス, 変数名, time/level isel, 投影法 (Ortho/PlateCarree/Mollweide/Polar)
    - カラーマップ, vmin/vmax (Manual mode), contour/quiver オーバーレイ
  - `Tab::CodePanel`: monospace TextEdit (read-only) + Copy ボタン
  - Inspector 右パネル隣タブとして配置

### Interactive Profile (v0.4)
- [x] Globe/Map クリックで観測点選択 (lon/lat → profile_point)
  - Map: hover UV 座標変換を流用、ドラッグと区別
  - Globe: 逆 view 行列 (rot_y^T * rot_x^T) による ray-sphere intersection
- [x] スプリットビュー: Globe/Map (60%) + Profile (40%) を並列表示
  - x ボタンで閉じる
  - 分割線 + ツールバー
- [x] 選択点マーカー (金色十字 + 円 + 黒縁取り)
  - Globe: view_proj 投影 + view 行列 z 行による裏面カリング
  - Map: UV→screen 変換 (pan/zoom 対応)
- [x] 3 プロファイルモード切替 (Vertical / Time / T-Lev)
  - Vertical: 鉛直プロファイル (y軸=level, x軸=物理量 — 気象学慣例準拠)
  - Time: 時系列 (time 次元、playhead マーカー付き)
  - T-Lev: Time × Level ヒートマップ (ColorImage テクスチャ)
- [x] TimeLevelData 構造体 + load_time_level_data() (全 time × 全 level 一括読み込み)
- [x] プロファイルタイトルに実際の lon/lat 度数表示

### UI Improvements (v0.4)
- [x] タイムコントロール: 動画プレイヤー風2段レイアウト (フル幅シークバー + コントロール行)
- [x] コマ送りボタン (⏮/⏭): 1フレームずつ前後移動
- [x] レベルスライダー: 縦型サイドパネル化、「Level」ヘッダー + 次元名 + 座標値表示、中央配置
- [x] フローティングズームボタン: ビューポート右下に +/− ボタン (Google Maps 風)

### Bug Fixes (v0.4)
- [x] Contour Globe 描画修正: 独自座標系 → Globe mesh 座標系 + 共有 view_proj 行列に統一
- [x] Globe overlay 共通化: vector/contour/trajectory が同一の build_view_proj を共有

### Done (v0.4+ — 品質改善)

- [x] セッション永続化 (eframe persistence + serde, アプリ終了時に保存/再起動時に復元)
  - ファイルパス, 変数選択, ビューモード, カメラ位置, オーバーレイ設定, プロファイルポイント
  - 重複ファイルオープン防止 (DataStore::open_file でパス重複チェック)
  - auto_save_interval を無効化 (スライダー操作時のカクツキ防止)
- [x] Globe クリック精度改善
  - 二重否定バグ修正 (lat_deg の南北反転)
  - paint() が実際に使った rect を返して overlay/click と共有 (rect 不一致排除)
  - クリック逆変換で view 行列の転置を直接使用 (マーカー forward と完全一致保証)
- [x] 鉛直プロファイル軸入替 (y軸=level, x軸=物理量 — 気象学慣例準拠)
  - プレイヘッド: 垂直線→水平線, ホバー最近傍検索も y 方向ベースに
- [x] Level スライダー高さ修正
  - egui Slider は add_sized が効かない問題を spacing().slider_width で解決
  - プロファイル分割時にプロット領域と高さを合わせる (remaining * 0.6)
- [x] Projection ドロップダウンを Map ビュー時のみ表示
- [x] プロファイル range_mode 対応 (Global/Manual モード反映)
- [x] アニメーション最適化 (再生中はプロファイル再読み込みスキップ, ポイント変更時は即反映)

### Done (v0.4++ — UI/UX 大幅改善)

- [x] レイアウト刷新: egui_dock サイドバー → egui::SidePanel (アニメーション対応)
  - 左パネル (Data Browser): 10% 幅, コラプス対応
  - 右パネル (Inspector/Code): 20% 幅, コラプス対応
  - 中央ペーン: タブバー・×ボタン削除 (Viewport のみなので不要)
- [x] コラプシブルサイドバー: `[`/`]` キー + マウストグルボタン (‹/›, 28px ストリップ)
- [x] ビューモードアイコン: 🌐 Globe, 🗺 Map, 📊 Hovmoller, 📈 Spectrum, 🔪 Section, 📍 Profile
- [x] Inspector コラプシブルセクション: egui::CollapsingHeader 化 (Variable/Colormap/Range は展開, Vector/Contour 等は折りたたみ)
- [x] レベルスライダー: SidePanel → egui::Area (Order::Foreground) フローティングオーバーレイ
- [x] プレイヘッドライン: 全関連ビューに現在次元選択を線で表示
  - Hovmoller: 現在時刻の水平線 (黄色)
  - CrossSection: 現在レベルの水平線 (黄色)
  - Profile: モードに応じた時刻/レベルのプレイヘッド
  - T-Lev: 時刻 (縦線) + レベル (横線) の二重プレイヘッド
- [x] Profile タブモードセレクタ: Vertical/Time/T-Lev ボタンをビュー内に配置
- [x] 右クリックコンテキストメニュー: Globe/Map で右クリック → 座標表示 + Profile here / Open Profile / Export PNG / Center here
- [x] ドロッププレースホルダー: ファイル未読み込み時に「📂 Drop NetCDF file here」を中央に表示

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
