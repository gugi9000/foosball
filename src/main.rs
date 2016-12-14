extern crate pencil;
extern crate bbt;
extern crate rustc_serialize;
extern crate env_logger;
extern crate rand;
extern crate rusqlite;
extern crate time;

use pencil::Pencil;
use pencil::{Request, PencilResult, Response};
use pencil::method::Post;
use pencil::HTTPError;
use std::collections::BTreeMap;
use rustc_serialize::json::{Json, ToJson};
// use time::Timespec;
use rusqlite::Connection;

const BETA: f64 = 5000.0;

#[derive(Debug)]
struct PlayedGame {
    home: String,
    away: String,
    home_score: i32,
    away_score: i32,
    ball: i32,
}

impl ToJson for PlayedGame {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("home".to_string(), self.home.to_json());
        m.insert("away".to_string(), self.away.to_json());
        m.insert("home_score".to_string(), self.home_score.to_json());
        m.insert("away_score".to_string(), self.away_score.to_json());
        m.insert("ball".to_string(), self.ball.to_json());
        m.to_json()
    }
}

use rand::Rng;

fn newgame(request: &mut Request) -> PencilResult {
    request.app.render_template("index.html", &newgame_con())
}

fn newgame_con() -> BTreeMap<String, Json> {
    let conn = Connection::open("ratings.db").unwrap();
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
    context
}

