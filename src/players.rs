use crate::*;

use diesel::sql_types::Text;

#[get("/players")]
pub fn players<'a>(conn: DbConn) -> ContRes<'a> {
    let players = model::Player::read_all_ordered(&conn);

    let mut context = create_context("players");
    context.insert("players", &players);

    respond_page("players", context)
}

#[get("/player/<name>")]
pub fn player<'a>(conn: DbConn, name: String) -> ContRes<'a> {
    let results: Vec<model::PlayedGameQuery> = diesel::sql_query("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                    (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                    away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id) as ball_img, \
                    (SELECT name FROM balls b WHERE ball_id = b.id) as ball_name, dato FROM games g \
                    where (home = ?1) or (away = ?1) \
                    AND dato > date('now', 'start of month') \
                    ORDER BY ID DESC")
        .bind::<Text, _>(&name)
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

    let mut context = create_context("players");

    context.insert("games", &games);
    context.insert("name", &name);
    respond_page("player", context)
}

#[get("/newplayer")]
pub fn newplayer<'a>() -> ContRes<'a> {
    respond_page("newplayer", create_context("players"))
}

#[derive(FromForm)]
pub struct NewPlayerQuery {
    name: String,
    secret: String,
    #[allow(dead_code)]
    submit: IgnoreField
}

#[post("/newplayer/submit", data = "<f>")]
pub fn submit_newplayer<'r>(conn: DbConn, players: Players, f: Form<NewPlayerQuery>) -> Resp<'r> {
    let NewPlayerQuery{name, secret, ..} = f.into_inner();

    let mut context = create_context("players");
    if secret != CONFIG.secret {
        context.insert("fejl", &"Det indtastede kodeord er forkert 💩");
    } else if name.is_empty() {
        context.insert("fejl", &"Den indtastede spiller er ikke lovlig 😜");
    } else {
        let n = model::InsertablePlayer{name}.insert(&conn);

        if n == 0 {
            context.insert("fejl", &"Den indtastede spiller eksisterer allerede 💩");
        } else {
            players.reset(&conn);
            return Resp::red(Redirect::to("/"));
        }
    }
    Resp::cont(respond_page("newplayer_fejl", context))
}
