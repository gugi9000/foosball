#![feature(plugin, custom_derive, proc_macro)]
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

use std::path::{Path, PathBuf};
use std::io::Read;
use rocket::request::{FormItems, FromFormValue, FromForm, Form};
use rocket::response::{NamedFile, Response, Responder, Redirect};
use rocket::http::Status;
use rocket::Data;
use rusqlite::Connection;
use tera::{Tera, Context, Value};

const BETA: f64 = 5000.0;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug, RustcDecodable)]
struct Config {
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
    static ref TERA: Tera = {
        let mut tera = compile_templates!("templates/**/*.html");
        tera.autoescape_on(vec![]);
        tera.register_filter("egg", egg_filter);
        tera
    };
    static ref CONFIG: Config = {
        let mut buf = String::new();
        let mut file = std::fs::File::open("Foosball.toml").unwrap();
        file.read_to_string(&mut buf).unwrap();

        toml::decode_str(&buf).unwrap()
    };
}

#[derive(Debug, Serialize)]
struct PlayedGame {
    home: String,
    away: String,
    home_score: i32,
    away_score: i32,
    ball: i32,
    dato: String,
}

#[derive(Debug, Serialize)]
struct Named {
    id: i32,
    name: String
}

fn create_context(current_page: &str) -> Context {
    let mut c = Context::new();
    c.add("version", &VERSION);
    c.add("cur", &current_page);
    c
}

use rand::Rng;

#[get("/newgame")]
fn newgame() -> String {
    TERA.render("pages/newgame.html", newgame_con()).unwrap()
}

