use std::{
    fs::File,
    path::{Path, PathBuf},
    io::Read,
    cmp::Ordering::{Greater, Less},
    collections::HashMap,
    sync::{Mutex, MutexGuard}
};
use rocket::{
    catchers, form::{self, Form, FromFormField, ValueField}, fs::NamedFile, http::Status, launch, request::Request, response::{content::RawHtml, Redirect, Responder, Response}, routes,
};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tera::{try_get_value, Context, Tera, Value};
use bbt::{Rater, Rating};
use lazy_static::*;

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
const INITIAL_DATE_CAP: &'static str = "now','start of month";

pub type Res<T> = Result<T, Status>;
pub type ResHtml = Res<RawHtml<String>>;

struct IgnoreField;

impl FromFormField<'_> for IgnoreField {
    fn from_value(_: ValueField) -> form::Result<Self> {
        Ok(IgnoreField)
    }

    fn default() -> Option<Self> {
        Some(IgnoreField)
    }
}

pub enum Resp<T> {
    ContRes(Result<T, Status>),
    Redirect(Redirect)
}

impl<T> Resp<T> {
    pub fn cont(cont: Result<T, Status>) -> Self {
        Resp::ContRes(cont)
    }
    pub fn red(red: Redirect) -> Self {
        Resp::Redirect(red)
    }
}

impl<'r, 'o: 'r, T: Responder<'r, 'o>> Responder<'r, 'o> for Resp<T> {
    fn respond_to(self, req: &'r Request) -> Res<Response<'o>> {
        match self {
            Resp::ContRes(a) => a.respond_to(req),
            Resp::Redirect(a) => a.respond_to(req),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Config {
    database: String,
    title: String,
    secret: String,
    ace_egg_modifier: f64,
    streak_modifier: f64,
}

fn egg_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let goals = try_get_value!("egg", "value", i32, value);
    if goals == 0 {
        Ok(Value::String("🥚".to_owned()))
    } else {
        Ok(value.clone())
    }
}

fn da_genitive_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let mut name = try_get_value!("genitiv", "value", String, value);
    match name.chars().last() {
        Some('s') | Some('x') | Some('z') => name.push('\''),
        _ => name.push('s')
    }
    Ok(Value::String(name))
}

fn abs_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
    let num = try_get_value!("abs", "value", i32, value);
    Ok(num.abs().into())
}

lazy_static! {
    static ref TERA: Tera = {
        let mut tera = Tera::new("templates/**/*").unwrap();
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
    pub static ref PLAYERS: Mutex<HashMap<i32, PlayerRating>> = Mutex::new(gen_players());
    pub static ref LAST_DATE: Mutex<String> = Mutex::new(INITIAL_DATE_CAP.to_owned());
    // Has to be a `Mutex` because `Connection` isn't `Sync`
    pub static ref DB_CONNECTION: Mutex<Connection> = Mutex::new({
        let exists = match File::open(&CONFIG.database) {
            Ok(_) => true,
            Err(_) => false,
        };
        let conn = Connection::open(&CONFIG.database).unwrap();
        if ! exists {
            println!("Database didn't exist... creating one");
            conn.execute_batch(include_str!("ratings.schema")).unwrap();
        }
        conn
    });
    static ref BASE_CONTEXT: Context = {
        let mut c = Context::new();
        c.insert("version", &VERSION);
        c.insert("league", &CONFIG.title);
        c
    };
}

pub fn tera_render(template: &'_ str, c: Context) -> Res<RawHtml<String>> {
    match TERA.render(template, &c) {
        Ok(s) => Ok(RawHtml(s)),
        Err(_) => Err(Status::InternalServerError)
    }
}

pub fn respond_page(page: &'_ str, c: Context) -> Res<RawHtml<String>> {
    tera_render(&format!("pages/{}.html", page), c)
}

pub fn lock_database() -> MutexGuard<'static, Connection> {
    DB_CONNECTION.lock().unwrap()
}

fn gen_players() -> HashMap<i32, PlayerRating> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name from players").unwrap();
    let mut players = HashMap::new();
    for p in stmt.query_map((), |row| Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))).unwrap() {
        let (id, name) = p.unwrap();
        players.insert(id, PlayerRating::new(name));
    }
    players
}

fn reset_ratings() {
    *PLAYERS.lock().unwrap() = gen_players();
    *LAST_DATE.lock().unwrap() = INITIAL_DATE_CAP.to_owned();
}

pub struct Game {
    home: i32,
    away: i32,
    dato: DateTime,
    ace: bool,
    home_win: bool,
}

fn get_games<'a>() -> Vec<Game> {
    let conn = lock_database();
    let mut last_date = LAST_DATE.lock().unwrap();
    let mut stmt =
        conn.prepare(&format!("SELECT home_id, away_id, home_score, away_score, dato from games WHERE dato > datetime('{}') order by dato asc", *last_date))
            .unwrap();

    let gs = stmt.query_map((), |row| {
            let home_score = row.get::<_, i32>(2)?;
            let away_score = row.get::<_, i32>(3)?;
            // FIXME: 
            *last_date = row.get(4)?;
            Ok(Game {
                home: row.get(0)?,
                away: row.get(1)?,
                dato: last_date.clone(),
                ace: home_score == 0 || away_score == 0,
                home_win: home_score > away_score,
            })
        })
        .unwrap()
        .map(Result::unwrap);
    gs.collect()
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

#[derive(Debug, Serialize)]
struct Named {
    id: i32,
    name: String
}

#[derive(Debug, Serialize)]
struct Ball {
    id: i32,
    name: String,
    img: String,
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

#[launch]
fn rocket() -> _ {
    use crate::errors::*;
    _ = &*CONFIG;
    rocket::build()
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
        .register("/", catchers![page_not_found, bad_request, server_error])
}
