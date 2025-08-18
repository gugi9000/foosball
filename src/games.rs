use rocket::{form::FromForm, get, post};

use crate::*;

#[get("/newgame")]
pub fn newgame<'a>() -> ResHtml<'a> {
    respond_page("newgame", newgame_con())
}

pub fn newgame_con() -> Context {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name FROM players order by random()").unwrap();
    let names: Vec<_> = stmt.query_map(NO_PARAMS, |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut ballstmt = conn.prepare("SELECT id, name, img FROM balls").unwrap();
    let balls: Vec<_> = ballstmt.query_map(NO_PARAMS, |row| {
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

    context.insert("names", &names);
    context.insert("balls", &balls);
    context
}

#[derive(FromForm)]
pub struct NewGame {
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
pub fn submit_newgame(f: Form<NewGame>) -> Resp<RawHtml<String>> {
    let f = f.into_inner();

    if f.secret != CONFIG.secret {
        let mut context = newgame_con();
        context.insert("fejl", &"Det indtastede kodeord er forkert 💩");
        return Resp::cont(respond_page("newgame_fejl", context));
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away || f.home_score > 10 || f.away_score > 10 {
        let mut context = newgame_con();

        context.insert("fejl",
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
pub fn games<'a>() -> ResHtml<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g WHERE dato > date('now', 'start of month') ORDER BY dato DESC")
            .unwrap();
    let games: Vec<_> = stmt.query_map(NO_PARAMS, |row| {
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

    context.insert("games", &games);
    respond_page("games", context)
}
