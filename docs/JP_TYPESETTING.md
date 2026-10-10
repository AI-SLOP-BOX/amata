# 日本語組版パイプライン

日本語の組版要件（縦組み・禁則・和欧間・縦中横・ルビ）が Amata のどこで
どう実装されているかの解説です。数式は `src/core/document/object.rs` の
レイアウト層、各出力は `src/core/text_path.rs`（アウトライン）/
`src/ui/canvas/rendering.rs`（画面）/
`src/io/svg/export.rs`（SVG）/
`src/io/pdf_print.rs`（印刷PDF）に実装されています。

> English version: `docs/JP_TYPESETTING.en.md`.

## 1. レイヤ構成

| 層 | 関数 | 役割 |
|---|---|---|
| 正規化 | `normalize_text` | CRLF/禁則文字の統一 |
| 禁則 | `kinsoku_cannot_start_line` / `kinsoku_cannot_end_line` | 行頭・行末禁則 |
| 和欧間 | `is_ja_latin_boundary` / `ja_latin_gap_em` | 和文↔欧文境界の空き量 |
| 縦中横 | `tatechuyoko_run` / `tatechuyoko_scale` | 2〜3桁の数字を1emセルに横並び |
| ルビ | `parse_ruby_markup` / `parse_ruby` | `｜漢字《かんじ》` / `字(よみ)` |
| 折返 | `wrap_vertical_run` / `wrap_chars` | 縦組=列、横組=行 |

**設計方針**: 記法（`｜…《…》`, `字(よみ)`）は**文字列の中に保持**されます。
各レンダラは行/列の文字列を各自パースするため、レイアウト構造体への
書き換えが不要で、SVG・PDF・キャンバスが同じ結果になります。
ルビは**横組み（読みはベースライン上）と縦組み（読みは親列の右）の
両方**に対応しています。

## 2. 和欧間スペース（auto_spacing）

- `TextStyle::auto_spacing: bool`（既定ON）と
  `TextStyle::auto_spacing_em: f32`（既定 `0.25`）。
- UI: 書体パネルの「和欧間」トグル + `1/8em / 1/4em / 1/2em` の3択。
- 全レンダラが同じ `ja_latin_gap_em(font_size, em)` を呼ぶため、
  画面計測・SVGの `dx`・PDFのTJディスプレースメントが一致します。

## 2b. ぶら下げ（hanging punctuation）

`TextStyle.burasage`（既定ON）。行末の**閉じ約物**（`、。」）…`）が、約物の
*Ink* 分だけ行幅をはみ出して詰められます（Illustrator の和文組版設定と同系）。

- `burasage_hang(ch)`: 句読点 `、。…` は **0.5em**（半角ぶら下げ）、
  閉じ括弧 `」）` は **1.0em**（全角ぶら下げ）、それ以外は0
- 折返し（横組み・縦組み）: 「約物の1字前まで」が nominal 幅に収まるように
  先読みし、約物自身も Ink 分だけはみ出せる
- SVG: 領域テキストの clipPath をぶら下げ分量だけ拡大（Ink が切れない）
- UI: 書体パネルの「ぶら下げ ON/OFF」

## 3. 縦組み

- 方向: `vertical-rl` 相当（最初の列が右端、以降左へ）。
- グリフ: 全角は正立、半角（欧文・数字・ASCII記号）は90°時計回り。
  SVGでは `rotate(90 x y)`（`resvg` は `writing-mode` を無視するため
  CSS依存にしません）。
- 禁則: 行頭禁則（、。・閉じ括弧・小書きかな…）は前行へ**追い込み**、
  行末禁則（開き括弧）は次列へ**追い出し**。どちらも1文字単位の簡易実装
  （ぶら下げ量の調整・追い出し連鎖の反復は未実装）。
- 和欧間: 1/8〜1/2emを境界ごとに挿入。PDFの埋め込みフォント経路では
  TJ配列のディスプレースメントで表現します。

## 4. 縦中横（tate-chū-yoko）

- `tatechuyoko_run` は2〜3桁のASCII数字ランを1emセルとみなします。
  4桁以上は回転（従来通り）。ラン途中からは_split しません
  （`tatechuyoko_run(&['1','2','3','4'], 1)` は `None`）。
