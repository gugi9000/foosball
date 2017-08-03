use ::*;

#[get("/players")]
fn players<'a>() -> ContRes<'a> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name from players ORDER BY name ASC").unwrap();

    let mut players = Vec::new();

    for p in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1))).unwrap() {
        let (id, name) = p.unwrap();
        players.push(Named{id: id, name: name});
    }

    let mut context = create_context("players");
    context.add("players", &players);

    respond_page("players", context)
}

#[get("/player/<name>")]
fn player<'a>(mut name: String) -> ContRes<'a> {
    let conn = lock_database();
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

    let mut context = create_context("players");
    if games.is_empty() {
        name = "Ukendt spiller".to_owned();
    }
    context.add("games", &games);
    context.add("name", &name);
    respond_page("player", context)
}

#[get("/newplayer")]
fn newplayer<'a>() -> ContRes<'a> {
    respond_page("newplayer", create_context("players"))
}

#[derive(FromForm)]
struct NewPlayerQuery {
    name: String,
    secret: String,
    #[allow(dead_code)]
    submit: IgnoreField
}

#[post("/newplayer/submit", data = "<f>")]
fn submit_newplayer<'r>(f: Form<NewPlayerQuery>) -> Resp<'r> {
    let NewPlayerQuery{name, secret, ..} = f.into_inner();

    let mut context = create_context("players");
    if secret != CONFIG.secret {
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
    } else if name.is_empty() {
        context.add("fejl", &"Den indtastede spiller er ikke lovlig 😜");
    } else {
        let n = lock_database().execute("INSERT INTO players (name) VALUES (?)", &[&name]).unwrap();

        if n == 0 {
            context.add("fejl", &"Den indtastede spiller eksisterer allerede 💩");
        } else {
            reset_ratings();
            return Resp::red(Redirect::to("/"));
        }
    }
    Resp::cont(respond_page("newplayer_fejl", context))
}
