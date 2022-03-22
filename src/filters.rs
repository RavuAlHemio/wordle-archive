use std::fmt::Write;

use askama;


pub(crate) fn jsstring<S: AsRef<str>>(string: S) -> askama::Result<String> {
    let s = string.as_ref();
    let mut ret = String::with_capacity(s.len()+2);
    ret.push('"');
    for c in s.chars() {
        match c {
            '\u{08}' => ret.push_str("\\b"),
            '\u{09}' => ret.push_str("\\t"),
            '\u{0A}' => ret.push_str("\\n"),
            '\u{0B}' => ret.push_str("\\v"),
            '\u{0C}' => ret.push_str("\\f"),
            '\u{0D}' => ret.push_str("\\r"),
            '"' => ret.push_str("\\\""),
            '\\' => ret.push_str("\\\\"),
            // be defensive with regard to XML too
            '&' => ret.push_str("\\u0026"),
            '<' => ret.push_str("\\u003C"),
            '>' => ret.push_str("\\u003E"),
            other => {
                if other >= ' ' && other <= '~' {
                    ret.push(other);
                } else {
                    // pre-encode to UTF-16 for maximum compatibility
                    // (instead of using the "\u{123456}" syntax)
                    let mut buf = [0u16; 2];
                    for unit in other.encode_utf16(&mut buf) {
                        write!(&mut ret, "\\u{:04X}", unit).unwrap();
                    }
                }
            },
        }
    }
    ret.push('"');
    Ok(ret)
}
