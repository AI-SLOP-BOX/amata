
pub(super) const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub(super) fn base64_encode(bytes: &[u8]) -> String {

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    for chunk in bytes.chunks(3) {

        let b0 = chunk[0] as u32;

        let b1 = *chunk.get(1).unwrap_or(&0) as u32;

        let b2 = *chunk.get(2).unwrap_or(&0) as u32;

        let n = (b0 << 16) | (b1 << 8) | b2;

        out.push(B64_ALPHABET[((n >> 18) & 63) as usize] as char);

        out.push(B64_ALPHABET[((n >> 12) & 63) as usize] as char);

        out.push(if chunk.len() > 1 {

            B64_ALPHABET[((n >> 6) & 63) as usize] as char

        } else {

            '='

        });

        out.push(if chunk.len() > 2 {

            B64_ALPHABET[(n & 63) as usize] as char

        } else {

            '='

        });

    }

    out

}



pub(super) fn base64_value(c: u8) -> Option<u32> {

    match c {

        b'A'..=b'Z' => Some((c - b'A') as u32),

        b'a'..=b'z' => Some((c - b'a') as u32 + 26),

        b'0'..=b'9' => Some((c - b'0') as u32 + 52),

        b'+' => Some(62),

        b'/' => Some(63),

        _ => None,

    }

}



pub(super) fn base64_decode(s: &str) -> Option<Vec<u8>> {

    let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();

    if !clean.len().is_multiple_of(4) {

        return None;

    }

    let mut out = Vec::with_capacity(clean.len() / 4 * 3);

    for chunk in clean.chunks(4) {

        let mut n = 0u32;

        let mut pad = 0;

        for &c in chunk.iter() {

            if c == b'=' {

                pad += 1;

                n <<= 6;

            } else {

                n = (n << 6) | base64_value(c)?;

            }

        }

        if pad > 2 {

            return None;

        }

        out.push((n >> 16) as u8);

        if pad < 2 {

            out.push((n >> 8) as u8);

        }

        if pad < 1 {

            out.push(n as u8);

        }

    }

    Some(out)

}



pub(super) fn color_to_svg_str(c: &[f32; 4]) -> String {

    let r = (c[0] * 255.0) as u8;

    let g = (c[1] * 255.0) as u8;

    let b = (c[2] * 255.0) as u8;

    format!("#{r:02x}{g:02x}{b:02x}")

}



pub(super) fn xml_escape(s: &str) -> String {

    s.replace('&', "&amp;")

        .replace('<', "&lt;")

        .replace('>', "&gt;")

        .replace('"', "&quot;")

        .replace('\'', "&apos;")

}



pub fn xml_unescape(s: &str) -> String {

    s.replace("&quot;", "\"")

        .replace("&apos;", "'")

        .replace("&gt;", ">")

        .replace("&lt;", "<")

        .replace("&amp;", "&")

}
