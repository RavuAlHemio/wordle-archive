mod config;
mod database;
mod filters;
mod model;


use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::PathBuf;
use std::process::ExitCode;

use askama::Template;
use chrono::{Duration, Local, NaiveDate};
use clap::Parser;
use form_urlencoded;
use http_body_util::{BodyExt, Full};
use hyper::{Method, Request, Response};
use hyper::body::{Bytes, Incoming};
use hyper::service::service_fn;
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder;
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use rand::{Rng, thread_rng};
use regex::Regex;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::config::{CONFIG, CONFIG_PATH, load_config};
use crate::database::{DbConnection, OptionResult};
use crate::model::{Puzzle, PuzzleSite, SiteAndPuzzle, Stats, StatsSubject};


#[derive(Parser)]
struct Opts {
    #[clap(short, long, default_value = "config.toml")] pub config_file: PathBuf,
}


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "400.html")]
struct Error400Template {
    pub reason: String,
    pub static_prefix: String,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "403.html")]
struct Error403Template {
    pub static_prefix: String,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "404.html")]
struct Error404Template {
    pub static_prefix: String,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "no-puzzles.html")]
struct NoPuzzlesTemplate {
    pub static_prefix: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Template)]
#[template(path = "puzzles.html")]
struct PuzzlesTemplate {
    pub allow_spoiling: bool,
    pub spoil: bool,
    pub puzzles: Vec<PuzzlePart>,
    pub date_opt: Option<NaiveDate>,
    pub token: Option<String>,
    pub stats_upwards_curve: bool,
    pub static_prefix: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PuzzlePart {
    pub site: PuzzleSite,
    pub id: i64,
    pub day_ordinal: i64,
    pub head: String,
    pub tail: String,
    pub sub_puzzles: Vec<SubPuzzle>,
    pub raw_guesses: Option<String>,
    pub attempts: Option<i64>,
}


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SubPuzzle {
    pub pattern_lines: Vec<String>,
    pub solution_lines: Vec<String>,
    pub guess_lines: Vec<(String, String)>,
    pub solution: String,
    pub victory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Template)]
#[template(path = "populate.html")]
struct PopulateTemplate {
    pub sites: Vec<PuzzleSite>,
    pub solved_sites: HashSet<i64>,
    pub today_string: String,
    pub token: Option<String>,
    pub static_prefix: String,
}

#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "populate-success.html")]
struct PopulateSuccessTemplate {
    pub static_prefix: String,
}

#[derive(Clone, Debug, PartialEq, Template)]
#[template(path = "stats.html")]
struct StatsTemplate {
    pub stats: Vec<Stats>,
    pub static_prefix: String,
}


#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PuzzleData<'h, 'r, 't, 'p, 's> {
    pub head: Cow<'h, str>,
    pub raw_pattern: Cow<'r, str>,
    pub tail: Cow<'t, str>,
    pub pattern: Cow<'p, str>,
    pub solution: Cow<'s, str>,
    pub attempts: Option<usize>,
    pub expected_solution_line_count: Option<usize>,
}
impl<'h, 'r, 't, 'p, 's> PuzzleData<'h, 'r, 't, 'p, 's> {
    pub fn new<
        H: Into<Cow<'h, str>>,
        R: Into<Cow<'r, str>>,
        T: Into<Cow<'t, str>>,
        P: Into<Cow<'p, str>>,
        S: Into<Cow<'s, str>>,
    >(
        head: H,
        raw_pattern: R,
        tail: T,
        pattern: P,
        solution: S,
        attempts: Option<usize>,
        expected_solution_line_count: Option<usize>,
    ) -> Self {
        Self {
            head: head.into(),
            raw_pattern: raw_pattern.into(),
            tail: tail.into(),
            pattern: pattern.into(),
            solution: solution.into(),
            attempts,
            expected_solution_line_count,
        }
    }
}


static RESULT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(concat!(
    // squares: black, white, [red, blue, orange, yellow, green, purple, brown]
    "[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]+",
    "(?:",
        "\r?\n",
        "[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]+",
    ")*",
)).unwrap());

static GEO_RESULT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(concat!(
    // squares as above
    // arrows: right, [left, up, down], [up-left, up-right, down-left, down-right], party popper
    // emoji variant selector after the arrow (optional)
    "[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]+[\u{27A1}\u{2B05}-\u{2B07}\u{2196}-\u{2199}\u{1F389}]\u{FE0F}?",
    "(?:",
        "\r?\n",
        "[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]+[\u{27A1}\u{2B05}-\u{2B07}\u{2196}-\u{2199}\u{1F389}]\u{FE0F}?",
    ")*",
)).unwrap());

