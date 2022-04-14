use std::cell::RefCell;
use std::fmt::Write;
use std::ops::Range;

use askama;

use crate::PuzzlePart;


pub(crate) struct WrongSolutionManager {
    base_guesses: usize,
    current_index: RefCell<usize>,
}
impl WrongSolutionManager {
    pub fn new(base_guesses: usize) -> Self {
        Self {
            base_guesses,
            current_index: RefCell::new(base_guesses),
        }
    }

    pub fn correct_indexes(&self) -> Range<usize> { 0..self.base_guesses }

    pub fn advance(&self) -> usize {
        let mut current_index_mut = self.current_index.borrow_mut();
        let ret = *current_index_mut;
        *current_index_mut += 1;
        ret
    }
}


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

pub(crate) fn get_index<'t, 'i, T>(slice: &'t [T], index: &'i usize) -> askama::Result<Option<&'t T>> {
    Ok(slice.get(*index))
}

pub(crate) fn make_wrong_solution_manager(puzzle: &PuzzlePart) -> askama::Result<WrongSolutionManager> {
    // calculate how many wrong guesses we have
    let wrong_guesses = puzzle.pattern_lines
        .iter()
        .flat_map(|ln| ln.split(' '))
        .filter(|chunk| *chunk == "XX")
        .count();

    // the number of base guesses is the total number of solution lines minus the number of wrong guesses
    let base_guesses = puzzle.solution_lines.len() - wrong_guesses;

    Ok(WrongSolutionManager::new(base_guesses))
}
