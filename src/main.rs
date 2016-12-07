extern crate pencil;
extern crate bbt;
extern crate rustc_serialize;
extern crate env_logger;

use pencil::Pencil;
use pencil::{Request, PencilResult, Response};
//use pencil::method::Get;
use pencil::HTTPError;
use std::collections::BTreeMap;
use rustc_serialize::json::{Json, ToJson};

fn index_template(request: &mut Request) -> PencilResult {
    let mut context = BTreeMap::new();
    context.insert("heading".to_string(), "Top 4".to_string());
    context.insert("body".to_string(), "Alle ratings".to_string());
    request.app.render_template("index.html", &context)
}

fn page_not_found(_: HTTPError) -> PencilResult {
    let mut response = Response::from("Uh-ohh, 404 :O");
    response.status_code = 404;
    Ok(response)
}

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
    fn duel(&mut self, rater: &Rater, o: &mut Player, won: bool) {
        let a = replace(&mut self.rating, Default::default());
        let b = replace(&mut o.rating, Default::default());

        let (a, b) = rater.duel(a, b, if won{Win}else{Loss});
        self.rating = a;
        o.rating = b;

        self.kampe += 1;
        o.kampe += 1;
        if won{
            self.vundne += 1;
            o.tabte += 1;
        }else{
            self.tabte += 1;
            o.vundne += 1;
        }
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

use bbt::{Rater, Rating};

fn rating(request: &mut Request) -> PencilResult {
    let rater = Rater::new(1500.0/6.0);
    let mut p1 = Player::new("Biver");
    let mut p2 = Player::new("Bo-Erik");
    let mut p3 = Player::new("Qrizdan");

    let mut context = BTreeMap::new();
    p3.duel(&rater, &mut p2, true);
    for _ in 0..15{
        p1.duel(&rater, &mut p2, false);
        p1.duel(&rater, &mut p2, true);
    }
    context.insert("ps".to_string(), js_arr![p1, p2, p3]);
    context.insert("heading".to_string(), "Top 4".to_json());
    context.insert("body".to_string(), "Alle ratings".to_json());
    request.app.render_template("index.html", &context)
}

fn main() {
    let mut app = Pencil::new("");
    app.register_template("index.html");
    app.get("/", "index_template", index_template);
    app.get("/rating", "flest heste", rating);
    app.enable_static_file_handling();
    app.httperrorhandler(404, page_not_found);
    app.set_debug(true);
    app.set_log_level();
    env_logger::init().unwrap();
    app.run("0.0.0.0:5000");
}