- スケール: `tatechuyoko_scale(n) = 1/n`（2桁=50%、3桁=33%）。
  SVGでは `font-size="50%"` の tspan、PDF/アウトラインでは
  PathData をスケール配置します。

## 5. ルビ

- 記法は2種:
  - `｜漢字《かんじ》` — `｜` から `《` までが親文字（複数文字可）
  - `字(よみ)` — 直前の1文字に付けます
- 配置:
  - 縦組み — 読みは親文字列の**右側**（strip中心 `RUBY_STRIP_CENTER_EM = 1.25em`）、
    親文字列の垂直中央、`RUBY_SCALE = 0.5`
  - 横組み — 読みは親文字列の**上**（`RUBY_ABOVE_EM = 0.95em`）、
    親文字列の水平中央、同じく半サイズ
- `wrap_vertical_run` は記法に空 advancing 0 を与えるため、
  折返し計算は親文字だけを勘定します（列文字列には記法が残る）。
- PDF: 縦組みは常にアウトライン化されるので
  `vertical_column_outline` が読みも同時に描きます。横組みは
  `horizontal_ruby_outline` がベースRun（カーニング・フォールバック付き）に
  読みを重ねます。

## 6. OpenType フィーチャー

`TextStyle::ot_features: Vec<OtFeature>`（4文字タグ＋on/off）で、HarfBuzzが
既定で OFF にしている和文系フィーチャーを明示的に制御できます。

- **シェーピング**: `shape_run_hb` は `style.ot_feature_pairs()` を
  HarfBuzz の `Feature` リストに変換して渡します。旧 `ligatures` ブールは
  `liga/dlig/clig/rlig` の4エントリに展開され、`ot_features` が後勝ちします。
- **フォントの対応確認**: `FontRegistry::face_open_type_features` が
  解決済みフェイスのGSUB/GPOSにある全タグを列挙します。Typographicパネルの
  「OpenType 機能」欄に `提供機能: palt, vert, vrt2, …` と表示し、
  未対応トグルは `add_enabled(false)` でグレーアウトします。
- **SVG**: `<text font-feature-settings="'palt' 1">` を出力するので、
  ブラウザ/resvg でも同じOpenType設定が効きます（従来のSVGは生テキスト
  出力で設定が落ちていました）。
- **実測効果**: Noto Sans JP の `、` は既定 1000/1000 em（1em）、
  `palt` 強制ONで 500/1000 em（半角）。テストで検証済み。

### 縦組みの OpenType（vert / vrt2）

縦組みは HarfBuzz の **top-to-bottom シェーピング**（`shape_run_hb_dir`
に `vertical` フラグ）を使い、`vertical_feature_pairs()` が
`vert / vrt2 / vkrn / vpal / valt / vchw / vrtr` を既定ONで追加します
（明示オフはスタイル側が優先）。

- フォントが縦組形を持つ文字は**フォントのグリフをそのまま使う**。
  実測（Noto Sans JP, 40pt）: `（` は横組みで 10.4×38.1 → 縦組みで
  38.1×10.4（=90°回転）、`ー` は 31.8×3.9 → 3.9×31.4（縦方向に伸びる）、
  `、` は 11.6×11.2 → 11.4×11.0（回転せず再配置のみ）。
- フォントが縦組形を持たない半角グリフ（欧文・数字）は従来通り
  90°時計回りに回転（Illustrator の基本挙動）。
- キャンバス/SVG は生テキストを描くため、`is_vert_rotated_char()`
  が「フォントの縦組形が90°回転になるブラケット集合」を定義し、
  同じ回転を適用します（PDF と見た目が揃う）。
- SVG には `font-feature-settings="'palt' 1"` も出るので、
  ブラウザ側でも同等のOpenType設定が効きます。

UI（Typographicパネル）: 合字 / カーニング / 約物半角(palt) / 半角化(halt) /
縦組グリフ(vert) / 縦組代替(vrt2) / ルビ(ruby) のクイックトグルに加え、
**「すべての機能」**パネルで解決済みフェイスの全GSUB/GPOSタグ
（jp78/jp83/jp90/nlck/ccmp/zero/tnum…）を個数無制限にon/offできます
（×で既定に戻す、「オフ設定を全て解除」で全クリア）。

