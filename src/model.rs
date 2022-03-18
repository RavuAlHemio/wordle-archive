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
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SiteAndPuzzle {
    pub site: PuzzleSite,
    pub puzzle: Puzzle,
}
