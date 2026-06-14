use gpui::{div, img, px, AnyElement, IntoElement, ParentElement, SharedString, Styled};

/// A piece of a display name: plain text, or a flag rendered as an SVG image
/// (gpui paints color emoji blank on Windows, so 🇸🇪 → `assets/flags/se.svg`).
enum Seg {
    Text(String),
    Flag(&'static str),
}

/// Maps a Unicode regional-indicator codepoint (U+1F1E6–U+1F1FF) to its ASCII
/// letter, e.g. 🇸 → `S`.
fn regional_letter(c: char) -> Option<char> {
    let cp = c as u32;
    (0x1F1E6..=0x1F1FF)
        .contains(&cp)
        .then(|| (b'A' + (cp - 0x1F1E6) as u8) as char)
}

/// Splits a name into text + flag segments; regional-indicator pairs validated
/// by the `emojis` crate become flags.
fn segments(name: &str) -> Vec<Seg> {
    let chars: Vec<char> = name.chars().collect();
    let mut out: Vec<Seg> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let (Some(a), Some(b)) = (
            regional_letter(chars[i]),
            chars.get(i + 1).copied().and_then(regional_letter),
        ) {
            let flag: String = [chars[i], chars[i + 1]].iter().collect();
            if let Some(code) = emojis::get(&flag).map(|_| iso_to_code(a, b)) {
                if !buf.is_empty() {
                    out.push(Seg::Text(std::mem::take(&mut buf)));
                }
                out.push(Seg::Flag(code));
                i += 2;
                continue;
            }
        }
        buf.push(chars[i]);
        i += 1;
    }
    if !buf.is_empty() {
        out.push(Seg::Text(buf));
    }
    out
}

/// Lowercases the two flag letters into a static asset basename (`se`, `us`, …),
/// or `""` if we ship no SVG for that code.
fn iso_to_code(a: char, b: char) -> &'static str {
    CODES
        .iter()
        .find(|c| c.as_bytes() == [a.to_ascii_lowercase() as u8, b.to_ascii_lowercase() as u8])
        .copied()
        .unwrap_or("")
}

/// Whether `name` contains at least one renderable flag.
pub(crate) fn has_flag(name: &str) -> bool {
    segments(name)
        .iter()
        .any(|s| matches!(s, Seg::Flag(c) if !c.is_empty()))
}

/// Renders a display name with inline flag images. Falls back to plain text when
/// the name has no flags, so callers keep their existing truncation behaviour.
pub(crate) fn render_name(name: &str) -> AnyElement {
    if !has_flag(name) {
        return name.to_string().into_any_element();
    }
    let mut row = gpui_component::h_flex()
        .items_center()
        .flex_wrap()
        .gap(px(5.));
    for seg in segments(name) {
        row = match seg {
            Seg::Text(t) => row.child(t),
            Seg::Flag(code) if !code.is_empty() => row.child(
                img(SharedString::from(format!("flags/{code}.svg")))
                    .h(px(11.))
                    .w(px(15.))
                    .rounded(px(2.)),
            ),
            Seg::Flag(_) => row,
        };
    }
    div().child(row).into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codes(name: &str) -> Vec<&'static str> {
        segments(name)
            .into_iter()
            .filter_map(|s| match s {
                Seg::Flag(c) => Some(c),
                Seg::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn extracts_flags() {
        assert_eq!(codes("🇸🇪 Stockholm 01"), vec!["se"]);
        assert_eq!(codes("🇺🇸 US · 🇯🇵 JP"), vec!["us", "jp"]);
        assert!(codes("Direct").is_empty());
        assert!(has_flag("🇩🇪 Frankfurt"));
        assert!(!has_flag("auto-fallback"));
    }
}

/// Lowercase codes we ship a flag SVG for (flag-icons 4x3, ISO 3166-1 alpha-2
/// plus a few exceptional reservations). Generated from `assets/flags/`.
const CODES: &[&str] = &[
    "ad", "ae", "af", "ag", "ai", "al", "am", "ao", "aq", "ar", "as", "at", "au", "aw", "ax", "az",
    "ba", "bb", "bd", "be", "bf", "bg", "bh", "bi", "bj", "bl", "bm", "bn", "bo", "bq", "br", "bs",
    "bt", "bv", "bw", "by", "bz", "ca", "cc", "cd", "cf", "cg", "ch", "ci", "ck", "cl", "cm", "cn",
    "co", "cp", "cr", "cu", "cv", "cw", "cx", "cy", "cz", "de", "dg", "dj", "dk", "dm", "do", "dz",
    "ec", "ee", "eg", "eh", "er", "es", "et", "eu", "fi", "fj", "fk", "fm", "fo", "fr", "ga", "gb",
    "gd", "ge", "gf", "gg", "gh", "gi", "gl", "gm", "gn", "gp", "gq", "gr", "gs", "gt", "gu", "gw",
    "gy", "hk", "hm", "hn", "hr", "ht", "hu", "ic", "id", "ie", "il", "im", "in", "io", "iq", "ir",
    "is", "it", "je", "jm", "jo", "jp", "ke", "kg", "kh", "ki", "km", "kn", "kp", "kr", "kw", "ky",
    "kz", "la", "lb", "lc", "li", "lk", "lr", "ls", "lt", "lu", "lv", "ly", "ma", "mc", "md", "me",
    "mf", "mg", "mh", "mk", "ml", "mm", "mn", "mo", "mp", "mq", "mr", "ms", "mt", "mu", "mv", "mw",
    "mx", "my", "mz", "na", "nc", "ne", "nf", "ng", "ni", "nl", "no", "np", "nr", "nu", "nz", "om",
    "pa", "pc", "pe", "pf", "pg", "ph", "pk", "pl", "pm", "pn", "pr", "ps", "pt", "pw", "py", "qa",
    "re", "ro", "rs", "ru", "rw", "sa", "sb", "sc", "sd", "se", "sg", "sh", "si", "sj", "sk", "sl",
    "sm", "sn", "so", "sr", "ss", "st", "sv", "sx", "sy", "sz", "tc", "td", "tf", "tg", "th", "tj",
    "tk", "tl", "tm", "tn", "to", "tr", "tt", "tv", "tw", "tz", "ua", "ug", "um", "un", "us", "uy",
    "uz", "va", "vc", "ve", "vg", "vi", "vn", "vu", "wf", "ws", "xk", "xx", "ye", "yt", "za", "zm",
    "zw",
];
