# GeoScope Mockup Guide

## Overview

Pencil (.pen) ファイルによるインタラクティブモックアップの構成と、ペルソナ検証の結果を記録する。

- **ファイル**: `/Users/aiq/Documents/untitled.pen`
- **デザインシステム**: lunaris (100 コンポーネント)
- **テーマ**: ダークモード (`--primary: #00A49A`)

---

## Screen Map

### メイン画面 (4 screens)

| ID | Name | Description |
|----|------|-------------|
| `fLpXD` | Level 0: Welcome | ファイルドロップゾーン、最近使ったファイル、サンプルデータ |
| `b6VDt` | Level 1: Basic View | 地球儀 + サイドバー + タイムスライダー。初回表示状態 |
| `Vadwy` | Level 2: Explorer | Data Browser + Inspector + Code Panel 全表示 |
| `DW8H0` | Level 3: Code Mode | Rhai/Python コードエディタ中心のレイアウト |

### 追加画面 (5 screens)

| ID | Name | Description |
|----|------|-------------|
| `XFkEj` | A1: Onboarding Overlay | 初回起動時のヒントカード (回転/ズーム/クリック) + Got it ボタン |
| `0zpFd` | A2: Hovmoller View | 時間-経度ヒートマップダイアグラム。RdBu_r カラーマップ |
| `BhlDU` | A3: Spectrum View | E(n) エネルギースペクトル log-log プロット。n^-3 参照線付き |
| `cgUZf` | A4: Fullscreen Mode | 地球儀最大化。HUD オーバーレイ (変数名/座標/値/時刻) |
| `fQYeH` | A5: Variable Label | ビューポート上の変数メタデータオーバーレイ (名前/次元/範囲) |

---

## Persona Journey Validation

### Persona A: 田中 (M1 院生、GFD 初学者)

**Journey**: ファイルを開く → 地球儀を回す → 時系列アニメーション → 発表用スクショ

| Step | Screen | Validation | Issue |
|------|--------|------------|-------|
| サンプルデータで起動 | L0 Welcome | サンプルボタンあり | - |
| 初回操作ガイド | A1 Onboarding | 3 つのヒントカードで操作説明 | - |
| 地球儀回転 | L1 Basic | ドラッグで回転 (左ドラッグ) | - |
| アニメーション再生 | L1 Basic | タイムスライダー + 再生ボタン | - |
| スクリーンショット保存 | L1 Basic | トップバーに保存アイコン想定 | P1: エクスポート UI 未デザイン |

**Result**: 基本フローは L0 → A1 → L1 でカバー。エクスポート UI が追加モックアップ候補。

---

### Persona B: 佐藤 (D2 院生、数値実験の日常ユーザー)

**Journey**: NetCDF を開く → 変数選択 → Hovmoller → スペクトル解析 → コード生成

| Step | Screen | Validation | Issue |
|------|--------|------------|-------|
| ファイルドロップ | L0 Welcome | ドロップゾーンあり | - |
| 変数推定 (3-stage fallback) | L1 Basic | 自動推定 + サイドバーで変数選択 | - |
| 変数メタ確認 | A5 Variable Label | 変数名/次元/範囲をビューポート上に表示 | - |
| Hovmoller 表示 | A2 Hovmoller | 経度-時間ヒートマップ | - |
| E(n) スペクトル | A3 Spectrum | log-log プロット + n^-3 参照線 | - |
| フルスクリーンで確認 | A4 Fullscreen | HUD 付きで集中表示 | - |
| コード生成 | L3 Code Mode | Rhai/Python コードエディタ | - |

**Result**: 全ステップをカバー。D2 院生の典型ワークフローを完全にサポート。

---

### Persona C: 山田 (准教授、講義・研究指導)

**Journey**: 講義資料用に複数変数を比較 → 学生に共有 → 論文用図の品質調整

| Step | Screen | Validation | Issue |
|------|--------|------------|-------|
| 複数変数の切り替え | L2 Explorer | Data Browser で変数一覧 | - |
| Inspector でパラメータ調整 | L2 Explorer | Inspector パネルで詳細設定 | - |
| 論文品質エクスポート | - | 未実装 | P0: 高解像度 PNG/SVG エクスポート必要 |
| 講義での画面共有 | A4 Fullscreen | HUD 付きフルスクリーン | - |
| コード共有 | L3 Code Mode | コード生成 → コピー → 学生に配布 | - |

**Result**: 基本フローはカバー。論文品質エクスポート機能の UI が今後の課題。

---

## Identified Gaps (from Persona Validation)

| Priority | Gap | Proposed Solution |
|----------|-----|-------------------|
| P0 | 高解像度エクスポート UI | Export ダイアログモックアップ (今後追加) |
| P1 | 複数変数の比較ビュー (split view) | L2 Explorer の拡張 (v0.2 スコープ) |
| P1 | ベクトル場オーバーレイの操作 UI | Inspector に矢印/流線設定追加 |
| P2 | アニメーション GIF/MP4 エクスポート | Export ダイアログの拡張 |

---

## Design Decisions

1. **ダークモード優先**: 長時間作業とデータ可視化のコントラスト確保
2. **Progressive Disclosure**: L0→L1→L2→L3 の段階的 UI 開示
3. **HUD パターン**: フルスクリーン時は半透明オーバーレイで最小限の情報表示
4. **カラーマップ規約**: 温度=sequential (inferno)、偏差=diverging (RdBu_r)、自動判定あり
5. **Onboarding**: 初回のみ、3 ステップの非モーダルヒントカード