static AUDIO_RESULT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(concat!(
    // squares as above, but only one row
    // emoji variant selector optionally after each square
    "(?:[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]\u{FE0F}?)+",
)).unwrap());

static GLOBLE_RESULT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(concat!(
    // squares as above, but line breaks may be in between
    "(?:[\u{2B1B}\u{2B1C}\u{1F7E5}-\u{1F7EB}]\\s*)+",
)).unwrap());

static WORDLE32_RESULT_BLOCK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(concat!(
    // on success: twice: digit (U+0030 to U+0039), emoji variant selector (U+FE0F), enclosing keycap (U+20E3)
    // on failure: two red squares (U+1F7E5)
    // above chunk four times, separated by spaces (U+0020)
    // above chunk multiple (generally eight) times, separated by newlines (optionally U+000D, then U+000A)
    "(?:[0-9]\u{FE0F}\u{20E3}[0-9]\u{FE0F}\u{20E3}|\u{1F7E5}\u{1F7E5})",
    "(?:",
        "[ ]",
        "(?:[0-9]\u{FE0F}\u{20E3}[0-9]\u{FE0F}\u{20E3}|\u{1F7E5}\u{1F7E5})",
    "){3}",
    "(?:",
        "\r?\n",
        "(?:[0-9]\u{FE0F}\u{20E3}[0-9]\u{FE0F}\u{20E3}|\u{1F7E5}\u{1F7E5})",
        "(?:",
            "[ ]",
            "(?:[0-9]\u{FE0F}\u{20E3}[0-9]\u{FE0F}\u{20E3}|\u{1F7E5}\u{1F7E5})",
        "){3}",
    ")*",
)).unwrap());


fn return_500() -> Result<Response<Full<Bytes>>, Infallible> {
    let body = Full::new(Bytes::from("500 Internal Server Error"));
    let resp = Response::builder()
        .status(500)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body)
        .expect("failed to construct HTTP 500 response");
    Ok(resp)
}

fn return_400<S: Into<String>, P: Into<String>>(reason: S, static_prefix: P) -> Result<Response<Full<Bytes>>, Infallible> {
    let template = Error400Template {
        reason: reason.into(),
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 400, HashMap::new())
}

fn return_403<P: Into<String>>(static_prefix: P) -> Result<Response<Full<Bytes>>, Infallible> {
    let template = Error403Template {
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 403, HashMap::new())
}

fn return_404<P: Into<String>>(static_prefix: P) -> Result<Response<Full<Bytes>>, Infallible> {
    let template = Error404Template {
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 404, HashMap::new())
}

fn to_path_segments<'a>(path: &'a str, strip_trailing_empty: bool) -> Option<Vec<Cow<'a, str>>> {
    let mut segments = Vec::new();
    let pieces_percent: Vec<&str> = path.split('/').collect();
    for (i, piece_percent) in pieces_percent.iter().enumerate() {
        if piece_percent.len() == 0 {
            if i == 0 {
                continue;
            }
            if strip_trailing_empty && i == pieces_percent.len() - 1 {
                continue;
            }
        }

        let piece: Cow<str> = percent_decode_str(piece_percent)
            .decode_utf8()
            .ok()?;
        if piece == "." {
            // ignore
        } else if piece == ".." {
            // one up!
            // (returns None if no elements are left, no panics)
            segments.pop();
        } else {
            segments.push(piece);
        }
    }
    Some(segments)
}

fn get_query_pairs(uri: &hyper::Uri) -> HashMap<Cow<str>, Cow<str>> {
    if let Some(query_string) = uri.query() {
        form_urlencoded::parse(query_string.as_bytes())
            .collect()
    } else {
        HashMap::new()
    }
}

async fn has_valid_token(query_pairs: &HashMap<Cow<'_, str>, Cow<'_, str>>, value_if_no_token_configured: bool) -> bool {
    let config_guard = CONFIG
        .get().expect("CONFIG not set")
        .read().await;
    if config_guard.write_tokens.len() == 0 {
        return value_if_no_token_configured;
    }

    if let Some(token) = query_pairs.get("token") {
        if config_guard.write_tokens.iter().any(|t| t == token) {
            return true;
        }
    }

    false
}

fn render_template<T: Template>(
    template: &T,
    status: u16,
    headers: HashMap<Cow<str>, Cow<str>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let rendered = match template.render() {
        Ok(r) => r,
        Err(e) => {
            error!("error rendering template: {}", e);
            return return_500();
        },
    };
    let body = Full::new(Bytes::from(rendered));

    let mut builder = Response::builder()
        .status(status)
        .header("Content-Type", "text/html; charset=utf-8");
    for (k, v) in headers {
        builder = builder.header(k.as_ref(), v.as_ref());
    }
    let response = match builder.body(body) {
        Ok(r) => r,
        Err(e) => {
            error!("error assembling response: {}", e);
            return return_500();
        }
    };
    Ok(response)
}