fn submit_newgame(request: &mut Request) -> PencilResult {
    let conn = Connection::open("ratings.db").unwrap();

    let home: i32 = request.form().get("home").unwrap().parse().unwrap();
    let away: i32 = request.form().get("away").unwrap().parse().unwrap();
    let home_score: i32 = request.form().get("home_score").unwrap().parse().unwrap();
    let away_score: i32 = request.form().get("away_score").unwrap().parse().unwrap();
    let ball: i32 = request.form().get("ball").unwrap().parse().unwrap();

    let secret = request.form().get("secret").unwrap();
    if secret != "jeg er sikker" {
        let mut context = newgame_con();
        context.insert("nykamp_fejl".to_string(),
                       "Det indtastede kodeord er forkert 💩".to_json());
        context.insert("heading".to_string(), "Fejl: Ny kamp".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny kamp".to_json());
        return request.app.render_template("index.html", &context);
    }

    if !(home_score == 10 || away_score == 10) || home_score == away_score || home == away {
        let mut context = newgame_con();

        context.insert("nykamp_fejl".to_string(),
                       "Den indtastede kamp er ikke lovlig 😜".to_json());
        context.insert("heading".to_string(), "Fejl: Ny kamp".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny kamp".to_json());
        return request.app.render_template("index.html", &context);
    }

    println!("{:?}",
             conn.execute("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
                           ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)",
                          &[&home, &away, &home_score, &away_score, &ball]));

    pencil::redirect("/", 302)
}

fn games(request: &mut Request) -> PencilResult {
    let conn = Connection::open("ratings.db").unwrap();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id FROM games g ORDER BY ID DESC")
            .unwrap();
    let games: Vec<_> = stmt.query_map(&[], |row| {
            PlayedGame {
                home: row.get(0),
                away: row.get(1),
                home_score: row.get(2),
                away_score: row.get(3),
                ball: row.get(4),
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut context = BTreeMap::new();

    context.insert("games".to_string(), games.to_json());
    context.insert("heading".to_string(), "Kampe".to_json());
    context.insert("body".to_string(), "Alle kampe".to_json());
    request.app.render_template("index.html", &context)
}

fn page_not_found(_: HTTPError) -> PencilResult {
    let mut response = Response::from("Uh-ohh, 404 :O");
    response.status_code = 404;
    Ok(response)
}

#[derive(Debug, Clone)]
struct Player {
    name: String,
    rating: Rating,
    kampe: u32,
    vundne: u32,
    tabte: u32,
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
    home_win: bool,
}

use std::cmp::Ordering::{Greater, Less};
use std::collections::HashMap;

fn rating(request: &mut Request) -> PencilResult {
    let rater = Rater::new(BETA / 6.0);

    let conn = Connection::open("ratings.db").unwrap();
    let mut stmt = conn.prepare("SELECT id, name from players").unwrap();
    let mut stmt2 =
        conn.prepare("SELECT home_id, away_id, home_score, away_score, dato, ball_id from games")
            .unwrap();

    let mut players = HashMap::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.insert(id, Player::new(name));
    }

    for g in stmt2.query_map(&[], |row| {
            Game {
                home: row.get(0),
                away: row.get(1),
                home_win: row.get::<_, i32>(2) > row.get::<_, i32>(3),
            }
        })
        .unwrap() {
        let g = g.unwrap();
        let away_rating = players[&g.away].rating.clone();
        let home_rating = players[&g.home].rating.clone();
        players.get_mut(&g.home).unwrap().duel(&rater, away_rating, g.home_win);
        players.get_mut(&g.away).unwrap().duel(&rater, home_rating, !g.home_win);
    }

    let mut ps: Vec<_> = players.values().map(Clone::clone).collect();
    ps.sort_by(|a, b| if get_rating(&b.rating) < get_rating(&a.rating) {
        Less
    } else {
        Greater
    });
    ps.insert(0, Player::new("N/A"));

    let mut context = BTreeMap::new();

    context.insert("ps".to_string(), ps.to_json());
    context.insert("heading".to_string(), "Stilling".to_json());
    context.insert("body".to_string(), "Stilling".to_json());
    request.app.render_template("index.html", &context)
}

fn players(request: &mut Request) -> PencilResult {
    let conn = Connection::open("ratings.db").unwrap();
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
    request.app.render_template("index.html", &context)
}

fn newplayer(request: &mut Request) -> PencilResult {
    request.app.render_template("index.html", &newplayer_con())
}

fn newplayer_con() -> BTreeMap<String, Json> {
    let mut context = BTreeMap::new();
    context.insert("newplayer".to_string(), true.to_json());
    context.insert("heading".to_string(), "Ny spiller".to_json());
    context.insert("body".to_string(), "Ny spiller".to_json());
    context
}

fn submit_newplayer(request: &mut Request) -> PencilResult {
    {
        let secret = request.form().get("secret").unwrap();
        if secret != "jeg er sikker" {
            let mut context = newplayer_con();
            context.insert("nyspiller_fejl".to_string(),
                           "Det indtastede kodeord er forkert 💩".to_json());
            context.insert("heading".to_string(), "Fejl: Ny spiller".to_json());
            context.insert("body".to_string(),
                           "Fejl ved registrering af ny spiller".to_json());
            return request.app.render_template("index.html", &context);
        }
    }
    let name = request.form().get("name").unwrap();
    if name.is_empty() {
        let mut context = newplayer_con();

        context.insert("nyspiller_fejl".to_string(),
                       "Den indtastede spiller er ikke lovlig 😜".to_json());
        context.insert("heading".to_string(), "Fejl: Ny spiller".to_json());
        context.insert("body".to_string(),
                       "Fejl ved registrering af ny spiller".to_json());
        return request.app.render_template("index.html", &context);
    }
    let conn = Connection::open("ratings.db").unwrap();
    println!("{:?}",
             conn.execute("INSERT INTO players (name) VALUES (?)", &[&*name]));

    pencil::redirect("/", 302)
}


fn main() {
    let mut app = Pencil::new("");
    app.register_template("index.html");
    app.get("/", "index_template", rating);
    app.get("/rating", "flest heste", rating);
    app.get("/games", "games", games);
    app.get("/newgame", "new squirtle game", newgame);
    app.get("/players", "players hest", players);
    app.route("/newgame/submit", &[Post], "ny kamp", submit_newgame);
    app.get("/newplayer", "players new new new hest", newplayer);
    app.route("/newplayer/submit",
              &[Post],
              "ny hestesandwcih",
              submit_newplayer);

    app.enable_static_file_handling();
    app.httperrorhandler(404, page_not_found);
    // app.set_debug(true);
    // app.set_log_level();
    // env_logger::init().unwrap();
    app.run("127.0.0.1:5000");
}
