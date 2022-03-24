use std::fmt::Write;

use askama;

use crate::PuzzlePart;


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


pub(crate) fn puzzle_string(puzzle_part: &PuzzlePart) -> askama::Result<String> {
    if let Some(raw_guesses) = &puzzle_part.raw_guesses {
        let mut ret = String::new();
        ret.push_str(&puzzle_part.head);
        ret.push_str(&raw_guesses);
        ret.push_str(&puzzle_part.tail);
        return Ok(ret);
    }

    // attempt to reconstruct
    let mut ret = String::new();
    ret.push_str(&puzzle_part.head);

    let correct_square = '\u{1F7E9}';
    let misplaced_square = if puzzle_part.site.css_class == "nerdle" { '\u{1F7EA}' } else { '\u{1F7E8}' };
    let wrong_square = '\u{2B1C}'; // or \u{2B1B} in dark mode
    for (i, (guess, _solution)) in puzzle_part.guess_lines.iter().enumerate() {
        for row_char in guess.chars() {
            if row_char == 'C' {
                ret.push(correct_square);
            } else if row_char == 'M' {
                ret.push(misplaced_square);
            } else if row_char == 'W' {
                ret.push(wrong_square);
            } else if puzzle_part.site.variant == "geo" {
                // probably the arrow behind the squares
                ret.push(row_char);
                // append the emoji variation selector
                ret.push('\u{FE0F}');
            }
        }

        if puzzle_part.site.variant != "audio" && i < puzzle_part.guess_lines.len()-1 {
            ret.push('\n');
        }
    }

    ret.push_str(&puzzle_part.tail);
    Ok(ret)
}
