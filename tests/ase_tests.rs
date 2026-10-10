//! Adobe Swatch Exchange (.ase) import tests.
//!
//! The `.ase` binary is synthesised here instead of shipped as a
//! fixture: the format is a small, stable container and a builder keeps
//! the cases readable.

use irasu_illustrator::core::print::{
    pantone_cmyk, pantone_display_name, rgb_to_cmyk_ink, SpotColor,
};
use irasu_illustrator::io::ase::{import_ase, import_ase_bytes, AseError};

/// One colour entry: (model tag, values, name).
type Entry = (&'static [u8; 4], Vec<f32>, &'static str);

/// Build an `.ase` container from colour entries and optional group
/// blocks (`true` = group start, `false` = group end).
fn ase(version: u16, blocks: &[AseBlock]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"ASEF");
    out.extend_from_slice(&version.to_be_bytes());
    out.extend_from_slice(&(blocks.len() as u32).to_be_bytes());
    for b in blocks {
        let (kind, body) = match b {
            AseBlock::Color(e) => (0x0001_u16, color_body(e)),
            AseBlock::GroupStart(name) => (0xC001_u16, group_body(name)),
            AseBlock::GroupEnd => (0xC002_u16, Vec::new()),
        };
        out.extend_from_slice(&kind.to_be_bytes());
        out.extend_from_slice(&(body.len() as u32).to_be_bytes());
        out.extend_from_slice(&body);
    }
    out
}

