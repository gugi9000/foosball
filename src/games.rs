use ::*;

#[get("/newgame")]
fn newgame<'a>() -> ContRes<'a> {
    respond_page("newgame", newgame_con())
}

fn newgame_con() -> Context {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name FROM players order by random()").unwrap();
    let names: Vec<_> = stmt.query_map(&[], |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut ballstmt = conn.prepare("SELECT id, name, img FROM balls").unwrap();
    let balls: Vec<_> = ballstmt.query_map(&[], |row| {
            Ball {
                id: row.get(0),
                name: row.get(1),
                img: row.get(2),
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut context = create_context("games");

    context.add("names", &names);
    context.add("balls", &balls);
    context
}

#[derive(FromForm)]
struct NewGame {
  home: i32,
  away: i32,
  home_score: i32,
  away_score: i32,
  ball: i32,
  secret: String,
  #[allow(dead_code)]
  submit: IgnoreField
}

#[post("/newgame/submit", data = "<f>")]
fn submit_newgame<'a>(f: Form<NewGame>) -> Resp<'a> {
    let f = f.into_inner();

    if f.secret != CONFIG.secret {
        let mut context = newgame_con();
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
        return Resp::cont(respond_page("newgame_fejl", context));
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away {
        let mut context = newgame_con();

        context.add("fejl",
                       &"Den indtastede kamp er ikke lovlig 😜");
        return Resp::cont(respond_page("newgame_fejl", context));
    }

    let res = lock_database().execute("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
                            ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)",
                           &[&f.home, &f.away, &f.home_score, &f.away_score, &f.ball]);
    println!("{:?}", res);

    Resp::red(Redirect::to("/"))
}

#[get("/games")]
fn games<'a>() -> ContRes<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g ORDER BY dato DESC")
            .unwrap();
    let games: Vec<_> = stmt.query_map(&[], |row| {
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

    let mut context = create_context("games");

    context.add("games", &games);
    respond_page("games", context)
}
