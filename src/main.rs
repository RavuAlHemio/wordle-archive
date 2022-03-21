mod config;
mod database;
mod model;


use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::Infallible;
use std::path::PathBuf;

use askama::Template;
use chrono::{Duration, Local, NaiveDate};
use clap::Parser;
use env_logger;
use form_urlencoded;
use hyper::{Body, Method, Request, Response};
use hyper::service::{make_service_fn, service_fn};
use log::{error, warn};
use once_cell::sync::Lazy;
use percent_encoding::percent_decode_str;
use regex::Regex;
use tokio::sync::RwLock;

use crate::config::{CONFIG, CONFIG_PATH, load_config};
use crate::database::{DbConnection, OptionResult};
use crate::model::{Puzzle, PuzzleSite, SiteAndPuzzle};


#[derive(Parser)]
struct Opts {
    #[clap(short, long, default_value = "config.toml")] pub config_file: PathBuf,
}


#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "400.html")]
struct Error400Template {
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "403.html")]
struct Error403Template;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "404.html")]
struct Error404Template;

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "no-puzzles.html")]
struct NoPuzzlesTemplate;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Template)]
#[template(path = "puzzles.html")]
struct PuzzlesTemplate {
    pub allow_spoiling: bool,
    pub spoil: bool,
    pub puzzles: Vec<PuzzlePart>,
    pub date_opt: Option<NaiveDate>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct PuzzlePart {
    pub site: PuzzleSite,
    pub id: i64,
    pub day_ordinal: i64,
    pub head: String,
    pub tail: String,
    pub guess_lines: Vec<(String, String)>,
    pub solved: bool,
    pub solution: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Template)]
#[template(path = "populate.html")]
struct PopulateTemplate {
    pub sites: Vec<PuzzleSite>,
    pub solved_sites: HashSet<i64>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Template)]
#[template(path = "populate-success.html")]
struct PopulateSuccessTemplate;


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


fn return_500() -> Result<Response<Body>, Infallible> {
    let body = Body::from("500 Internal Server Error");
    let resp = Response::builder()
        .status(500)
        .header("Content-Type", "text/plain; charset=utf-8")
        .body(body)
        .expect("failed to construct HTTP 500 response");
    Ok(resp)
}

fn return_400<S: Into<String>>(reason: S) -> Result<Response<Body>, Infallible> {
    let template = Error400Template {
        reason: reason.into(),
    };
    render_template(&template, 400, HashMap::new())
}

fn return_403() -> Result<Response<Body>, Infallible> {
    render_template(&Error403Template, 403, HashMap::new())
}

fn return_404() -> Result<Response<Body>, Infallible> {
    render_template(&Error404Template, 404, HashMap::new())
}