fn return_internal_redirect(base_path_segs: &[Cow<str>], path: &str, code: u16) -> Result<Response<Full<Bytes>>, Infallible> {
    let mut local_url = String::new();
    for bps in base_path_segs {
        local_url.push('/');
        local_url.push_str(&bps);
    }
    local_url.push_str(path);

    let response_res = Response::builder()
        .status(code)
        .header("Location", &local_url)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(Full::new(Bytes::from(format!("Redirecting to {}", local_url))));
    match response_res {
        Ok(r) => Ok(r),
        Err(e) => {
            error!("failed to build redirect response: {}", e);
            return_500()
        },
    }
}

fn return_redirect_todays_wordle(base_path_segs: &[Cow<str>]) -> Result<Response<Full<Bytes>>, Infallible> {
    let today = Local::now().naive_local().date().format("%Y-%m-%d").to_string();
    let mut today_path = String::new();
    today_path.push_str("/wordle/");
    today_path.push_str(&today);
    return_internal_redirect(&base_path_segs, &today_path, 303)
}

async fn handle_request(req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
    let path_segs_opt = to_path_segments(req.uri().path(), false);
    let mut path_segs: Vec<String> = match path_segs_opt {
        Some(p) => p.iter().map(|s| s.clone().into_owned()).collect(),
        None => return return_404(""),
    };

    let base_path = {
        let config_guard = CONFIG
            .get().expect("CONFIG not set")
            .read().await;
        config_guard.base_path.clone()
    };
    let base_path_segs_opt = to_path_segments(&base_path, true);
    let base_path_segs = match base_path_segs_opt {
        Some(bps) => bps,
        None => {
            error!("failed to convert config base_path into segments");
            return return_500();
        },
    };

    if base_path_segs.len() > path_segs.len() {
        // path cannot be a subpath of base_path if base_path has more components
        return return_404("");
    }
    // all() returns true if the iterator is empty, e.g. if base_path_segs is empty
    let base_path_is_prefix_of_path = base_path_segs.iter()
        .zip(path_segs.iter().take(base_path_segs.len()))
        .all(|(bp, p)| bp == p);
    if !base_path_is_prefix_of_path {
        return return_404("");
    }

    // remove path prefix
    path_segs.drain(0..base_path_segs.len());

    // calculate static path prefix
    let mut static_prefix = String::new();
    if path_segs.len() > 0 {
        for _ in 0..path_segs.len()-1 {
            static_prefix.push_str("../");
        }
    }
    static_prefix.push_str("static");

    if path_segs.len() == 0 || (path_segs.len() == 1 && path_segs[0] == "") {
        // http://example.com/wordle-archive or http://example.com/wordle-archive/
        return_redirect_todays_wordle(&base_path_segs)
    } else if path_segs.len() == 1 && path_segs[0] == "wordle" {
        // http://example.com/wordle-archive/wordle
        return_redirect_todays_wordle(&base_path_segs)
    } else if path_segs.len() == 2 && path_segs[0] == "wordle" {
        if path_segs[1].len() == 0 {
            // http://example.com/wordle-archive/wordle/
            return_redirect_todays_wordle(&base_path_segs)
        } else {
            // http://example.com/wordle-archive/wordle/2022-06-16
            handle_wordle(req, static_prefix, path_segs.get(1)).await
        }
    } else if path_segs.len() == 2 && path_segs[0] == "puzzle" {
        handle_puzzle(req, static_prefix, &path_segs[1]).await
    } else if path_segs.len() == 1 && path_segs[0] == "populate" {
        handle_populate(req, static_prefix).await
    } else if path_segs.len() == 1 && path_segs[0] == "stats" {
        handle_stats(req, static_prefix).await
    } else if path_segs.len() == 2 && path_segs[0] == "static" {
        handle_static(req, static_prefix, &path_segs[1]).await
    } else {
        return_404(static_prefix)
    }
}

