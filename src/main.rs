#![feature(plugin, custom_derive)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate bbt;
extern crate rustc_serialize;
extern crate rand;
extern crate rusqlite;
extern crate time;
extern crate toml;
#[macro_use]
extern crate lazy_static;
extern crate handlebars;

use std::path::{Path, PathBuf};
use std::io::Read;
use std::collections::BTreeMap;
use rustc_serialize::json::{Json, ToJson};
// use time::Timespec;
use rocket::request::{FormItems, FromFormValue, FromForm, Form};
use rocket::response::{NamedFile, Response, Responder, Redirect};
use rocket::http::Status;
use rocket::Data;
use rusqlite::Connection;
use handlebars::Handlebars;

const BETA: f64 = 5000.0;
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

#[derive(Debug, RustcDecodable)]
struct Config {
    database: String,
    title: String,
    secret: String,
}

lazy_static! {
    static ref HB: Handlebars = {
        let mut hb = Handlebars::new();
        hb.register_template_file("index.html", "templates/index.html").unwrap();
        hb
    };
    static ref CONFIG: Config = {
        let mut buf = String::new();
        let mut file = std::fs::File::open("Foosball.toml").unwrap();
        file.read_to_string(&mut buf).unwrap();

        toml::decode_str(&buf).unwrap()
    };
}

#[derive(Debug)]
struct PlayedGame {
    home: String,
    away: String,
    home_score: i32,
    away_score: i32,
    ball: i32,
    dato: String,
}

impl ToJson for PlayedGame {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("home".to_string(), self.home.to_json());
        m.insert("away".to_string(), self.away.to_json());
        m.insert("home_score".to_string(), self.home_score.to_json());
        m.insert("away_score".to_string(), self.away_score.to_json());
        m.insert("ball".to_string(), self.ball.to_json());
        m.insert("dato".to_string(), self.dato.to_json());
        m.to_json()
    }
}

use rand::Rng;

#[get("/newgame")]
fn newgame() -> String {
    HB.render("index.html", &newgame_con()).unwrap()
}

