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
    rating: String,
    name: String,
    kampe: u32,
    vundet: u32,
    tabt: u32
}

impl ToJson for Player {
    fn to_json(&self) -> Json {
        let mut m: BTreeMap<String, Json> = BTreeMap::new();
        m.insert("rating".to_string(), self.rating.to_json());
        m.insert("name".to_string(), self.name.to_json());
        m.insert("kampe".to_string(), self.kampe.to_json());
        m.insert("vundet".to_string(), self.vundet.to_json());
        m.insert("tabt".to_string(), self.tabt.to_json());
        m.to_json()
    }
}

use bbt::{Rater, Rating};

fn rating(request: &mut Request) -> PencilResult {
    let rater = Rater::new(1500.0/6.0);
    let p1 = Rating::new(1500.0, 1500.0/3.0);
    let p2 = Rating::new(1500.0, 1500.0/3.0);
    let mut context = BTreeMap::new();
    let (p1, p2) = rater.duel(p1.clone(), p2.clone(), bbt::Outcome::Loss);
    let p1 = Player{rating: p1.to_string(),kampe, name, tabt, vundet};
    let p2 = Player{rating: p1.to_string(),kampe, name, tabt, vundet};
    context.insert("ps".to_string(), [p1.to_json(), p2.to_json()].to_json());
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
    app.run("127.0.0.1:5000");
}