fn db_puzzle_to_puzzle_part(db_puzzle: &SiteAndPuzzle) -> PuzzlePart {
    let sub_puzzle_patterns: Vec<&str> = db_puzzle.puzzle.pattern
        .split("\n\n").collect();
    let solution_lines: Vec<String> = db_puzzle.puzzle.solution
        .split("\n").map(|l| l.to_owned()).collect();

    let mut sub_puzzles = Vec::with_capacity(sub_puzzle_patterns.len());
    for (i, pattern) in sub_puzzle_patterns.iter().enumerate() {
        let pattern_lines: Vec<String> = pattern
            .split("\n").map(|l| l.to_owned()).collect();
        let guess_lines = pattern_lines.iter().zip(solution_lines.iter())
            .map(|(p, s)| (p.to_owned(), s.to_owned()))
            .collect();
        let solution = solution_lines.get(solution_lines.len() - sub_puzzle_patterns.len() + i)
            .unwrap().clone();
        let victory = pattern_lines.iter().any(|ln| ln.chars().all(|c| ![ 'M', 'W', '1', '2', '3', '4', '5' ].contains(&c)));

        sub_puzzles.push(SubPuzzle {
            pattern_lines,
            solution_lines: solution_lines.clone(),
            guess_lines,
            solution,
            victory,
        });
    }

    PuzzlePart {
        site: db_puzzle.site.clone(),
        id: db_puzzle.puzzle.id,
        day_ordinal: db_puzzle.puzzle.day_ordinal,
        head: db_puzzle.puzzle.head.clone(),
        tail: db_puzzle.puzzle.tail.clone(),
        sub_puzzles,
        attempts: db_puzzle.puzzle.attempts,
        raw_guesses: db_puzzle.puzzle.raw_pattern.clone(),
    }
}

async fn check_allow_spoiling(puzzle_date: &NaiveDate) -> bool {
    let spoiler_protection_days = {
        let config_guard = CONFIG
            .get().expect("CONFIG not set")
            .read().await;
        config_guard.spoiler_protection_days
    };
    if spoiler_protection_days < 0 {
        // no spoilers, ever
        false
    } else {
        let most_recent_unprotected_day = Local::now().naive_local().date() - Duration::days(spoiler_protection_days);
        puzzle_date <= &most_recent_unprotected_day
    }
}

async fn handle_wordle<S: AsRef<str>, P: Into<String>>(
    req: Request<Incoming>,
    static_prefix: P,
    date_string_opt: Option<S>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let date_opt = match date_string_opt {
        Some(ds) => {
            match NaiveDate::parse_from_str(ds.as_ref(), "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => return return_404(static_prefix),
            }
        },
        None => None,
    };

    let mut spoil = false;
    let query_pairs = get_query_pairs(req.uri());
    if let Some(spoil_str) = query_pairs.get("spoil") {
        if let Ok(spoil_bool) = spoil_str.parse() {
            spoil = spoil_bool;
        }
    }

    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let date = match date_opt {
        Some(d) => d,
        None => {
            // get freshest date from database
            match db_conn.get_most_recent_puzzle_date().await {
                OptionResult::Present(d) => d,
                OptionResult::Absent => {
                    let template = NoPuzzlesTemplate {
                        static_prefix: static_prefix.into(),
                    };
                    return render_template(&template, 404, HashMap::new());
                },
                OptionResult::Error => return return_500(), // error already logged
            }
        },
    };

    let allow_public_spoiling = check_allow_spoiling(&date).await;
    let allow_private_spoiling = has_valid_token(&query_pairs, false).await;
    let allow_spoiling = allow_public_spoiling || allow_private_spoiling;
    if !allow_spoiling {
        spoil = false;
    }

    // obtain puzzles on that date
    let db_puzzles = match db_conn.get_puzzles_on_date(date).await {
        Some(ps) => ps,
        None => return return_500(), // error already logged
    };

    // process them
    let mut puzzles = Vec::with_capacity(db_puzzles.len());
    for db_puzzle in &db_puzzles {
        puzzles.push(db_puzzle_to_puzzle_part(db_puzzle));
    }

    let token = query_pairs.get("token").map(|t| t.clone().into_owned());
    let stats_upwards_curve: bool = thread_rng().gen();

    let template = PuzzlesTemplate {
        allow_spoiling,
        spoil,
        puzzles,
        date_opt: Some(date),
        token,
        stats_upwards_curve,
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 200, HashMap::new())
}

