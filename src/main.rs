extern crate pencil;
extern crate bbt;
extern crate rustc_serialize;
extern crate env_logger;
extern crate rand;
extern crate rusqlite;
extern crate time;

use pencil::Pencil;
use pencil::{Request, PencilResult, Response};
//use pencil::method::Get;
use pencil::HTTPError;
use std::collections::BTreeMap;
use rustc_serialize::json::{Json, ToJson};
use rand::random;
use time::Timespec;
use rusqlite::Connection;

#[derive(Debug)]
struct SqlPlayer {
    id:     i32,
    name:   String,
    rating: i32,
    wins:   i32,
    games:  i32,
}

fn sqltest(_: &mut Request) -> PencilResult {
        let conn = Connection::open("ratings.db").unwrap_or_else(|e| panic!("{:?}", e));
        let mut stmt = conn.prepare("SELECT id, name, rating, wins, games from players").unwrap_or_else(|e| panic!("{:?}", e));
        let sqlplayer_iter = stmt.query_map(&[], |row|
            SqlPlayer {
                id: row.get(0),
                name: row.get(1),
                rating: row.get(2),
                wins: row.get(3),
                games: row.get(4)
            }
        ).unwrap_or_else(|e| panic!("{:?}", e));

        for player in sqlplayer_iter {
            println!("Found person {:?}", player);
        }
        Ok(Response::from("SQL-test"))
}

fn page_not_found(_: HTTPError) -> PencilResult {
    let mut response = Response::from("Uh-ohh, 404 :O");
    response.status_code = 404;
    Ok(response)
}

#[derive(Debug)]
struct Player {
    name: String,
    rating: Rating,
    kampe: u32,
    vundne: u32,
    tabte: u32
}

use std::mem::replace;
use bbt::Outcome::{Win, Loss};

impl Player {
    fn new<S: ToString>(name: S) -> Self{
        Player{
            name: name.to_string(),
            rating: Rating::new(1500.0, 1500.0/3.0),
            kampe: 0,
            vundne: 0,
            tabte: 0
        }
    }
    fn duel(&mut self, rater: &Rater, o: Rating, won: bool) {
        let a = replace(&mut self.rating, Default::default());

        let (a, _) = rater.duel(a, o, if won{Win}else{Loss});
        self.rating = a;

        self.kampe += 1;
        if won{
            self.vundne += 1;
        }else{
            self.tabte += 1;
        }
    }
    fn duel_mut(&mut self, rater: &Rater, o: &mut Player, won: bool) {
        self.duel(rater, o.rating.clone(), won);
        o.duel(rater, self.rating.clone(), won);
    }
}

impl ToJson for Player {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("rating".to_string(), self.rating.to_string().to_json());
        m.insert("name".to_string(), self.name.to_json());
        m.insert("kampe".to_string(), self.kampe.to_json());
        m.insert("vundne".to_string(), self.vundne.to_json());
        m.insert("tabte".to_string(), self.tabte.to_json());
        m.to_json()
    }
}

macro_rules! js_arr {
    ($($v:expr),+) => (
        [$($v.to_json()),+].to_json()
    );
}
macro_rules! players {
    ($($v:expr),+) => (
        vec![$(Player::new($v)),+]
    );
}

use bbt::{Rater, Rating};

fn get_rating(rating: &Rating) -> f64 {
    rating.mu() - 3. * rating.sigma()
}

use std::cmp::Ordering::{Greater, Less};
use std::time::{Instant};

fn rating(request: &mut Request) -> PencilResult {
    let rater = Rater::new(1500.0/6.0);
    let mut ps = players!["Biver", "Bo-Bent", "Niels-Erik", "Bente-Magrethe", "Ib", "Josefine", "Mads", "Bjarke", "Lucas", "Niels", "Jacob", "Troels", "Frodo", "Niqhlas"];

    let mut context = BTreeMap::new();
    let now = Instant::now();
    for _ in 0..500{
        for i in 0..ps.len() {
            for j in (0..ps.len()).filter(|&j| j != i) {
                let r = ps[j].rating.clone();
                ps[i].duel(&rater, r, random());
                let r = ps[i].rating.clone();
                ps[j].duel(&rater, r, random());
            }
        }
    }

    let dur = Instant::now() - now;
    println!("Time taken: {}.{:09}s", dur.as_secs(), dur.subsec_nanos());

    ps.sort_by(|a, b| if get_rating(&b.rating) < get_rating(&a.rating){Less}else{Greater});
    context.insert("ps".to_string(), ps.to_json());
    context.insert("heading".to_string(), "Top 4".to_json());
    context.insert("body".to_string(), "Alle ratings".to_json());
    request.app.render_template("index.html", &context)
}

fn main() {
    let mut app = Pencil::new("");
    app.register_template("index.html");
    app.get("/", "index_template", rating);
    app.get("/rating", "flest heste", rating);
    app.get("/sql", "sql", sqltest);
    app.enable_static_file_handling();
    app.httperrorhandler(404, page_not_found);
    // app.set_debug(true);
    // app.set_log_level();
    // env_logger::init().unwrap();
    app.run("0.0.0.0:5000");
}
