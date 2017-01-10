#![feature(plugin, custom_derive, conservative_impl_trait)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate bbt;
extern crate rustc_serialize;
extern crate rand;
extern crate rusqlite;
extern crate toml;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate tera;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::Read;
use std::cmp::Ordering::{Greater, Less};
use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard};
use rocket::request::{FormItems, FromFormValue, FromForm, Form};
use rocket::response::{NamedFile, Response, Redirect};
use rocket::http::Status;
use rocket::Data;
use rusqlite::Connection;
use tera::{Tera, Context, Value};
use bbt::{Rater, Rating};

macro_rules! fromform_struct {
    (struct $strct:ident {
        $($field:ident: $ftype:ty,)*
    }) => (
        struct $strct {
            $($field: $ftype,)*
        }
        impl<'a> FromForm<'a> for $strct {
            type Error = &'a str;
            fn from_form_string(form_string: &'a str) -> Result<Self, Self::Error> {
                $(let mut $field = FromFormValue::default();)*
                for (k, v) in FormItems(form_string) {
                    match k {
                        $(stringify!($field) => $field = Some(<$ftype>::from_form_value(v)?),)*
                        _ => ()
                    }
                }
                Ok($strct {
                    $($field: $field.ok_or(concat!("no `", stringify!($field), "` found"))?),*
                })
            }
        }
    );
}

mod balls;
mod errors;
mod games;
mod players;
mod statics;
mod ratings;
mod analysis;

const BETA: f64 = 5000.0;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const INITIAL_DATE_CAP: &'static str = "now','-90 day";

pub type Res<'a> = Result<Response<'a>, Status>;

#[derive(Debug, RustcDecodable)]
pub struct Config {
    database: String,
    title: String,
    secret: String,
}

fn egg_filter(value: Value, args: HashMap<String, Value>) -> tera::Result<Value> {
    let goals = try_get_value!("egg", "value", i32, value);
    if goals == 0 {
        Ok(Value::String(format!(r#"<img src="/static/egg.png" alt="{} fik æg!">"#, try_get_value!("egg", "person", String, args["person"]))))
    } else {
        Ok(value)
    }
}

lazy_static! {
    pub static ref TERA: Tera = {
        let mut tera = compile_templates!("templates/**/*.html");
        tera.autoescape_on(vec![]);
        tera.register_filter("egg", egg_filter);
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

        toml::decode_str(&buf).unwrap()
    };
    static ref RATER: Rater = Rater::new(BETA / 6.0);
    pub static ref PLAYERS: Mutex<HashMap<i32, Player>> = Mutex::new(gen_players());
    pub static ref LAST_DATE: Mutex<String> = Mutex::new(INITIAL_DATE_CAP.to_owned());
    // Has to be a `Mutex` because `Connection` isn't `Sync`
    pub static ref DB_CONNECTION: Mutex<Connection> = Mutex::new({
        let exists = match File::open(&CONFIG.database) {
            Ok(_) => true,
            Err(_) => false,
        };
        let conn = Connection::open(&CONFIG.database).unwrap();
        if !exists {
            println!("Database didn't exist... creating one");
            conn.execute_batch(include_str!("ratings.schema")).unwrap();
        }
        conn
    });
    static ref BASE_CONTEXT: Context = {
        let mut c = Context::new();
        c.add("version", &VERSION);
        c.add("league", &CONFIG.title);
        c
    };
}

pub fn lock_database() -> MutexGuard<'static, Connection> {
    DB_CONNECTION.lock().unwrap()
}

fn gen_players() -> HashMap<i32, Player> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name from players").unwrap();
    let mut players = HashMap::new();
    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.insert(id, Player::new(name));
    }
    players
}

fn reset_ratings() {
    *PLAYERS.lock().unwrap() = gen_players();
    *LAST_DATE.lock().unwrap() = INITIAL_DATE_CAP.to_owned();
}

fn get_games<'a>() -> Vec<Game> {
    let conn = lock_database();
    let mut last_date = LAST_DATE.lock().unwrap();
    let mut stmt =
        conn.prepare(&format!("SELECT home_id, away_id, home_score, away_score, dato from games WHERE dato > datetime('{}') order by dato asc", *last_date))
            .unwrap();

    let gs = stmt.query_map(&[], |row| {
            let home_score = row.get::<_, i32>(2);
            let away_score = row.get::<_, i32>(3);
            // FIXME
            *last_date = row.get(4);
            Game {
                home: row.get(0),
                away: row.get(1),
                ace: home_score == 0 || away_score == 0,
                home_win: home_score > away_score,
            }
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
    c.add("cur", &current_page);
    c
}

#[derive(Debug, Clone)]
pub struct SerRating(Rating);

impl SerRating {
    fn get_rating(&self) -> f64 {
        self.0.mu() - 3. * self.0.sigma()
    }
}

impl serde::Serialize for SerRating {
    fn serialize<S: serde::Serializer>(&self, serializer: &mut S) -> Result<(), S::Error> {
        let mut state = serializer.serialize_struct("Rating", 2)?;
        serializer.serialize_struct_elt(&mut state, "mu", format!("{:.1}", self.0.mu()))?;
        serializer.serialize_struct_elt(&mut state, "sigma", format!("{:.1}", self.0.sigma()))?;
        serializer.serialize_struct_end(state)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Player {
    pub name: String,
    pub rating: SerRating,
    pub kampe: u32,
    pub vundne: u32,
    pub tabte: u32,
    pub eggs: u32,
    pub aces: u32,
}

use std::mem::replace;
use bbt::Outcome::{Win, Loss};

impl Player {
    fn new<S: ToString>(name: S) -> Self {
        Player {
            name: name.to_string(),
            rating: SerRating(Rating::new(BETA, BETA / 3.0)),
            kampe: 0,
            vundne: 0,
            tabte: 0,
            eggs: 0,
            aces: 0,

        }
    }
    fn duel(&mut self, o: Rating, won: bool) {
        let a = replace(&mut self.rating.0, Default::default());

        let (a, _) = RATER.duel(a, o, if won { Win } else { Loss });
        self.rating.0 = a;

        self.kampe += 1;
        if won {
            self.vundne += 1;
        } else {
            self.tabte += 1;
        }
    }
}

struct Game {
    home: i32,
    away: i32,
    ace: bool,
    home_win: bool,
}

fn main() {
    use errors::*;
    &*CONFIG;
    rocket::ignite()
        .mount("/",
               routes![analysis::analysis,
                       ratings::root,
                       statics::favicon_handler,
                       games::games,
                       games::newgame,
                       games::submit_newgame,
                       balls::balls,
                       balls::ball,
                       players::players,
                       players::player,
                       players::newplayer,
                       players::submit_newplayer,
                       ratings::reset,
                       ratings::rating,
                       statics::static_handler])
        .catch(errors![page_not_found, bad_request, server_error])
        .launch();
}
