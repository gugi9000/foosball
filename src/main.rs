#![feature(decl_macro, proc_macro_hygiene)]
#[macro_use]
extern crate tera;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate rocket;
#[macro_use] extern crate rocket_contrib;
#[macro_use] extern crate diesel;

use std::{
    path::{Path, PathBuf},
    io::Read,
    cmp::Ordering::{Greater, Less},
    ops::Deref,
    collections::HashMap,
    sync::{Arc, Mutex}
};
use rocket::{
    request::{Request, FromFormValue, Form, Outcome, FromRequest, State},
    response::{Content, NamedFile, Response, Responder, Redirect},
    http::{RawStr, Status, ContentType}
};
use diesel::query_dsl::RunQueryDsl;
use tera::{Tera, Context, Value};
use bbt::{Rater, Rating};
use lazy_static::*;

pub mod model;
pub mod schema;

mod balls;
mod errors;
mod games;
mod players;
mod statics;
mod ratings;
mod pvp;
mod ratingsdev;

const BETA: f64 = 5000.0;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

pub type Res<'a> = Result<Response<'a>, Status>;
pub type ContRes<'a> = Content<Res<'a>>;

struct IgnoreField;

impl<'a> FromFormValue<'a> for IgnoreField {
    type Error = &'a str;

    fn from_form_value(_: &'a RawStr) -> Result<Self, Self::Error> {
        Ok(IgnoreField)
    }

    fn default() -> Option<Self> {
        Some(IgnoreField)
    }
}

pub enum Resp<'a> {
    ContRes(ContRes<'a>),
    Redirect(Redirect)
}

impl<'a> Resp<'a> {
    pub fn cont(cont: ContRes<'a>) -> Self {
        Resp::ContRes(cont)
    }
    pub fn red(red: Redirect) -> Self {
        Resp::Redirect(red)
    }
}

impl<'a> Responder<'a> for Resp<'a> {
    fn respond_to(self, req: &Request) -> Res<'a> {
        match self {
            Resp::ContRes(a) => a.respond_to(req),
            Resp::Redirect(a) => a.respond_to(req),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    title: String,
    secret: String,
    ace_egg_modifier: f64,
    streak_modifier: f64,
}

fn egg_filter(value: Value, _args: HashMap<String, Value>) -> tera::Result<Value> {
    let goals = try_get_value!("egg", "value", i32, value);
    if goals == 0 {
        Ok(Value::String("🥚".to_owned()))
    } else {
        Ok(value)
    }
}

fn da_genitive_filter(value: Value, _args: HashMap<String, Value>) -> tera::Result<Value> {
    let mut name = try_get_value!("genitiv", "value", String, value);
    match name.chars().last() {
        Some('s') | Some('x') | Some('z') => name.push('\''),
        _ => name.push('s')
    }
    Ok(Value::String(name))
}

fn abs_filter(value: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
    let num = try_get_value!("abs", "value", i32, value);
    Ok(num.abs().into())
}

lazy_static! {
    static ref TERA: Tera = {
        let mut tera = compile_templates!("templates/**/*");
        tera.autoescape_on(vec![]);
        tera.register_filter("egg", egg_filter);
        tera.register_filter("abs", abs_filter);
        tera.register_filter("genitiv", da_genitive_filter);
        tera
    };
    pub static ref CONFIG: Config = {
        let mut file = match std::fs::File::open("Foosball.toml") {
            Ok(f) => f,
            Err(e) => {
                let code = e.raw_os_error().unwrap_or(1);
                drop(e);
                println!("Couldn't open 'Foosball.toml'. Perhaps you didn't create one.\n\
                          Look at 'Foosball.toml.sample' for an example.");
                std::process::exit(code);
            }
        };

        let mut buf = String::new();
        file.read_to_string(&mut buf).unwrap();

        toml::from_str(&buf).unwrap()
    };
    static ref RATER: Rater = Rater::new(BETA / 6.0);
    pub static ref LAST_DATE: Mutex<Option<String>> = Mutex::new(None);
    static ref BASE_CONTEXT: Context = {
        let mut c = Context::new();
        c.insert("version", &VERSION);
        c.insert("league", &CONFIG.title);
        c
    };
}

pub fn tera_render(template: &str, c: Context) -> Res<'static> {
    use std::io::Cursor;
    match TERA.render(template, &c) {
        Ok(s) => Response::build().sized_body(Cursor::new(s)).ok(),
        Err(_) => Err(Status::InternalServerError)
    }
}

pub fn respond_page(page: &'static str, c: Context) -> ContRes<'static> {
    Content(ContentType::HTML, tera_render(&format!("pages/{}.html", page), c))
}

fn gen_players(db_conn: &DbConn) -> HashMap<i32, PlayerRating> {
    use model::Player;

    let mut players = HashMap::new();

    for Player{id, name} in Player::read_all(db_conn) {
        players.insert(id, PlayerRating::new(name));
    }

    players
}

pub struct Game {
    home: i32,
    away: i32,
    dato: DateTime,
    ace: bool,
    home_win: bool,
}

fn get_games<'a>(conn: &DbConn) -> Vec<Game> {
    let last_date = LAST_DATE.lock().unwrap();

    if let Some(last_date) = &*last_date {
        model::GameForScores::read_all(conn, last_date)
    } else {
        model::GameForScores::read_all_from_start_of_month(conn)
    }.into_iter()
    .map(|gfs| Game {
        home: gfs.home_id,
        away: gfs.away_id,
        dato: gfs.dato,
        ace: gfs.home_score == 0 || gfs.away_score == 0,
        home_win: gfs.home_score > gfs.away_score,
    })
    .collect()
}