fn newgame_con() -> BTreeMap<String, Json> {
    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt = conn.prepare("SELECT id, name FROM players").unwrap();
    let mut names: Vec<_> = stmt.query_map(&[], |row| {
            let mut n = BTreeMap::new();
            n.insert("id".to_string(), row.get::<_, i32>(0).to_json());
            n.insert("name".to_string(), row.get::<_, String>(1).to_json());
            n.to_json()
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    names.sort_by(|_, _| *rand::thread_rng().choose(&[Greater, Less]).unwrap());

    let mut ballstmt = conn.prepare("SELECT id, name FROM balls").unwrap();
    let balls: Vec<_> = ballstmt.query_map(&[], |row| {
            let mut b = BTreeMap::new();
            b.insert("id".to_string(), row.get::<_, i32>(0).to_json());
            b.insert("name".to_string(), row.get::<_, String>(1).to_json());
            b.to_json()
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();


    let mut context = BTreeMap::new();

    context.insert("names".to_string(), names.to_json());
    context.insert("balls".to_string(), balls.to_json());
    context.insert("newgame".to_string(), true.to_json());
    context.insert("heading".to_string(), "Ny kamp".to_json());
    context.insert("body".to_string(), "Ny kamp".to_json());
    context.insert("version".to_string(), VERSION.to_json());
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
    type Error = ();
    fn from_form_string(form_string: &'a str) -> Result<Self, Self::Error> {
        let mut home = FromFormValue::default();
        let mut away = FromFormValue::default();
        let mut home_score = FromFormValue::default();
        let mut away_score = FromFormValue::default();
        let mut ball = FromFormValue::default();
        let mut secret = FromFormValue::default();
        for (k, v) in FormItems(form_string) {
            match k {
                "home" => home = i32::from_form_value(v).ok(),
                "away" => away = i32::from_form_value(v).ok(),
                "home_score" => home_score = i32::from_form_value(v).ok(),
                "away_score" => away_score = i32::from_form_value(v).ok(),
                "ball" => ball = i32::from_form_value(v).ok(),
                "secret" => secret = String::from_form_value(v).ok(),
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
        context.insert("nykamp_fejl".to_string(),
                       "Det indtastede kodeord er forkert 💩".to_json());
        context.insert("heading".to_string(), "Fejl: Ny kamp".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny kamp".to_json());
        return HB.render("index.html", &context).unwrap().respond();
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away {
        let mut context = newgame_con();

        context.insert("nykamp_fejl".to_string(),
                       "Den indtastede kamp er ikke lovlig 😜".to_json());
        context.insert("heading".to_string(), "Fejl: Ny kamp".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny kamp".to_json());
        return HB.render("index.html", &context).unwrap().respond();
    }

    println!("{:?}",
             conn.execute("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
                           ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)",
                          &[&f.home, &f.away, &f.home_score, &f.away_score, &f.ball]));

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

    let mut context = BTreeMap::new();

    context.insert("games".to_string(), games.to_json());
    context.insert("heading".to_string(), "Kampe".to_json());
    context.insert("body".to_string(), "Alle kampe".to_json());
    context.insert("version".to_string(), VERSION.to_json());
    HB.render("index.html", &context).unwrap()
}

#[error(404)]
fn page_not_found() -> String {
    let mut context = BTreeMap::new();

    context.insert("not_found".to_string(), "Not Found".to_json());
    context.insert("heading".to_string(), "404 Not Found".to_json());
    context.insert("body".to_string(), "404 Not Found".to_json());
    context.insert("version".to_string(), VERSION.to_json());
    HB.render("index.html", &context).unwrap()
}

#[derive(Debug, Clone)]
struct Player {
    name: String,
    rating: Rating,
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
            rating: Rating::new(BETA, BETA / 3.0),
            kampe: 0,
            vundne: 0,
            tabte: 0,
            eggs: 0,
            aces: 0,
        }
    }
    fn duel(&mut self, rater: &Rater, o: Rating, won: bool) {
        let a = replace(&mut self.rating, Default::default());

        let (a, _) = rater.duel(a, o, if won { Win } else { Loss });
        self.rating = a;

        self.kampe += 1;
        if won {
            self.vundne += 1;
        } else {
            self.tabte += 1;
        }
    }
}

impl ToJson for Player {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("rating".to_string(),
                 format!("{:.1}", self.rating.mu()).to_json());
        m.insert("sigma".to_string(),
                 format!("{:.1}", self.rating.sigma()).to_json());
        m.insert("name".to_string(), self.name.to_json());
        m.insert("kampe".to_string(), self.kampe.to_json());
        m.insert("vundne".to_string(), self.vundne.to_json());
        m.insert("tabte".to_string(), self.tabte.to_json());
        m.insert("eggs".to_string(), self.eggs.to_json());
        m.insert("aces".to_string(), self.aces.to_json());
        m.to_json()
    }
}

use bbt::{Rater, Rating};

fn get_rating(rating: &Rating) -> f64 {
    rating.mu() - 3. * rating.sigma()
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
        let away_rating = players[&g.away].rating.clone();
        let home_rating = players[&g.home].rating.clone();

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
    ps.sort_by(|a, b| if get_rating(&b.rating) < get_rating(&a.rating) {
        Less
    } else {
        Greater
    });
    ps.retain(|a| a.kampe != 0);
    ps.insert(0, Player::new("N/A"));

    let mut context = BTreeMap::new();

    context.insert("ps".to_string(), ps.to_json());
    context.insert("heading".to_string(), "Stilling".to_json());
    context.insert("body".to_string(), "Stilling".to_json());
    context.insert("version".to_string(), VERSION.to_json());
    HB.render("index.html", &context).unwrap()
}

#[get("/players")]
fn players() -> String {
    let conn = Connection::open(&CONFIG.database).unwrap();
    let mut stmt = conn.prepare("SELECT id, name from players").unwrap();

    let mut players = Vec::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        let mut player = BTreeMap::new();
        player.insert("id".to_string(), id.to_json());
        player.insert("name".to_string(), name.to_json());
        players.push(player.to_json());
    }

    let mut context = BTreeMap::new();

    context.insert("players".to_string(), players.to_json());
    context.insert("heading".to_string(), "Spillere".to_json());
    context.insert("body".to_string(), "Spillere".to_json());
    context.insert("version".to_string(), VERSION.to_json());
    HB.render("index.html", &context).unwrap()
}

#[get("/newplayer")]
fn newplayer() -> String {
    HB.render("index.html", &newplayer_con()).unwrap()
}

fn newplayer_con() -> BTreeMap<String, Json> {
    let mut context = BTreeMap::new();
    context.insert("newplayer".to_string(), true.to_json());
    context.insert("heading".to_string(), "Ny spiller".to_json());
    context.insert("body".to_string(), "Ny spiller".to_json());
    context.insert("version".to_string(), VERSION.to_json());
    context
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
    {
        if secret != CONFIG.secret {
            let mut context = newplayer_con();
            context.insert("nyspiller_fejl".to_string(),
                           "Det indtastede kodeord er forkert 💩".to_json());
            context.insert("heading".to_string(), "Fejl: Ny spiller".to_json());
            context.insert("body".to_string(),
                           "Fejl ved registrering af ny spiller".to_json());
            return HB.render("index.html", &context).respond();
        }
    }
    if name.is_empty() {
        let mut context = newplayer_con();

        context.insert("nyspiller_fejl".to_string(),
                       "Den indtastede spiller er ikke lovlig 😜".to_json());
        context.insert("heading".to_string(), "Fejl: Ny spiller".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny spiller".to_json());
        return HB.render("index.html", &context).respond();
    }
    let conn = Connection::open(&CONFIG.database).unwrap();
    println!("{:?}",
             conn.execute("INSERT INTO players (name) VALUES (?)", &[&name]));

    Redirect::to("/").respond()
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
                       static_handler])
        .catch(errors![page_not_found])
        .launch();
}

#[get("/static/<file..>")]
fn static_handler(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}