async fn handle_puzzle<S: AsRef<str>, P: Into<String>>(
    req: Request<Incoming>,
    static_prefix: P,
    id_string: S,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let id: i64 = match id_string.as_ref().parse() {
        Ok(i) => i,
        Err(_) => return return_404(static_prefix),
    };

    let mut spoil = false;
    let query_pairs = get_query_pairs(req.uri());
    if let Some(spoil_str) = query_pairs.get("spoil") {
        if let Ok(spoil_bool) = spoil_str.parse() {
            spoil = spoil_bool;
        }
    }

    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let db_puzzle = match db_conn.get_puzzle_by_id(id).await {
        OptionResult::Present(d) => d,
        OptionResult::Absent => return return_404(static_prefix),
        OptionResult::Error => return return_500(), // error already logged
    };
    let puzzle = db_puzzle_to_puzzle_part(&db_puzzle);

    let allow_public_spoiling = check_allow_spoiling(&db_puzzle.puzzle.date).await;
    let allow_private_spoiling = has_valid_token(&query_pairs, false).await;
    let allow_spoiling = allow_public_spoiling || allow_private_spoiling;
    if !allow_spoiling {
        spoil = false;
    }

    let token = query_pairs.get("token").map(|t| t.clone().into_owned());
    let stats_upwards_curve: bool = thread_rng().gen();

    let template = PuzzlesTemplate {
        allow_spoiling,
        spoil,
        puzzles: vec![puzzle],
        date_opt: None,
        token,
        stats_upwards_curve,
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 200, HashMap::new())
}

async fn handle_populate<P: Into<String>>(req: Request<Incoming>, static_prefix: P) -> Result<Response<Full<Bytes>>, Infallible> {
    // check for token
    let query_pairs = get_query_pairs(req.uri());
    if !has_valid_token(&query_pairs, true).await {
        return return_403(static_prefix);
    }

    if req.method() == Method::POST {
        handle_populate_post(req, static_prefix).await
    } else if req.method() == Method::GET {
        handle_populate_get(&req, static_prefix, &query_pairs).await
    } else {
        let body = Full::new(Bytes::from("invalid method; requires GET or POST"));
        let response_res = Response::builder()
            .status(405)
            .header("Content-Type", "text/plain; charset=utf-8")
            .header("Allow", "GET, POST")
            .body(body);
        match response_res {
            Ok(r) => Ok(r),
            Err(e) => {
                error!("failed to obtain 405 response: {}", e);
                return_500()
            },
        }
    }
}

async fn handle_populate_get<P: Into<String>>(
    _req: &Request<Incoming>,
    static_prefix: P,
    query_pairs: &HashMap<Cow<'_, str>, Cow<'_, str>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let sites = match db_conn.get_sites().await {
        Some(ps) => ps,
        None => return return_500(), // error already logged
    };
    let today = Local::now().naive_local().date();
    let solved_sites = match db_conn.get_solved_sites_for_date(today).await {
        Some(ss) => ss,
        None => return return_500(),
    };
    let today_string = today.format("%Y-%m-%d").to_string();
    let token = query_pairs.get("token").map(|t| t.clone().into_owned());

    let template = PopulateTemplate {
        sites,
        solved_sites,
        today_string,
        token,
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 200, HashMap::new())
}

fn decode_square(square: char, variant: &String) -> Option<char> {
    match variant.as_str() {
        "globle" => match square {
            // white => wrong
            '\u{2B1C}' => Some('W'),
            // red => 1
            '\u{1F7E5}' => Some('1'),
            // orange => 2
            '\u{1F7E7}' => Some('2'),
            // yellow => 3
            '\u{1F7E8}' => Some('3'),
            // green => correct
            '\u{1F7E9}' => Some('C'),
            c => {
                warn!("unexpected result character {:?}; ignoring", c);
                None
            },
        },
        "globlec" => match square {
            // black => wrong
            '\u{2B1B}' => Some('W'),
            // orange => 1
            '\u{1F7E7}' => Some('1'),
            // yellow => 2
            '\u{1F7E8}' => Some('2'),
            // green => 3
            '\u{1F7E9}' => Some('3'),
            // blue => 4
            '\u{1F7E6}' => Some('4'),
            // purple => 5
            '\u{1F7EA}' => Some('5'),
            // red => correct
            '\u{1F7E5}' => Some('C'),
            c => {
                warn!("unexpected result character {:?}; ignoring", c);
                None
            },
        },
        _ => match square {
            // black, white => wrong
            '\u{2B1B}'|'\u{2B1C}' => Some('W'),
            // red, orange, yellow, purple, brown => misplaced
            // (purple via Nerdle, orange and brown by assumption;
            // red from Heardle to differentiate from wrong = skipped)
            '\u{1F7E5}'|'\u{1F7E7}'|'\u{1F7E8}'|'\u{1F7EA}'|'\u{1F7EB}' => Some('M'),
            // blue, green => correct
            // (blue by assumption)
            '\u{1F7E6}'|'\u{1F7E9}' => Some('C'),
            c => {
                warn!("unexpected result character {:?}; ignoring", c);
                None
            },
        }
    }
}