fn to_path_segments<'a>(path: &'a str) -> Option<Vec<Cow<'a, str>>> {
    let mut segments = Vec::new();
    for piece_percent in path.split("/") {
        if piece_percent.len() == 0 {
            continue;
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
) -> Result<Response<Body>, Infallible> {
    let rendered = match template.render() {
        Ok(r) => r,
        Err(e) => {
            error!("error rendering template: {}", e);
            return return_500();
        },
    };
    let body = Body::from(rendered);

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

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let path_segs_opt = to_path_segments(req.uri().path());
    let mut path_segs: Vec<String> = match path_segs_opt {
        Some(p) => p.iter().map(|s| s.clone().into_owned()).collect(),
        None => return return_404(),
    };

    let base_path = {
        let config_guard = CONFIG
            .get().expect("CONFIG not set")
            .read().await;
        config_guard.base_path.clone()
    };
    let base_path_segs_opt = to_path_segments(&base_path);
    let base_path_segs = match base_path_segs_opt {
        Some(bps) => bps,
        None => {
            error!("failed to convert config base_path into segments");
            return return_500();
        },
    };

    if base_path_segs.len() > path_segs.len() {
        // path cannot be a subpath of base_path if base_path has more components
        return return_404();
    }
    // all() returns true if the iterator is empty, e.g. if base_path_segs is empty
    let base_path_is_prefix_of_path = base_path_segs.iter()
        .zip(path_segs.iter().take(base_path_segs.len()))
        .all(|(bp, p)| bp == p);
    if !base_path_is_prefix_of_path {
        return return_404();
    }

    // remove path prefix
    path_segs.drain(0..base_path_segs.len());

    if path_segs.len() == 0 {
        // redirect to today's wordle
        let today = Local::now().date().naive_local().format("%Y-%m-%d").to_string();
        let mut today_url = String::new();
        for bps in base_path_segs {
            today_url.push('/');
            today_url.push_str(&bps);
        }
        today_url.push_str("/wordle/");
        today_url.push_str(&today);

        let response_res = Response::builder()
            .status(303)
            .header("Location", &today_url)
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(Body::from(format!("Redirecting to {}", today_url)));
        match response_res {
            Ok(r) => Ok(r),
            Err(e) => {
                error!("failed to build redirect response: {}", e);
                return_500()
            },
        }
    } else if path_segs.len() > 0 && path_segs[0] == "wordle" {
        handle_wordle(req, path_segs.get(1)).await
    } else if path_segs.len() == 2 && path_segs[0] == "puzzle" {
        handle_puzzle(req, &path_segs[1]).await
    } else if path_segs.len() > 0 && path_segs[0] == "populate" {
        handle_populate(req).await
    } else {
        return_404()
    }
}

fn db_puzzle_to_puzzle_part(db_puzzle: &SiteAndPuzzle) -> PuzzlePart {
    let pattern_lines: Vec<&str> = db_puzzle.puzzle.pattern.split("\n").collect();
    let solution_lines: Vec<&str> = db_puzzle.puzzle.solution.split("\n").collect();

    let guess_lines = pattern_lines.iter().zip(solution_lines.iter())
        .map(|(&p, &s)| (p.to_owned(), s.to_owned()))
        .collect();

    PuzzlePart {
        site: db_puzzle.site.clone(),
        id: db_puzzle.puzzle.id,
        day_ordinal: db_puzzle.puzzle.day_ordinal,
        head: db_puzzle.puzzle.head.clone(),
        tail: db_puzzle.puzzle.tail.clone(),
        guess_lines,
        solved: pattern_lines.len() == solution_lines.len(),
        solution: (*solution_lines.last().unwrap()).to_owned(),
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
        let most_recent_unprotected_day = Local::today().naive_local() - Duration::days(spoiler_protection_days);
        puzzle_date <= &most_recent_unprotected_day
    }
}

async fn handle_wordle<S: AsRef<str>>(
    req: Request<Body>,
    date_string_opt: Option<S>,
) -> Result<Response<Body>, Infallible> {
    let date_opt = match date_string_opt {
        Some(ds) => {
            match NaiveDate::parse_from_str(ds.as_ref(), "%Y-%m-%d") {
                Ok(d) => Some(d),
                Err(_) => return return_404(),
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
                    let template = NoPuzzlesTemplate;
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

    let template = PuzzlesTemplate {
        allow_spoiling,
        spoil,
        puzzles,
        date_opt: Some(date),
    };
    render_template(&template, 200, HashMap::new())
}

async fn handle_puzzle<S: AsRef<str>>(
    req: Request<Body>,
    id_string: S,
) -> Result<Response<Body>, Infallible> {
    let id: i64 = match id_string.as_ref().parse() {
        Ok(i) => i,
        Err(_) => return return_404(),
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
        OptionResult::Absent => return return_404(),
        OptionResult::Error => return return_500(), // error already logged
    };
    let puzzle = db_puzzle_to_puzzle_part(&db_puzzle);

    let allow_public_spoiling = check_allow_spoiling(&db_puzzle.puzzle.date).await;
    let allow_private_spoiling = has_valid_token(&query_pairs, false).await;
    let allow_spoiling = allow_public_spoiling || allow_private_spoiling;
    if !allow_spoiling {
        spoil = false;
    }

    let template = PuzzlesTemplate {
        allow_spoiling,
        spoil,
        puzzles: vec![puzzle],
        date_opt: None,
    };
    render_template(&template, 200, HashMap::new())
}

async fn handle_populate(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // check for token
    let query_pairs = get_query_pairs(req.uri());
    if !has_valid_token(&query_pairs, true).await {
        return return_403();
    }

    if req.method() == Method::POST {
        handle_populate_post(req).await
    } else if req.method() == Method::GET {
        handle_populate_get(req).await
    } else {
        let body = Body::from("invalid method; requires GET or POST");
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

async fn handle_populate_get(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let sites = match db_conn.get_sites().await {
        Some(ps) => ps,
        None => return return_500(), // error already logged
    };
    let today = Local::today().naive_local();
    let solved_sites = match db_conn.get_solved_sites_for_date(today).await {
        Some(ss) => ss,
        None => return return_500(),
    };

    let template = PopulateTemplate {
        sites,
        solved_sites,
    };
    render_template(&template, 200, HashMap::new())
}

fn decode_square(square: char) -> Option<char> {
    match square {
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

async fn handle_populate_post(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let db_conn = match DbConnection::new().await {
        Some(c) => c,
        None => return return_500(), // error already logged
    };

    let (_head, body) = req.into_parts();
    let body_bytes = match hyper::body::to_bytes(body).await {
        Ok(bs) => bs.to_vec(),
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
        None => return return_400("missing field \"site\""),
    };
    let site_id: i64 = match site_id_str.parse() {
        Ok(s) => s,
        Err(_) => return return_400("invalid value for field \"site\""),
    };

    let day_ordinal_str = form_pairs.get("day-ordinal")
        .map(|d| d.clone().into_owned())
        .unwrap_or_else(|| "0".to_owned());
    let day_ordinal: i64 = match day_ordinal_str.parse() {
        Ok(d) => d,
        Err(_) => return return_400("invalid value for field \"day-ordinal\""),
    };

    let sites = match db_conn.get_sites().await {
        Some(ps) => ps,
        None => return return_500(), // error already logged
    };

    let site = match sites.iter().filter(|s| s.id == site_id).nth(0) {
        Some(s) => s,
        None => return return_400(format!("site {} not found", site_id)),
    };

    let result = match form_pairs.get("result") {
        Some(s) => s.replace("\r", ""),
        None => return return_400("missing field \"result\""),
    };
    let raw_solution = match form_pairs.get("solution") {
        Some(s) => s.replace("\r", "").to_owned(),
        None => return return_400("missing field \"solution\""),
    };

    let (head, tail, pattern, solution) = if site.variant == "geo" {
        if let Some(m) = GEO_RESULT_BLOCK_RE.find(&result) {
            let mut result_string = String::new();
            for line in m.as_str().split("\n") {
                if result_string.len() > 0 {
                    result_string.push('\n');
                }

                for c in line.chars() {
                    if let Some(sq) = decode_square(c) {
                        result_string.push(sq);
                    } else if c == '\u{FE0F}' {
                        // emoji variant selector; we don't need to store it
                    } else {
                        // probably the arrow; add it verbatim
                        result_string.push(c);
                    }
                }
            }
            (&result[0..m.start()], &result[m.end()..], result_string, raw_solution.trim())
        } else {
            return return_400("failed to decode guesses");
        }
    } else if site.variant == "audio" {
        let solution_lines: Vec<&str> = raw_solution.split('\n').collect();
        if let Some(m) = AUDIO_RESULT_BLOCK_RE.find(&result) {
            let mut result_string = String::new();
            for c in m.as_str().chars() {
                if c == '\u{FE0F}' {
                    // emoji variant selector; skip it
                } else if let Some(sq) = decode_square(c) {
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
                return return_400(format!(
                    "{} guesses derived from result {:?}, {} solution lines; must be the same",
                    expected_line_count, result_string, solution_lines.len(),
                ));
            }

            // intersperse newline characters in the result string
            let mut newline_result_string = String::with_capacity(result_string.len()*2);
            for c in result_string.chars() {
                if newline_result_string.len() > 0 {
                    newline_result_string.push('\n');
                }
                newline_result_string.push(c);
            }

            (&result[0..m.start()], &result[m.end()..], newline_result_string, raw_solution.as_str())
        } else {
            return return_400("failed to decode guesses");
        }
    } else {
        // verify solution
        let solution = raw_solution.trim();
        let solution_lines: Vec<&str> = solution.split('\n').collect();
        if let Some(m) = RESULT_BLOCK_RE.find(&result) {
            let mut result_string = String::new();
            for line in m.as_str().split('\n') {
                if result_string.len() > 0 {
                    result_string.push('\n');
                }

                for c in line.chars() {
                    if let Some(sq) = decode_square(c) {
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
                result_string.split('\n').count() + 1
            };

            if expected_line_count != solution_lines.len() {
                return return_400(format!(
                    "calculated {} result lines, obtained {} solution lines",
                    expected_line_count, solution_lines.len(),
                ));
            }

            (&result[0..m.start()], &result[m.end()..], result_string, solution)
        } else {
            return return_400("failed to decode guesses");
        }
    };

    let puzzle = Puzzle {
        id: -1,
        site_id,
        date: Local::now().date().naive_local(),
        day_ordinal,
        head: head.to_owned(),
        tail: tail.to_owned(),
        pattern,
        solution: solution.to_string(),
    };
    if !db_conn.store_puzzle(&puzzle).await {
        return_500()
    } else {
        render_template(&PopulateSuccessTemplate, 200, HashMap::new())
    }
}

async fn run() -> i32 {
    // parse command line
    let opts = Opts::parse();

    // set up logging
    env_logger::init();

    // store config file path
    CONFIG_PATH.set(opts.config_file)
        .expect("CONFIG_PATH already set");

    // load initial config (logs any errors using log::error!)
    let config = match load_config() {
        Some(c) => c,
        None => return 1,
    };

    // remember listen address
    let listen_addr = config.listen_addr.clone();

    // store initial config
    CONFIG.set(RwLock::new(config))
        .expect("CONFIG already set");

    // hey, listen!
    let make_service = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });
    let server = hyper::Server::bind(&listen_addr)
        .serve(make_service);

    // keep going
    if let Err(e) = server.await {
        error!("server error: {}", e);
        1
    } else {
        0
    }
}


#[tokio::main]
async fn main() {
    std::process::exit(run().await)
}
