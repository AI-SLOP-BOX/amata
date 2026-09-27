//! Display-side Japanese labels for strings that live in the data model as
//! English identifiers.
//!
//! Commands keep their original English `label` (documents written by older
//! builds stay byte-identical, and undo/redo keeps matching on the same key);
//! the translation happens only where the string is drawn.

/// Undo / redo history action name → 日本語表示。未知の名前はそのまま返す。
pub fn history_name(name: &str) -> &str {
    match name {
        // ── オブジェクトの配置・階層 ──
        "Bring to Front" => "最前面へ",
        "Bring Forward" => "1つ前へ",
        "Send Backward" => "1つ後ろへ",
        "Send to Back" => "最背面へ",
        "Reorder Object" => "オブジェクトを並べ替え",
        "Move Object in Tree" => "ツリー内で移動",

        // ── グループ・パス ──
        "Group" => "グループ化",
        "Ungroup" => "グループ解除",
        "Make Compound Path" => "複合パスを作成",
        "Release Compound" => "複合パスを解除",
        "Clipping Mask" => "クリッピングマスク",
        "Create Outlines" => "アウトラインを作成",
        "Simplify Path" => "パスを簡略化",
        "Pathfinder" => "パスファインダー",
        "Edit Path Nodes" => "パスノードを編集",

        // ── 整列・等間隔 ──
        "Align Left" => "左揃え",
        "Align Center H" => "水平中央揃え",
        "Align Right" => "右揃え",
        "Align Top" => "上揃え",
        "Align Center V" => "垂直中央揃え",
        "Align Bottom" => "下揃え",
        "Distribute H" => "水平等間隔",
        "Distribute V" => "垂直等間隔",

        // ── オブジェクト編集 ──
        "Add Object" => "オブジェクトを追加",
        "Remove Object" => "オブジェクトを削除",
        "Edit Object" => "オブジェクトを編集",
        "Move Object" => "オブジェクトを移動",
        "Edit Transform" => "変形を編集",
        "Change Typography" => "タイポグラフィを変更",

        // ── 切り取り・貼り付け・移動 ──
        "Cut Objects" => "オブジェクトを切り取り",
        "Paste Objects" => "オブジェクトを貼り付け",
        "Duplicate Objects" => "オブジェクトを複製",
        "Alt Duplicate" => "Altで複製",
        "Delete Objects" => "オブジェクトを削除",
        "Move Objects" => "オブジェクトを移動",
        "Move Multi" => "複数を移動",
        "Nudge" => "微調整",

        // ── レイヤー ──
        "Add Layer" => "レイヤーを追加",
        "Delete Layer" => "レイヤーを削除",
        "Edit Layer" => "レイヤーを編集",
        "Reorder Layers" => "レイヤーを並べ替え",

        // ── ジェネレーティブ・エフェクト ──
        "Neon Glow" => "ネオングロー",
        "Grid Repeat" => "グリッドリピート",
        "Radial Repeat" => "放射リピート",
        "Add Auto Layout" => "オートレイアウトを追加",

        _ => name,
    }
}
