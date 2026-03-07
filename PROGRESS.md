# GeoScope - Progress

## Overview
GFD (地球流体力学) データ可視化デスクトップアプリケーションの UX/UI デザインプロジェクト。

## Current Status: Design Phase (Mockup Complete)

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

### Next Steps
- [ ] LLM Copilot のモックアップ作成 (チャットUI + パレットUI)
- [ ] エクスポート UI ダイアログのモックアップ
- [ ] ベクトル場オーバーレイの Inspector UI
- [ ] v0.1a 技術 PoC 実装開始 (egui + wgpu + netcdf-rs)
- [ ] gtool-rs の CF standard_name 対応改善

## Architecture
```
geoscope/
├── docs/
│   ├── PRD.md              # プロダクト要求定義書 v0.3.0
│   ├── DESIGN_REVIEW.md    # 専門家パネルレビュー
│   └── MOCKUP_GUIDE.md     # モックアップ構成 + ペルソナ検証
├── PROGRESS.md             # このファイル
└── (src/ - 今後追加)
```
