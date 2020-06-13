use crate::*;

use diesel::sql_types::{Text, Integer};

#[derive(Debug, Serialize, Queryable, QueryableByName)]
pub struct Ballstats {
    #[sql_type = "Integer"]
    ball_id: i32,
    #[sql_type = "Integer"]
    goals: i32,
    #[sql_type = "Integer"]
    games: i32,
    #[sql_type = "Text"]
    name: String,
    #[sql_type = "Text"]
    img: String,
}

#[get("/balls")]
pub fn balls<'a>(conn: DbConn) -> ContRes<'a> {
    let ballstats =
        diesel::sql_query("select ball_id, sum(home_score+away_score) as goals, count(ball_id) as balls, (select name from balls where ball_id = balls.id), (select img from balls where ball_id = balls.id) FROM games WHERE dato > date('now','start of month') GROUP BY ball_id order by balls desc , goals desc")
            .load::<Ballstats>(&*conn).unwrap();

    let mut context = create_context("balls");
    context.insert("ballstats", &ballstats);

    respond_page("balls", context)
}

#[get("/ball/<ball>")]
pub fn ball<'a>(conn: DbConn, ball: String) -> ContRes<'a> {
    let results: Vec<model::PlayedGameQuery> =
        diesel::sql_query("SELECT \
            (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
            (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, away_score, ball_id,  \
            (SELECT img FROM balls b WHERE ball_id = b.id) as ball_img, \
            (SELECT name FROM balls b WHERE ball_id = b.id) AS ball_name, \
            dato \
            FROM games g  \
            WHERE ballname = ?1 AND dato > date('now','start of month') \
            ORDER BY ID DESC")
            .bind::<Text, _>(&*ball)
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

    let mut context = create_context("balls");

    context.insert("games", &games);
    context.insert("ball", &ball);
    respond_page("ball", context)
}

#[get("/newball")]
pub fn newball<'a>(conn: DbConn) -> ContRes<'a> {
    let balls = model::Ball::read_all_ordered(&*conn);

    let mut context = create_context("balls");
    context.insert("balls", &balls);
    
    respond_page("newball", context)
}

#[derive(FromForm)]
pub struct NewBallQuery {
    name: String,
    img: String,
    secret: String,
    #[allow(dead_code)]
    submit: IgnoreField
}

#[post("/newball/submit", data = "<f>")]
pub fn submit_newball<'r>(conn: DbConn, players: Players, f: Form<NewBallQuery>) -> Resp<'r> {
    let NewBallQuery{name, img, secret, ..} = f.into_inner();

    let mut context = create_context("balls");
    if secret != CONFIG.secret {
        context.insert("fejl", &"Det indtastede kodeord er forkert 💩");
    } else if name.is_empty() {
        context.insert("fejl", &"Den indtastede bold er ikke lovlig 😜");
    } else {
        let n =model::InsertableBall {
            name,
            img
        }.insert(&conn);

        if n == 0 {
            context.insert("fejl", &"Den indtastede bold eksisterer allerede 💩");
        } else {
            players.reset(&conn);
            return Resp::red(Redirect::to("/"));
        }
    }
    Resp::cont(respond_page("newball_fejl", context))
}
