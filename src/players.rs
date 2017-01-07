use ::*;

#[get("/players")]
fn players<'a>() -> Res<'a> {
    let conn = DB_CONNECTION.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name from players ORDER BY name ASC").unwrap();

    let mut players = Vec::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.push(Named{id: id, name: name});
    }

    let mut context = create_context("players");
    context.add("players", &players);

    TERA.render("pages/players.html", context).respond()
}

#[get("/player/<name>")]
fn player<'a>(name:String) -> Res<'a> {
    let conn = DB_CONNECTION.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g \
                      where (home = ?1) or (away = ?1) ORDER BY ID DESC")
            .unwrap();
    let games: Vec<_> = stmt.query_map(&[&name], |row| {
            PlayedGame {
                home: row.get(0),
                away: row.get(1),
                home_score: row.get(2),
                away_score: row.get(3),
                ball: row.get(5),
                ball_name: row.get(6),
                dato: row.get(7),
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    //let name: String = conn.query_row("SELECT name from players WHERE id = ?1", &[&id], |row| row.get(0)).unwrap();
    let mut context = create_context("player");
         // TODO handle players that don't exist
    if games.len() != 0 {
        println!("Ukendt spiller: {}",name);
    }
    context.add("games", &games);
    context.add("name", &name);
    TERA.render("pages/player.html", context).respond()
}

#[get("/newplayer")]
fn newplayer<'a>() -> Res<'a> {
    TERA.render("pages/newplayer.html", create_context("newplayer")).respond()
}

#[post("/newplayer/submit", data = "<f>")]
fn submit_newplayer<'r>(f: Data) -> Res<'r> {
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
        let conn = DB_CONNECTION.lock().unwrap();
        let n = conn.execute("INSERT INTO players (name) VALUES (?)", &[&name]).unwrap();

        if n == 0 {
            context.add("fejl", &"Den indtastede spiller eksisterer allerede 💩");
        } else {
            reset_ratings();
            return Redirect::to("/").respond();
        }
    }
    TERA.render("pages/newplayer_fejl.html", context).respond()
}