async fn handle_populate_post<P: Into<String>>(
    req: Request<Incoming>,
    static_prefix: P,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let (_head, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(bs) => bs.to_bytes().to_vec(),
        Err(e) => {
            error!("failed to assemble body bytes: {}", e);
            return return_500();
        },
    };

    let mut form_pairs = HashMap::new();
    for (k, v) in form_urlencoded::parse(&body_bytes) {
        form_pairs.insert(k, v);
    }

    let site_id_str = match form_pairs.get("site") {
        Some(s) => s,
        None => return return_400("missing field \"site\"", static_prefix),
    };
    let site_id: i64 = match site_id_str.parse() {
        Ok(s) => s,
        Err(_) => return return_400("invalid value for field \"site\"", static_prefix),
    };

    let day_ordinal_str = form_pairs.get("day-ordinal")
        .map(|d| d.clone().into_owned())
        .unwrap_or_else(|| "0".to_owned());
    let day_ordinal: i64 = match day_ordinal_str.parse() {
        Ok(d) => d,
        Err(_) => return return_400("invalid value for field \"day-ordinal\"", static_prefix),
    };

    let sites = match db_conn.get_sites().await {
        Some(ps) => ps,
        None => return return_500(), // error already logged
    };

    let site = match sites.iter().filter(|s| s.id == site_id).nth(0) {
        Some(s) => s,
        None => return return_400(format!("site {} not found", site_id), static_prefix),
    };

    let result = match form_pairs.get("result") {
        Some(s) => s.replace("\r", ""),
        None => return return_400("missing field \"result\"", static_prefix),
    };
    let raw_solution = match form_pairs.get("solution") {
        Some(s) => s.replace("\r", ""),
        None => return return_400("missing field \"solution\"", static_prefix),
    };

    let puzzle_data = if site.variant == "geo" {
        if let Some(m) = GEO_RESULT_BLOCK_RE.find(&result) {
            let mut result_string = String::new();
            for line in m.as_str().split("\n") {
                if result_string.len() > 0 {
                    result_string.push('\n');
                }

                for c in line.chars() {
                    if let Some(sq) = decode_square(c, &site.variant) {
                        result_string.push(sq);
                    } else if c == '\u{FE0F}' {
                        // emoji variant selector; we don't need to store it
                    } else {
                        // probably the arrow; add it verbatim
                        result_string.push(c);
                    }
                }
            }

            let result_lines: Vec<&str> = result_string.split("\n").collect();
            let last_result_line = result_lines.last().unwrap();
            let victory =
                last_result_line.contains('C')
                && !last_result_line.chars().any(|c| [ 'M', 'W', '1', '2', '3', '4', '5' ].contains(&c))
            ;
            let expected_line_count = if victory {
                result_lines.len()
            } else {
                result_lines.len() + 1
            };
            let attempts = if victory {
                Some(expected_line_count)
            } else {
                None
            };

            let solution_line_count = raw_solution.split("\n").count();
            if expected_line_count != solution_line_count {
                return return_400(
                    format!(
                        "{} result lines, {} => expected {} solution lines but obtained {}",
                        result_lines.len(), if victory { "victory" } else { "defeat" },
                        expected_line_count, solution_line_count,
                    ),
                    static_prefix,
                );
            }

            PuzzleData::new(
                &result[0..m.start()],
                m.as_str(),
                &result[m.end()..],
                result_string,
                raw_solution.trim(),
                attempts,
                Some(expected_line_count),
            )
        } else {
            return return_400("failed to decode guesses", static_prefix);
        }
    } else if site.variant == "audio" || site.variant == "globle" || site.variant == "globlec" {
        let solution_lines: Vec<&str> = raw_solution.split('\n').collect();
        let regex = if site.variant == "audio" {
            &AUDIO_RESULT_BLOCK_RE
        } else {
            &GLOBLE_RESULT_BLOCK_RE
        };
        if let Some(m) = regex.find(&result) {
            let mut result_string = String::new();
            for c in m.as_str().chars() {
                if c == '\u{FE0F}' {
                    // emoji variant selector; skip it
                } else if let Some(sq) = decode_square(c, &site.variant) {
                    result_string.push(sq);
                    if sq == 'C' {
                        // correct answer! stop here
                        break;
                    }
                }
            }

            let victory = result_string.chars().any(|c| c == 'C');
            let expected_line_count = if victory {
                result_string.chars().count()
            } else {
                result_string.chars().count() + 1
            };
            if expected_line_count != solution_lines.len() {
                return return_400(
                    format!(
                        "{} guesses derived from result {:?}, {} solution lines; must be the same",
                        expected_line_count, result_string, solution_lines.len(),
                    ),
                    static_prefix,
                );
            }

            // intersperse newline characters in the result string
            let mut newline_result_string = String::with_capacity(result_string.len()*2);
            for c in result_string.chars() {
                if newline_result_string.len() > 0 {
                    newline_result_string.push('\n');
                }
                newline_result_string.push(c);
            }

            let attempts = if victory {
                Some(newline_result_string.bytes().filter(|b| *b == b'\n').count() + 1)
            } else {
                None
            };

            PuzzleData::new(
                &result[0..m.start()],
                m.as_str(),
                &result[m.end()..],
                newline_result_string,
                raw_solution.as_str(),
                attempts,
                Some(expected_line_count),
            )
        } else {
            return return_400("failed to decode guesses", static_prefix);
        }
    } else if site.variant == "wordle32" {
        let solution = raw_solution.trim();
        let solution_lines: Vec<&str> = solution.split('\n').collect();
        if let Some(m) = WORDLE32_RESULT_BLOCK_RE.find(&result) {
            let mut result_string = String::new();
            for line in m.as_str().split('\n') {
                if result_string.len() > 0 {
                    result_string.push('\n');
                }

                for c in line.chars() {
                    if c >= '0' && c <= '9' {
                        result_string.push(c);
                    } else if c == '\u{1F7E5}' { // red square
                        result_string.push('X');
                    } else if c == ' ' || c == '\n' {
                        result_string.push(c);
                    } else if c == '\u{FE0F}' || c == '\u{20E3}' {
                        // ignore emoji variant selectors and enclosing keycaps
                    } else {
                        return return_400(
                            format!(
                                "unknown result character {} (U+{:04X})",
                                c, u32::from(c),
                            ),
                            static_prefix,
                        );
                    }
                }
            }

            let defeat_count = result_string.chars().filter(|c| *c == 'X').count() / 2;
            let attempts = if defeat_count == 0 {
                Some(solution_lines.len())
            } else {
                None
            };

            PuzzleData::new(
                &result[0..m.start()],
                m.as_str(),
                &result[m.end()..],
                result_string,
                solution,
                attempts,
                None,
            )
        } else {
            return return_400("failed to decode guesses", static_prefix);
        }
    } else {
        // verify solution
        let solution = raw_solution.trim();
        let solution_lines: Vec<&str> = solution.split('\n').collect();

        let mut puzzles: Vec<PuzzleData> = Vec::new();
        for m in RESULT_BLOCK_RE.find_iter(&result) {
            let mut result_string = String::new();
            for line in m.as_str().split('\n') {
                if result_string.len() > 0 {
                    result_string.push('\n');
                }

                for c in line.chars() {
                    if let Some(sq) = decode_square(c, &site.variant) {
                        result_string.push(sq);
                    }
                }
            }

            // victory: is there any line that consists only of "C"s (correct answers)?
            let victory_index_opt = result_string
                .split('\n')
                .position(|ln| ln.chars().all(|c| c == 'C'));
            let expected_line_count = if let Some(victory_index) = victory_index_opt {
                victory_index + 1
            } else {
                result_string.split('\n').count()
                // +1 is added further down (adding number of lost puzzles)
            };
            let attempts = victory_index_opt.map(|vi| vi + 1);

            puzzles.push(PuzzleData::new(
                &result[0..m.start()],
                m.as_str(),
                &result[m.end()..],
                result_string,
                solution,
                attempts,
                Some(expected_line_count),
            ));
        }

        if puzzles.len() == 0 {
            return return_400("failed to decode guesses", static_prefix);
        }

        let max_expected_line_count = puzzles.iter()
            .map(|p| p.expected_solution_line_count.unwrap())
            .max().expect("no puzzles?!");
        let expected_line_count = if puzzles.iter().all(|p| p.attempts.is_some()) {
            // all puzzles won
            // => expected line count is the maximum of each subpuzzle
            max_expected_line_count
        } else {
            // some puzzles lost
            // => expected line count is the maximum of each subpuzzle
            // + 1 for each lost subpuzzle
            let lost_count = puzzles.iter()
                .filter(|p| p.attempts.is_none())
                .count();
            max_expected_line_count + lost_count
        };

        if expected_line_count != solution_lines.len() {
            return return_400(
                format!(
                    "expected {}, obtained {} solution lines",
                    expected_line_count, solution_lines.len(),
                ),
                static_prefix,
            );
        }

        let first_puzzle = &puzzles[0];
        let last_puzzle = puzzles.last().unwrap();

        let raw_patterns: Vec<Cow<str>> = puzzles.iter().map(|p| p.raw_pattern.clone()).collect();
        let raw_pattern = raw_patterns.join("\n\n");

        let patterns: Vec<Cow<str>> = puzzles.iter().map(|p| p.pattern.clone()).collect();
        let pattern = patterns.join("\n\n");

        let mut attempts = Some(0);
        for puzzle in &puzzles {
            if let Some(a) = puzzle.attempts {
                attempts = Some(attempts.unwrap().max(a));
            } else {
                // one of the partial puzzles failed = whole puzzle failed
                attempts = None;
                break;
            }
        }

        PuzzleData::new(
            first_puzzle.head.clone(),
            raw_pattern,
            last_puzzle.tail.clone(),
            pattern,
            first_puzzle.solution.clone(),
            attempts,
            Some(expected_line_count),
        )
    };

    let attempts_i64 = puzzle_data.attempts
        .map(|a| a.try_into().expect("failed to convert attempt count to i64"));

    let puzzle = Puzzle {
        id: -1,
        site_id,
        date: Local::now().naive_local().date(),
        day_ordinal,
        head: puzzle_data.head.into_owned(),
        tail: puzzle_data.tail.into_owned(),
        pattern: puzzle_data.pattern.into_owned(),
        solution: puzzle_data.solution.into_owned(),
        attempts: attempts_i64,
        raw_pattern: Some(puzzle_data.raw_pattern.into_owned()),
    };
    if !db_conn.store_puzzle(&puzzle).await {
        return_500()
    } else {
        let template = PopulateSuccessTemplate {
            static_prefix: static_prefix.into(),
        };
        render_template(&template, 200, HashMap::new())
    }
}