enum AseBlock {
    Color(Entry),
    GroupStart(&'static str),
    GroupEnd,
}

fn color_body((model, values, name): &Entry) -> Vec<u8> {
    let mut body = Vec::new();
    let units: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    body.extend_from_slice(&(units.len() as u16).to_be_bytes());
    for u in &units {
        body.extend_from_slice(&u.to_be_bytes());
    }
    body.extend_from_slice(*model);
    for v in values {
        body.extend_from_slice(&v.to_bits().to_be_bytes());
    }
    body
}

fn group_body(name: &str) -> Vec<u8> {
    let mut body = Vec::new();
    let units: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    body.extend_from_slice(&(units.len() as u16).to_be_bytes());
    for u in &units {
        body.extend_from_slice(&u.to_be_bytes());
    }
    body
}

fn names(spots: &[SpotColor]) -> Vec<String> {
    spots.iter().map(|s| s.name.clone()).collect()
}

#[test]
fn rgb_swatches_convert_to_process_ink() {
    let bytes = ase(
        2,
        &[AseBlock::Color((b"RGB ", vec![1.0, 0.0, 0.0], "Brand Red"))],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    assert_eq!(names(&spots), vec!["Brand Red".to_string()]);
    let expect = rgb_to_cmyk_ink(1.0, 0.0, 0.0);
    for (a, b) in spots[0].cmyk.iter().zip(expect) {
        assert!((a - b).abs() < 1e-6, "{:?} vs {expect:?}", spots[0].cmyk);
    }
    // Pure red has no cyan/black, full magenta+yellow.
    assert!(spots[0].cmyk[0].abs() < 1e-6 && spots[0].cmyk[3].abs() < 1e-6);
}

#[test]
fn cmyk_and_gray_pass_through() {
    let bytes = ase(
        2,
        &[
            AseBlock::Color((b"CMYK", vec![0.1, 0.2, 0.3, 0.4], "Ink")),
            AseBlock::Color((b"Gray", vec![0.25], "Half Gray")),
        ],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    assert_eq!(spots.len(), 2);
    for (a, b) in spots[0].cmyk.iter().zip([0.1, 0.2, 0.3, 0.4]) {
        assert!((a - b).abs() < 1e-6);
    }
    assert_eq!(spots[1].cmyk, [0.0, 0.0, 0.0, 0.25]);
}

#[test]
fn lab_white_maps_to_zero_ink() {
    // L=100 a=0 b=0 is white → K 0, nothing else.
    let bytes = ase(
        2,
        &[AseBlock::Color((b"LAB ", vec![100.0, 0.0, 0.0], "Paper"))],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    for c in spots[0].cmyk {
        assert!(c.abs() < 0.02, "white has no ink: {:?}", spots[0].cmyk);
    }
}

#[test]
fn groups_flatten_and_duplicates_merge_last_wins() {
    let bytes = ase(
        2,
        &[
            AseBlock::GroupStart("Brand"),
            AseBlock::Color((b"CMYK", vec![0.0, 0.0, 1.0, 0.0], "Yellow")),
            AseBlock::Color((b"CMYK", vec![0.0, 0.5, 0.0, 0.0], "Red")),
            AseBlock::GroupEnd,
            AseBlock::GroupStart("Overrides"),
            AseBlock::Color((b"CMYK", vec![1.0, 0.9, 0.0, 0.0], "Red")),
            AseBlock::GroupEnd,
        ],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    assert_eq!(names(&spots), vec!["Yellow".to_string(), "Red".to_string()]);
    assert_eq!(spots[1].cmyk, [1.0, 0.9, 0.0, 0.0], "last write wins");
}

#[test]
fn japanese_names_survive_utf16be() {
    let bytes = ase(
        2,
        &[AseBlock::Color((
            b"CMYK",
            vec![0.0, 1.0, 1.0, 0.0],
            "企業赤",
        ))],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    assert_eq!(names(&spots), vec!["企業赤".to_string()]);
}

#[test]
fn unknown_models_and_bad_files_are_handled() {
    // Unknown model: swatch skipped, import still succeeds.
    let bytes = ase(
        2,
        &[
            AseBlock::Color((b"HWB ", vec![0.1, 0.2, 0.3], "Odd")),
            AseBlock::Color((b"CMYK", vec![0.0, 0.0, 0.0, 1.0], "Black")),
        ],
    );
    let spots = import_ase_bytes(&bytes).expect("ase import");
    assert_eq!(names(&spots), vec!["Black".to_string()]);

    assert_eq!(import_ase_bytes(b"NOPE..."), Err(AseError::NotAse));
    // Count promises two blocks but the file ends after one.
    let mut short = ase(2, &[AseBlock::Color((b"CMYK", vec![0.0; 4], "A"))]);
    short.truncate(short.len() - 4);
    assert_eq!(import_ase_bytes(&short), Err(AseError::Truncated));
    // Bogus header version.
    let mut junk = ase(2, &[]).to_vec();
    junk[4] = 0;
    junk[5] = 7;
    assert_eq!(import_ase_bytes(&junk), Err(AseError::NotAse));
}

#[test]
fn file_path_import_matches_bytes() {
    let bytes = ase(
        2,
        &[AseBlock::Color((b"CMYK", vec![0.1, 0.2, 0.3, 0.4], "Path"))],
    );
    let dir = std::env::temp_dir().join("amata_ase_test");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("swatches.ase");
    std::fs::write(&path, &bytes).expect("write ase");
    let from_path = import_ase(&path).expect("file import");
    assert_eq!(names(&from_path), vec!["Path".to_string()]);
    assert_eq!(from_path[0].cmyk, [0.1, 0.2, 0.3, 0.4]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn missing_file_reports_io_error() {
    let err = import_ase(std::path::Path::new("/nonexistent/nope.ase")).expect_err("missing file");
    assert!(matches!(err, AseError::Io(_)), "{err:?}");
}

#[test]
fn pantone_kit_accepts_aliases() {
    // Kit spelling, bare formula and loose punctuation all resolve.
    assert!(pantone_cmyk("Pantone 185 C").is_some());
    assert!(pantone_cmyk("185 C").is_some());
    assert!(pantone_cmyk("PANTONE  185c").is_some());
    let [c, m, y, k] = pantone_cmyk("185 C").expect("185 C in kit");
    assert!(
        c.abs() < 1e-6 && m > 0.9 && y > 0.8 && k < 0.1,
        "185 C ink: {c:?} {m:?} {y:?} {k:?}"
    );
    assert_eq!(
        pantone_display_name("185c"),
        Some("Pantone 185 C"),
        "registered plates use the kit's spelling"
    );
    assert_eq!(
        pantone_display_name("reflex blue c"),
        Some("Pantone Reflex Blue C")
    );
    assert!(pantone_cmyk("Pantone 1234 C").is_none());
    assert!(pantone_cmyk("").is_none());
    assert!(pantone_display_name("").is_none());
    // Every kit entry clamps to the 0..=1 ink range.
    for (name, cmyk) in [
        ("Pantone Green C", pantone_cmyk("green c").unwrap()),
        ("Pantone 354 C", pantone_cmyk("354 c").unwrap()),
        ("Pantone Black 2 C", pantone_cmyk("black 2 c").unwrap()),
        (
            "Pantone Cool Gray 1 C",
            pantone_cmyk("cool gray 1 c").unwrap(),
        ),
    ] {
        assert!(
            cmyk.iter().all(|c| (0.0..=1.0).contains(c)),
            "{name}: {cmyk:?}"
        );
    }
}