### halt（Alternate Half Widths）

`halt` は約物を半角幅にするフィーチャー。推定モデル側は
`is_halt_char()`（、。，．・：；！？…‥などの約物のみ。ブラケットは
Noto Sans JP実測で半角にならないため除外）で `char_advance_styled()` が
0.5emを返し、**折返し・計測・SVGの位置計算がシェーピング結果と一致**します。
アウトライン/キャンバス（HarfBuzzのhalt形）・PDFは自動で追従。

### テキストのアウトライン化（outline_text）

`export_svg_with_options(doc, embed_fonts, profile, outline_text)` /
`export_png_with_outline(...)` は文字を `<path>` に変換します。
**resvg（ラスタ書き出しエンジン）は `font-feature-settings` を無視する**
ことが実測で確定しているため（全構文で同一インク）、OpenType機能や
フォントの縦組形をSVG/PNGに残す唯一の手段です。Utilityパネルと
書き出しモーダルにチェックボックスがあります。

注意: テキストアウトラインのパスは**アプリ共通の y-down 規約**
（ベースラインより上が負のy）。キャンバスのメッシュキャッシュも
PDFも同じ規約なので、`path_data_to_d_flipped` は `by + p.y` で
そのまま写します（ここを反転させると文字が上下逆になります）。

### SVG ラウンドトリップ

エクスポート時に `font-feature-settings="'palt' 1, 'vert' 0"` を書き、
インポート時に `parse_font_feature_settings()` が読み戻します
（クォート2種・0/1・on/off・bareタグに対応、不正項目はスキップ）。
キャンバスのアウトラインメッシュキャッシュキーも
`ot_features` / `auto_spacing_em` / `text_anchor` を含むように拡張
（トグルが即画面に反映される）。

## 7. 自作アセット

| 種別 | 場所 | 読込 |
|---|---|---|
| スウォッチ | `assets/swatches/*.json` | 印刷パネル「標準スウォッチ」→ 特色として一括登録 |
| テンプレート | `assets/templates/*.amata` | ホーム画面「テンプレートから作成」 |

- どちらも `include_str!` でコンパイル時埋め込み（`src/io/library.rs`）。
  実行時のファイル解決が不要で、配布物単体で動きます。
- スウォッチは当アプリ独自のデータ（伝統色12色のCMYK近似）。
- テンプレートは `tests/asset_gen.rs`（`--ignored`）が生成。
  レイアウトを変えたら再実行してください。生成時にJSONラウンドトリップを
  検証します。

## 8. 目視サンプル

```
cargo test --test jp_sample_dump -- --ignored --nocapture
```

で `output/jp_typesetting_sample.svg`（`.gitignore` 済み）を書き出します。
縦組み＋ルビ＋縦中横／横組み＋ルビ／和欧間1/8・1/4・1/2emの比較／
年号と4桁数字（回転）／横組み禁則を含む1枚ものです。

## 9. テスト

| スイート | カバー |
|---|---|
| `tests/jp_typesetting_tests.rs` | 和欧間量・禁則（追い込み/追い出し）・縦中横・ルビ・SVG出力・禁則文字判定 |
| `tests/text_engine_tests.rs` | 横組みの禁則・折返し・幅推定・カーニング |
| `tests/print_tests.rs` | 埋め込みPDFの和欧間ギャップ・PDF/X・プリフライト |
| `tests/svg_pdf_compat_tests.rs` | SVG往復・PDF互換 |

## 10. 既知の制限

- 禁則は1文字単位の追い込み/追い出しのみ。ぶら下げ量の調整、
  追い出し連鎖の反復、行頭三点リーダーなどの連続処理は未実装。
  （※追い出し自体は連鎖する括弧に対応済み）
- 絵文字は per-glyph フォールバックですが、カラー絵文字フォントの
  エンベッド（PDF/SVG）は未対応です。
- `《》` が入れ子になった場合、最初の `》` で読みを閉じます。
- 横組みルビの読み位置は `char_advance_estimate` モデル由来の概算です
  （実際のグリフ送幅とは最大で数%ずれ得ます。縦組みも同様）。
