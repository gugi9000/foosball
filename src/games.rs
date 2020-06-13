use crate::*;

use diesel::sql_types::Integer;

#[get("/newgame")]
pub fn newgame<'a>(conn: DbConn) -> ContRes<'a> {
    respond_page("newgame", newgame_con(conn))
}

pub fn newgame_con(conn: DbConn) -> Context {
    let names: Vec<model::Player> = diesel::sql_query("SELECT id, name FROM players order by random()").load(&*conn).unwrap();

    let balls = model::Ball::read_all(&*conn);

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
pub fn submit_newgame<'a>(conn: DbConn, f: Form<NewGame>) -> Resp<'a> {
    let f = f.into_inner();

    if f.secret != CONFIG.secret {
        let mut context = newgame_con(conn);
        context.insert("fejl", &"Det indtastede kodeord er forkert 💩");
        return Resp::cont(respond_page("newgame_fejl", context));
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away || f.home_score > 10 || f.away_score > 10 {
        let mut context = newgame_con(conn);

        context.insert("fejl",
                       &"Den indtastede kamp er ikke lovlig 😜");
        return Resp::cont(respond_page("newgame_fejl", context));
    }

    let res = diesel::sql_query("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
        ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)")
        .bind::<Integer, _>(f.home)
        .bind::<Integer, _>(f.away)
        .bind::<Integer, _>(f.home_score)
        .bind::<Integer, _>(f.away_score)
        .bind::<Integer, _>(f.ball)
        .execute(&*conn)
        .unwrap();
    println!("{:?}", res);

    Resp::red(Redirect::to("/"))
}

#[get("/games")]
pub fn games<'a>(conn: DbConn) -> ContRes<'a> {
    let results: Vec<model::PlayedGameQuery> = diesel::sql_query("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
              (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
              away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id) as ball_img, \
              (SELECT name FROM balls b WHERE ball_id = b.id) as ball_name, dato FROM games g WHERE dato > date('now', 'start of month') ORDER BY dato DESC")
        .load(&*conn)
        .unwrap();
    let games: Vec<_> = results.into_iter().map(|row| {
            PlayedGame {
                home: row.home,
                away: row.away,
                home_score: row.home_score,
                away_score: row.away_score,
                ball: row.ball_img,
                ball_name: row.ball_name,
                dato: row.dato,
            }
        })
        .collect();

    let mut context = create_context("games");

    context.insert("games", &games);
    respond_page("games", context)
}
