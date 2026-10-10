# ファイル互換サポート範囲 (honest subset)

Amata は Adobe Illustrator の代替を狙っていますが、Illustrator の
ネイティブフォーマット（`.ai` / `.ait` / `.ase` / `.acb`）は**非公開**
です。この文書は、どの形式をどこまで読めて書けるかを正直に列挙した
ものです。「読める」は「完全な情報が失われずラウンドトリップする」
ことを意味します。

> 英語版: see `docs/COMPATIBILITY.en.md`.

## サマリ

| 形式 | 読込 | 書出 | 範囲 |
|---|---|---|---|
| SVG 1.1 | ◎ 広域 | ◎ 広域 | パス・グラデーション・変形・テキスト・クリップ・パターン |
| PDF | ◎ 広域（`pdfium`） | ○ 標準PDF / PDF/X-1a, PDF/X-4 | ベクタ・画像・テキスト・ICC・分版・OutputIntent |
| `.ai` | ○ PDF互換部分のみ | ○ PDF互換部分のみ | Illustrator 13+ の `.ai` はPDFベース。Amataはこの部分集合を読む／書く |
| PNG / JPEG / WebP | ◎ | ○ | ラスタ入出力（ICCプロファイル対応） |
| `.amata`（プロジェクトJSON） | ◎ 完全 | ◎ 完全 | 当アプリの唯一の完全な保存形式 |
| `.ase`（Adobe Swatch Exchange） | ○ 読込のみ | ✕ | 特色スポットのリスト（RGB/CMYK/Gray/LAB、グループは平坦化） |
| Pantone 特色 | ○ 内蔵キット | ○ 分版出力 | ソリッドコート主要18色。メタリック・パストル・ネオン・USC/Uバリアントは対象外 |

## .ai / .ait の詳細

- **読込**：Illustrator 13（2003）以降の `.ai` ファイルはPDF互換の
  コンテナです。Amata はこのPDF部分を [`src/io/pdf_import.rs`](../src/io/pdf_import.rs)
  経由で読み込みます。 Illustrator 8〜10 の旧ネイティブ形式、および
  Illustrator 固有の拡張（ライブカラー、グラフ、遠近グリッド、
  アピアランスの分解情報、ライブトレース結果など）は読み取れません。
- **書出**：PDF互換の`.ai`として書き出します。破損したPDFを
  生成しないことを最優先に、テキストは埋め込みアウトラインまたは
  埋め込みフォントのどちらかに解決されます（PDF/Xの場合は
  `OutputIntent` + 分版）。
- **非対応**： Illustrator の「アピアランス」パネル構造、効果の
  編集可能な保持、ライブシェイプのプリミティブ情報、グラフ、
  変数データ、3Dの材質定義。 これらはReaderとしては単純化された
  ベクタになります。

## .ase（スウォッチ）

- `src/io/ase.rs` は Adobe Swatch Exchange の**読込専用**サブセットです。
  RGB / CMYK / Gray / LAB の各スウォッチをドキュメントの特色
  ライブラリ（`Document::spots`）に取り込みます。グループ
  （`c001`/`c002`）は構造として保持せず平坦化し、重複名は後勝ちです。
- 書出（`.ase` ライタ）は意図的に未実装です。特色の書き出しは
  PDFの分版（Separation）経由で行ってください。
- LAB スウォッチはD50→D65のBradford適応を経てsRGB→CMYK（naive-UCR）
  に変換されます。ICCプロファイルによる精密な色管理は
  `src/core/icc.rs`（lcms2）が担当し、印刷出力ではそちらが優先されます。

## Pantone / 特色

- 特色はいつでも名前付きで登録でき、PDF出力では
  `/Separation` 色空間 + `tintTransform` の版として出ます。
  （`src/io/pdf_print.rs` の `Ctx::spot_tint_name`）
- 内蔵Pantoneキット（`src/core/print.rs` の `PANTONE_KIT`）は
  ソリッドコート主要18色の**公表プロセスフォールバック値**です。
  ICCで検証されたインキ実測値ではありません。メタリック・パストル・
  ネオン・ coated/USC/UバリアントはCMYKで表現できないため収録していません。