fn newgame_con() -> Context {
    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt = conn.prepare("SELECT id, name FROM players").unwrap();
    let mut names: Vec<_> = stmt.query_map(&[], |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    names.sort_by(|_, _| *rand::thread_rng().choose(&[Greater, Less]).unwrap());

    let mut ballstmt = conn.prepare("SELECT id, name FROM balls").unwrap();
    let balls: Vec<_> = ballstmt.query_map(&[], |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();


    let mut context = create_context("newgame");

    context.add("names", &names);
    context.add("balls", &balls);
    context
}

struct NewGame {
    home: i32,
    away: i32,
    home_score: i32,
    away_score: i32,
    ball: i32,
    secret: String,
}

impl<'a> FromForm<'a> for NewGame {
    type Error = &'a str;
    fn from_form_string(form_string: &'a str) -> Result<Self, Self::Error> {
        let mut home = FromFormValue::default();
        let mut away = FromFormValue::default();
        let mut home_score = FromFormValue::default();
        let mut away_score = FromFormValue::default();
        let mut ball = FromFormValue::default();
        let mut secret = FromFormValue::default();
        for (k, v) in FormItems(form_string) {
            match k {
                "home" => home = Some(i32::from_form_value(v)?),
                "away" => away = Some(i32::from_form_value(v)?),
                "home_score" => home_score = Some(i32::from_form_value(v)?),
                "away_score" => away_score = Some(i32::from_form_value(v)?),
                "ball" => ball = Some(i32::from_form_value(v)?),
                "secret" => secret = Some(String::from_form_value(v)?),
                _ => (),
            }
        }
        Ok(NewGame {
            home: home.unwrap(),
            away: away.unwrap(),
            home_score: home_score.unwrap(),
            away_score: away_score.unwrap(),
            ball: ball.unwrap(),
            secret: secret.unwrap(),
        })
    }
}

#[post("/newgame/submit", data = "<f>")]
fn submit_newgame(f: Form<NewGame>) -> Result<Response, Status> {
    let conn = Connection::open(&CONFIG.database).unwrap();

    let f = f.into_inner();

    if f.secret != CONFIG.secret {
        let mut context = newgame_con();
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
        return TERA.render("pages/newgame_fejl.html", context).respond();
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away {
        let mut context = newgame_con();

        context.add("fejl",
                       &"Den indtastede kamp er ikke lovlig 😜");
        return TERA.render("pages/newgame_fejl.html", context).respond();
    }

    let res = conn.execute("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
                            ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)",
                           &[&f.home, &f.away, &f.home_score, &f.away_score, &f.ball]);
    println!("{:?}", res);

    Redirect::to("/").respond()
}

#[get("/games")]
fn games() -> String {
    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, dato FROM games g ORDER BY ID DESC")
            .unwrap();
    // TODO: join ball_id to balls.img
    let games: Vec<_> = stmt.query_map(&[], |row| {
            PlayedGame {
                home: row.get(0),
                away: row.get(1),
                home_score: row.get(2),
                away_score: row.get(3),
                ball: row.get(4),
                dato: row.get(5),
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut context = create_context("games");

    context.add("games", &games);
    TERA.render("pages/games.html", context).unwrap()
}

#[error(404)]
fn page_not_found() -> String {
    TERA.render("pages/404.html", create_context("404")).unwrap()
}

use bbt::{Rater, Rating};

#[derive(Debug, Clone)]
struct SerRating(Rating);

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
struct Player {
    name: String,
    rating: SerRating,
    kampe: u32,
    vundne: u32,
    tabte: u32,
    eggs: u32,
    aces: u32,
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
    fn duel(&mut self, rater: &Rater, o: Rating, won: bool) {
        let a = replace(&mut self.rating.0, Default::default());

        let (a, _) = rater.duel(a, o, if won { Win } else { Loss });
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

use std::cmp::Ordering::{Greater, Less};
use std::collections::HashMap;

#[get("/")]
fn root() -> String {
    rating()
}

#[get("/rating")]
fn rating() -> String {
    let rater = Rater::new(BETA / 6.0);

    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt = conn.prepare("SELECT id, name from players").unwrap();
    let mut stmt2 =
        conn.prepare("SELECT home_id, away_id, home_score, away_score, dato, ball_id from games WHERE dato >= date('now','-90 day')")
            .unwrap();

    let mut players = HashMap::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.insert(id, Player::new(name));
    }

    for g in stmt2.query_map(&[], |row| {
            let home_score = row.get::<_, i32>(2);
            let away_score = row.get::<_, i32>(3);
            Game {
                home: row.get(0),
                away: row.get(1),
                ace: home_score == 0 || away_score == 0,
                home_win: home_score > away_score,
            }
        })
        .unwrap() {
        let g = g.unwrap();
        let away_rating = players[&g.away].rating.0.clone();
        let home_rating = players[&g.home].rating.0.clone();

        {
            let home_player = players.get_mut(&g.home).unwrap();
            home_player.duel(&rater, away_rating, g.home_win);
            if g.ace {
                if g.home_win {
                    home_player.aces += 1;
                } else {
                    home_player.eggs += 1;
                }
            }
        }
        {
            let away_player = players.get_mut(&g.away).unwrap();
            away_player.duel(&rater, home_rating, !g.home_win);
            if g.ace {
                if g.home_win {
                    away_player.eggs += 1;
                } else {
                    away_player.aces += 1;
                }
            }
        }
    }

    let mut ps: Vec<_> = players.values().map(Clone::clone).collect();
    ps.sort_by(|a, b| if b.rating.get_rating() < a.rating.get_rating() {
        Less
    } else {
        Greater
    });
    ps.retain(|a| a.kampe != 0);

    let mut context = create_context("rating");
    context.add("players", &ps);

    TERA.render("pages/rating.html", context).unwrap()
}

#[get("/players")]
fn players() -> String {
    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt = conn.prepare("SELECT id, name from players ORDER BY name ASC").unwrap();

    let mut players = Vec::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.push(Named{id: id, name: name});
    }

    let mut context = create_context("players");
    context.add("players", &players);

    TERA.render("pages/players.html", context).unwrap()
}

#[get("/newplayer")]
fn newplayer() -> String {
    TERA.render("pages/newplayer.html", create_context("newplayer")).unwrap()
}

#[post("/newplayer/submit", data = "<f>")]
fn submit_newplayer<'r>(f: Data) -> Result<Response<'r>, Status> {
    let mut v = Vec::new();
    f.stream_to(&mut v).unwrap();

    let mut name = Default::default();
    let mut secret = Default::default();
    for (k, v) in FormItems(&String::from_utf8(v).unwrap()) {
        match k {
            "name" => name = String::from_form_value(v).unwrap(),
            "secret" => secret = String::from_form_value(v).unwrap(),
            _ => (),
        }
    }
    let mut context = create_context("newplayer");
    if secret != CONFIG.secret {
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
    } else if name.is_empty() {
        context.add("fejl", &"Den indtastede spiller er ikke lovlig 😜");
    } else {
        let conn = Connection::open(&CONFIG.database).unwrap();
        println!("{:?}",
                 conn.execute("INSERT INTO players (name) VALUES (?)", &[&name]));

        return Redirect::to("/").respond();
    }
    TERA.render("pages/newplayer_fejl.html", context).respond()
}

fn main() {
    rocket::ignite()
        .mount("/",
               routes![root,
                       rating,
                       games,
                       newgame,
                       players,
                       submit_newgame,
                       newplayer,
                       submit_newplayer,
                       favicon_handler,
                       static_handler])
        .catch(errors![page_not_found])
        .launch();
}

#[get("/static/<file..>")]
fn static_handler(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[get("/favicon.ico")]
fn favicon_handler() -> Option<NamedFile> {
    NamedFile::open(Path::new("static/dynateam.ico")).ok()
}
