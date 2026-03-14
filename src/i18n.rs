//! Internationalization (i18n) support for GeoScope.
//!
//! Uses a global language setting with a `t("key")` lookup function.
//! Supports English and Japanese.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize)]
pub enum Lang {
    #[default]
    En,
    Ja,
}

impl Lang {
    pub fn label(&self) -> &'static str {
        match self {
            Lang::En => "EN",
            Lang::Ja => "JA",
        }
    }

    pub fn next(&self) -> Lang {
        match self {
            Lang::En => Lang::Ja,
            Lang::Ja => Lang::En,
        }
    }
}

static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT_LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn current_lang() -> Lang {
    match CURRENT_LANG.load(Ordering::Relaxed) {
        1 => Lang::Ja,
        _ => Lang::En,
    }
}

/// Translate a key to the current language.
/// Returns `"???"` if the key is not found (visible during development).
pub fn t(key: &str) -> &'static str {
    let lang = current_lang();
    let map = match lang {
        Lang::En => EN.get_or_init(en_translations),
        Lang::Ja => JA.get_or_init(ja_translations),
    };
    map.get(key).copied().unwrap_or("???")
}

static EN: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
static JA: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

macro_rules! define_translations {
    ($($key:literal => $val:literal),* $(,)?) => {{
        let mut m = HashMap::new();
        $(m.insert($key, $val);)*
        m
    }};
}

fn en_translations() -> HashMap<&'static str, &'static str> {
    define_translations! {
        // --- Panel / Tab titles ---
        "data" => "Data",
        "globe" => "Globe",
        "inspector" => "Inspector",
        "code" => "Code",
        "copilot" => "Copilot",

        // --- View mode buttons ---
        "view_globe" => "🌐 Globe",
        "view_map" => "🗺 Map",
        "view_hovmoller" => "📊 Hovmoller",
        "view_spectrum" => "📈 Spectrum",
        "view_section" => "🔪 Section",
        "view_profile" => "📍 Profile",

        // --- Profile modes ---
        "vertical" => "Vertical",
        "time" => "Time",
        "t_lev" => "T-Lev",

        // --- Inspector section headers ---
        "variable" => "Variable",
        "projection" => "Projection",
        "colormap_header" => "Colormap",
        "display" => "Display",
        "spectral_filter" => "Spectral Filter",
        "range" => "Range",
        "cross_section" => "Cross Section",
        "vector_overlay" => "Vector Overlay",
        "contour_lines" => "Contour Lines",
        "streamlines" => "Streamlines",
        "trajectory" => "Trajectory",
        "suggested" => "Suggested",
        "inference" => "Inference",

        // --- Buttons ---
        "grid" => "Grid",
        "smooth" => "Smooth",
        "slice" => "Slice",
        "global" => "Global",
        "manual" => "Manual",
        "apply" => "Apply",
        "reset" => "Reset",
        "run" => "▶ Run",
        "save" => "Save",
        "load" => "Load",
        "copy" => "Copy",
        "export_png_btn" => "Export PNG...",
        "set" => "Set",
        "explain" => "Explain",
        "explore" => "Explore",
        "load_file" => "Load File...",
        "save_png" => "Save PNG",
        "save_gif" => "Save GIF",
        "enabled" => "Enabled",
        "profile_here" => "📍 Profile here",
        "open_profile_tab" => "📊 Open Profile tab",
        "ctx_export_png" => "💾 Export PNG",
        "center_here" => "🌐 Center here",

        // --- Labels ---
        "density" => "Density",
        "scale" => "Scale",
        "levels" => "Levels",
        "trail" => "Trail",
        "axis" => "Axis",
        "fix_lat" => "Fix Lat",
        "fix_lon" => "Fix Lon",
        "index" => "Index",
        "to" => "to",
        "format_label" => "Format:",
        "title_label" => "Title:",
        "resolution_label" => "Resolution:",
        "fps_label" => "FPS:",
        "api_key" => "API Key",
        "sequential" => "Sequential",
        "diverging" => "Diverging",
        "value_label" => "Value:",

        // --- Checkboxes ---
        "wavenumber_filter" => "Wavenumber Filter",
        "temporal_filter" => "Temporal Filter",
        "click_for_spectrum" => "Click a point on Globe/Map",
        "include_colorbar" => "Include colorbar",
        "publication_quality" => "Publication quality",
        "symmetric_centered" => "Symmetric (0-centered)",

        // --- Empty states ---
        "drop_nc_sidebar" => "Drop a .nc file here\nor click Open",
        "drop_nc_main" => "Drop NetCDF file here",
        "drop_nc_hint" => "or use File → Open",
        "select_variable" => "Select a variable",
        "no_file_loaded" => "No file loaded",
        "no_trajectory_pair" => "No trajectory pair detected",
        "no_cross_section" => "No cross-section data available\n(requires variable with level dimension)",
        "no_spectrum" => "No spectral data available",
        "no_matching_commands" => "No matching commands",
        "copilot_empty" => "Ask me about your data or GFD concepts.",

        // --- Tooltips ---
        "cycle_fps" => "Click to cycle FPS",
        "hide_left" => "Hide sidebar [",
        "show_left" => "Show Data [",
        "hide_right" => "Hide sidebar ]",
        "show_right" => "Show Inspector ]",

        // --- Status messages ---
        "ready" => "Ready",
        "thinking" => "Thinking...",
        "exporting_gif" => "Exporting GIF...",
        "code_copied" => "Code copied to clipboard",
        "no_changes" => "No changes detected",

        // --- Export dialog ---
        "export_png" => "Export PNG",
        "export_gif" => "Export GIF",

        // --- Copilot ---
        "you" => "You",
        "copilot_name" => "Copilot",
        "api_key_env_hint" => "Or set ANTHROPIC_API_KEY env var",
        "ask_data_hint" => "Ask about your data...",
        "explain_prompt" => "Explain what I'm currently looking at and its physical significance.",
        "explore_prompt" => "What interesting features can you see in this data? Suggest what to explore next.",

        // --- Command palette ---
        "cmd_placeholder" => "Type a command...",

        // --- Dynamic format fragments ---
        "opened" => "Opened:",
        "error_prefix" => "Error:",
        "exported_prefix" => "Exported",
        "export_error" => "Export error:",
        "gif_export_error" => "GIF export error:",
        "recipe_saved" => "Recipe saved to:",
        "save_failed" => "Save failed:",
        "recipe_loaded" => "Recipe loaded from:",
        "load_failed" => "Load failed:",
        "applied" => "Applied:",
        "units_label" => "Units:",
        "frames" => "frames",
        "detected" => "Detected:",
    }
}