- キットに無い名前は「現在の塗り色」で特色登録されます（UIで明示）。

## PDF/X と分版

- PDF/X-1a: 全要素をプロセスCMYKまたは特色に変換。 透明度は
  加算ではなく press-DPI ラスタに平坦化（`flatten_regions`）。
- PDF/X-4: ライブ透明度・ICCベースのカラーを保持。
- 両者とも `OutputIntent` を埋め込み、 `BleedBox`/`TrimBox` の整合を
  検証します（`validate_pdfx`）。

## 自作アセット（バンドル）

`assets/` に、Amataチームが作成したコンテンツを同梱しています（`src/io/library.rs`
が `include_str!` でコンパイル時埋め込み）。

| 種別 | 内容 | 読み込み |
|---|---|---|
| 標準スウォッチ（4種） | 日本の伝統色18色・DIC近似18色・無彩色12段階・UIフラット12色 | 印刷パネル「標準スウォッチ」 |
| テンプレート（7種） | A4縦組みポスター・名刺・A4横2段チラシ・請求書（CMYK+特色）・年賀状・SNS 9:16・方眼ノート | ホーム画面「テンプレートから作成」 |

- スウォッチは全部 **CMYK近似**（DIC/伝統色は実測値ではなく近似）。特色版名はslugがそのまま出ます。
- テンプレートは `tests/asset_gen.rs`（`--ignored`）が生成。レイアウト変更時は再実行。
- 目視確認は `cargo test --test asset_preview -- --ignored --nocapture` →
  `output/templates-preview.png` に全テンプレートを並べて書き出します。

## 印刷オプション早見

| オプション | 効果 |
|---|---|
| `PrintPdfOptions.outline_text` | 全テキストをグリフパスに（フォント未埋込・OpenType忠実） |
| SVG/PNG の `outline_text` | `export_svg_with_options` / `export_png_with_outline`。resvg が `font-feature-settings` を無視するため、OpenType を残す唯一の手段 |
| CLI | `amata render --outline-text` / `amata convert --outline-text`（.svg 入力では警告して無視） |
| ぶら下げ | `TextStyle.burasage`（既定ON）。`JP_TYPESETTING.md` 参照 |

### PDF/X 準拠項目（ISO 15930-1 / PDF/X-1a:2001）

| 必須項目 | 状態 |
|---|---|
| `/OutputIntents` + 埋め込みICC `/DestOutputProfile` | ✅ |
| `MediaBox/CropBox/BleedBox/TrimBox` 整合 | ✅（`validate_pdfx` が検証） |
| `/GTS_PDFXVersion`（/Metadata XMP の `pdfxid:GTS_PDFXVersion`） | ✅ |
| **XMP `/Metadata` ストリーム**（`pdfxid` スキーマ、Producer/title） | ✅ 実装済み |
| **trailer `/ID`**（同一hex32桁×2、FNV-1aで内容から導出） | ✅ 実装済み |
| PDFバージョン | PDF 1.4（X-1aの基底規格。**X-4を名乗るなら1.6が必要**） |
| 生の透明・RGB-only・未埋込フォント | ✅ それぞれ検出/平坦化して排除 |
| **外部検証器での検証** | ✅ `qpdf --check` をテストに統合（構造検証） |

**残る正直な注記**:
- PDF/X-1a は「特色を含む透明オブジェクト」をラスタ化するため**特色版が失われる**。
  エクスポータは警告する（`tests/print_capability_tests.rs` のシナリオ1）。
- **RIP実機検証は未実施**。qpdf は構造保証のみで、ISO 15930 の適合判定は
  veraPDF 等の専用ツールが必要。
- 自動トラップは濃度（`luminance`）で暗い側を選ぶが、**幅は全域固定**。

## テスト

| スイート | 内容 |
|---|---|
| `tests/svg_pdf_compat_tests.rs` | SVG往復・PDFガード・AI報告・互換 |
| `tests/ase_tests.rs` | `.ase` 読込（RGB/CMYK/Gray/LAB・グループ・UTF-16BE名）・Pantoneキット |
| `tests/print_tests.rs` | 埋め込みテキスト・PDF/X検証・プリフライト・分版 |