#[derive(Debug, Serialize)]
struct PlayedGame {
    home: String,
    away: String,
    home_score: i32,
    away_score: i32,
    ball: String,
    ball_name: String,
    dato: String,
}

pub fn create_context(current_page: &str) -> Context {
    let mut c = BASE_CONTEXT.clone();
    c.insert("cur", &current_page);
    c
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregatedRating {
    #[serde(flatten)]
    rating: Rating,
    score: f64
}

impl AggregatedRating {
    fn from_player(p: &PlayerRating) -> Self {
        let streak = p.streak as f64;
        let modifier = CONFIG.ace_egg_modifier * (p.aces as f64 - p.eggs as f64)
            + CONFIG.streak_modifier * (streak - streak.signum());
        AggregatedRating {
            rating: p.rating.clone(),
            score: p.rating.mu() - 3. * p.rating.sigma() + modifier
        }
    }
}

type DateTime = String;

#[derive(Debug, Clone)]
pub struct PlayerRating {
    pub name: String,
    pub score_history: Vec<(DateTime, f64)>,
    pub streak: i16,
    pub rating: Rating,
    pub kampe: u32,
    pub vundne: u32,
    pub tabte: u32,
    pub eggs: u32,
    pub aces: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayerData {
    pub name: String,
    pub streak: i16,
    pub rating: AggregatedRating,
    pub kampe: u32,
    pub vundne: u32,
    pub tabte: u32,
    pub eggs: u32,
    pub aces: u32,
}

use std::mem::replace;
use bbt::Outcome::{Win, Loss};

impl PlayerRating {
    fn new<S: ToString>(name: S) -> Self {
        PlayerRating {
            name: name.to_string(),
            rating: Rating::new(BETA, BETA / 3.0),
            score_history: Vec::new(),
            kampe: 0,
            streak: 0,
            vundne: 0,
            tabte: 0,
            eggs: 0,
            aces: 0,
        }
    }
    fn mod_streak(&mut self, won: bool) {
        use std::cmp::Ordering::*;
        match (self.streak.cmp(&0), won) {
            (Less, false) => self.streak -= 1,
            (Greater, true) => self.streak += 1,
            (_, false)  => self.streak = -1,
            (_, true) => self.streak = 1,
        }
    }
    fn duel(&mut self, time: DateTime, o: Rating, won: bool) {
        let a = replace(&mut self.rating, Default::default());

        let (a, _) = RATER.duel(a, o, if won { Win } else { Loss });
        self.rating = a;

        self.kampe += 1;
        self.mod_streak(won);
        if won {
            self.vundne += 1;
        } else {
            self.tabte += 1;
        }
        let rat = AggregatedRating::from_player(&self);
        self.score_history.push((time, rat.score));
    }
    fn to_data(&self) -> PlayerData {
        let rating = AggregatedRating::from_player(self);
        PlayerData {
            name: self.name.clone(),
            rating,
            kampe: self.kampe,
            streak: self.streak,
            vundne: self.vundne,
            tabte: self.tabte,
            eggs: self.eggs,
            aces: self.aces
        }
    }
}

#[database("ratings")]
pub struct DbConn(pub diesel::SqliteConnection);

pub type PlayersMap = HashMap<i32, PlayerRating>;

pub struct Players(pub Arc<Mutex<PlayersMap>>);

impl FromRequest<'_, '_> for Players {
    type Error = ();
    fn from_request(request: &Request) -> Outcome<Self, Self::Error> {
        let players: State<Self> = request.guard()?;

        Outcome::Success(Players(players.inner().0.clone()))
    }
}

impl Deref for Players {
    type Target = Mutex<PlayersMap>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Players {
    pub fn reset(&self, db_conn: &DbConn) {
        *self.0.lock().unwrap() = gen_players(db_conn);
        *LAST_DATE.lock().unwrap() = None;
    }
}

fn main() {
    use crate::errors::*;
    &*CONFIG;
    let rocket = rocket::ignite()
        .mount("/",
               routes![crate::ratingsdev::ratingsdev,
                       crate::ratingsdev::developmenttsv,
                       crate::pvp::pvp,
                       crate::pvp::pvpindex,
                       crate::statics::robots_handler,
                       crate::statics::favicon_handler,
                       crate::games::games,
                       crate::games::newgame,
                       crate::games::submit_newgame,
                       crate::balls::balls,
                       crate::balls::ball,
                       crate::balls::newball,
                       crate::balls::submit_newball,
                       crate::players::players,
                       crate::players::player,
                       crate::players::newplayer,
                       crate::players::submit_newplayer,
                       crate::ratings::ratings,
                       crate::ratings::reset,
                       crate::ratings::root,
                       crate::statics::static_handler])
        .register(catchers![page_not_found, bad_request, server_error])
        .attach(DbConn::fairing());
    {
        let db_conn = DbConn::get_one(&rocket).expect("DbConn attached");

        diesel::sql_query("PRAGMA foreign_keys = ON;").execute(&*db_conn).unwrap();

        let players = gen_players(&db_conn);

        rocket.manage(Players(Arc::new(Mutex::new(players))))
    }.launch();
}