async fn run() -> ExitCode {
    // parse command line
    let opts = Opts::parse();

    // set up logging
    let (stdout_non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(stdout_non_blocking)
        .init();

    // store config file path
    CONFIG_PATH.set(opts.config_file)
        .expect("CONFIG_PATH already set");

    // load initial config (logs any errors using log::error!)
    let config = match load_config() {
        Some(c) => c,
        None => return ExitCode::FAILURE,
    };

    // remember listen address
    let listen_addr = config.listen_addr.clone();

    // store initial config
    CONFIG.set(RwLock::new(config))
        .expect("CONFIG already set");

    // connect to the database once (to perform any necessary migrations)
    {
        if DbConnection::new().await.is_none() {
            // error already output
            return ExitCode::FAILURE;
        }
    }
    info!("database schema is up to date");

    // hey, listen!
    let listener = TcpListener::bind(listen_addr).await
        .expect("failed to create listening socket");

    loop {
        let (stream, remote_addr) = listener.accept().await
            .expect("failed to accept incoming connection");
        tokio::task::spawn(async move {
            let result = Builder::new(TokioExecutor::new())
                .http1()
                .http2()
                .serve_connection(TokioIo::new(stream), service_fn(handle_request))
                .await;
            if let Err(e) = result {
                error!("error serving connection from {}: {}", remote_addr, e);
            }
        });
    }
}

async fn handle_stats<P: Into<String>>(_req: Request<Incoming>, static_prefix: P) -> Result<Response<Full<Bytes>>, Infallible> {
    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let stats = match db_conn.get_stats().await {
        Some(s) => s,
        None => return return_500(), // error already logged
    };

    let template = StatsTemplate {
        stats,
        static_prefix: static_prefix.into(),
    };
    render_template(&template, 200, HashMap::new())
}

async fn handle_static<P: Into<String>>(_req: Request<Incoming>, static_prefix: P, static_path: &str) -> Result<Response<Full<Bytes>>, Infallible> {
    macro_rules! typescript {
        ($basename:expr) => {
            if static_path == concat!($basename, ".js") {
                return_static(include_bytes!(concat!("../static/", $basename, ".js")), "text/javascript")
            } else if static_path == concat!($basename, ".js.map") {
                return_static(include_bytes!(concat!("../static/", $basename, ".js.map")), "application/json")
            } else if static_path == concat!($basename, ".ts") {
                return_static(include_bytes!(concat!("../static/", $basename, ".ts")), "text/vnd.typescript")
            } else {
                return_404(static_prefix)
            }
        };
    }

    if static_path == "style.css" {
        return_static(include_bytes!("../static/style.css"), "text/css")
    } else if static_path.starts_with("puzzles.") {
        typescript!("puzzles")
    } else if static_path.starts_with("wordle32-spoiler.") {
        typescript!("wordle32-spoiler")
    } else {
        return_404(static_prefix)
    }
}

fn return_static(body_bytes: &[u8], content_type: &str) -> Result<Response<Full<Bytes>>, Infallible> {
    let response_res = Response::builder()
        .status(200)
        .header("Content-Type", content_type)
        .body(Full::new(Bytes::from(body_bytes.to_vec())));
    let response = match response_res {
        Ok(r) => r,
        Err(e) => {
            error!("error assembling response: {}", e);
            return return_500();
        }
    };
    Ok(response)
}


#[tokio::main]
async fn main() -> ExitCode {
    run().await
}