fn ja_translations() -> HashMap<&'static str, &'static str> {
    define_translations! {
        // --- Panel / Tab titles ---
        "data" => "データ",
        "globe" => "地球儀",
        "inspector" => "設定",
        "code" => "コード",
        "copilot" => "Copilot",

        // --- View mode buttons ---
        "view_globe" => "🌐 地球儀",
        "view_map" => "🗺 地図",
        "view_hovmoller" => "📊 ホフメラー",
        "view_spectrum" => "📈 スペクトル",
        "view_section" => "🔪 断面",
        "view_profile" => "📍 プロファイル",

        // --- Profile modes ---
        "vertical" => "鉛直",
        "time" => "時系列",
        "t_lev" => "時×高度",

        // --- Inspector section headers ---
        "variable" => "変数",
        "projection" => "投影法",
        "colormap_header" => "カラーマップ",
        "display" => "表示",
        "spectral_filter" => "スペクトルフィルタ",
        "range" => "範囲",
        "cross_section" => "断面",
        "vector_overlay" => "ベクトルオーバーレイ",
        "contour_lines" => "等値線",
        "streamlines" => "流線",
        "trajectory" => "軌跡",
        "suggested" => "推奨設定",
        "inference" => "推論",

        // --- Buttons ---
        "grid" => "格子点",
        "smooth" => "補間",
        "slice" => "スライス",
        "global" => "全体",
        "manual" => "手動",
        "apply" => "適用",
        "reset" => "リセット",
        "run" => "▶ 実行",
        "save" => "保存",
        "load" => "読込",
        "copy" => "コピー",
        "export_png_btn" => "PNGエクスポート...",
        "set" => "設定",
        "explain" => "解説",
        "explore" => "探索",
        "load_file" => "ファイル読込...",
        "save_png" => "PNG保存",
        "save_gif" => "GIF保存",
        "enabled" => "有効",
        "profile_here" => "📍 ここのプロファイル",
        "open_profile_tab" => "📊 プロファイルタブを開く",
        "ctx_export_png" => "💾 PNGエクスポート",
        "center_here" => "🌐 ここを中心に",

        // --- Labels ---
        "density" => "密度",
        "scale" => "スケール",
        "levels" => "レベル数",
        "trail" => "軌跡長",
        "axis" => "軸",
        "fix_lat" => "緯度固定",
        "fix_lon" => "経度固定",
        "index" => "インデックス",
        "to" => "〜",
        "format_label" => "形式:",
        "title_label" => "タイトル:",
        "resolution_label" => "解像度:",
        "fps_label" => "FPS:",
        "api_key" => "APIキー",
        "sequential" => "Sequential",
        "diverging" => "Diverging",
        "value_label" => "値:",

        // --- Checkboxes ---
        "wavenumber_filter" => "波数フィルタ",
        "temporal_filter" => "時間フィルタ",
        "click_for_spectrum" => "Globe/Mapで点をクリック",
        "include_colorbar" => "カラーバーを含む",
        "publication_quality" => "論文品質",
        "symmetric_centered" => "対称 (0中心)",

        // --- Empty states ---
        "drop_nc_sidebar" => ".ncファイルをドロップ\nまたは「開く」をクリック",
        "drop_nc_main" => "NetCDFファイルをドロップ",
        "drop_nc_hint" => "または File → Open",
        "select_variable" => "変数を選択してください",
        "no_file_loaded" => "ファイルが開かれていません",
        "no_trajectory_pair" => "軌跡ペアが検出されません",
        "no_cross_section" => "断面データがありません\n（レベル次元を持つ変数が必要です）",
        "no_spectrum" => "スペクトルデータがありません",
        "no_matching_commands" => "一致するコマンドがありません",
        "copilot_empty" => "データやGFDの概念について質問できます。",

        // --- Tooltips ---
        "cycle_fps" => "クリックでFPS切替",
        "hide_left" => "サイドバーを隠す [",
        "show_left" => "データを表示 [",
        "hide_right" => "サイドバーを隠す ]",
        "show_right" => "設定を表示 ]",

        // --- Status messages ---
        "ready" => "準備完了",
        "thinking" => "考え中...",
        "exporting_gif" => "GIFエクスポート中...",
        "code_copied" => "コードをクリップボードにコピーしました",
        "no_changes" => "変更は検出されませんでした",

        // --- Export dialog ---
        "export_png" => "PNGエクスポート",
        "export_gif" => "GIFエクスポート",

        // --- Copilot ---
        "you" => "あなた",
        "copilot_name" => "Copilot",
        "api_key_env_hint" => "または ANTHROPIC_API_KEY 環境変数を設定",
        "ask_data_hint" => "データについて質問...",
        "explain_prompt" => "今見ているデータの内容と物理的な意味を説明してください。",
        "explore_prompt" => "このデータにどんな興味深い特徴がありますか？次に何を探索すべきか提案してください。",

        // --- Command palette ---
        "cmd_placeholder" => "コマンドを入力...",

        // --- Dynamic format fragments ---
        "opened" => "開いた:",
        "error_prefix" => "エラー:",
        "exported_prefix" => "エクスポート完了",
        "export_error" => "エクスポートエラー:",
        "gif_export_error" => "GIFエクスポートエラー:",
        "recipe_saved" => "レシピ保存先:",
        "save_failed" => "保存失敗:",
        "recipe_loaded" => "レシピ読込元:",
        "load_failed" => "読込失敗:",
        "applied" => "適用:",
        "units_label" => "単位:",
        "frames" => "フレーム",
        "detected" => "検出:",
    }
}
