use chrono::NaiveDate;


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PuzzleSite {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub css_class: String,
    pub variant: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct Puzzle {
    pub id: i64,
    pub site_id: i64,
    pub date: NaiveDate,
    pub day_ordinal: i64,
    pub head: String,
    pub tail: String,
    pub pattern: String,
    pub solution: String,
    pub attempts: Option<i64>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SiteAndPuzzle {
    pub site: PuzzleSite,
    pub puzzle: Puzzle,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SiteStats {
    pub site: Option<PuzzleSite>,
    pub puzzles_won: i64,
    pub puzzles_lost: i64,
    pub average_attempts: f64,
}
impl SiteStats {
    pub fn percent_won(&self) -> f64 {
        if self.puzzles_won + self.puzzles_lost == 0 {
            0.0
        } else {
            (self.puzzles_won as f64) * 100.0 / ((self.puzzles_won + self.puzzles_lost) as f64)
        }
    }
}
