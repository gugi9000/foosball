use crate::*;

use diesel::sql_types::Integer;

#[get("/pvp")]
pub fn pvpindex<'a>(conn: DbConn) -> ContRes<'a> {
    let names = model::Player::read_all_ordered(&conn);

    let mut context = create_context("pvp");
    context.insert("names", &names);

    respond_page("pvpindex", context)
}

#[get("/pvp/<p1>/<p2>")]
pub fn pvp<'a>(conn: DbConn, p1: i32, p2: i32) -> ContRes<'a> {
    let results = diesel::sql_query("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id) as ball_img, \
                      (SELECT name FROM balls b WHERE ball_id = b.id) as ball_name, dato, home_id FROM games g \
                      WHERE (home_id = ?1 and away_id = ?2) and dato > date('now','start of month') \
                      or (home_id = ?2 and away_id = ?1) and dato > date('now','start of month') \
                      ORDER BY dato DESC").bind::<Integer, _>(p1).bind::<Integer, _>(p2).load::<model::PlayedGameQueryWithHomeId>(&*conn).unwrap();

    let mut map = ((0, 0, "".to_owned()), (0, 0, "".to_owned()));

    let pvp: Vec<_> = results.into_iter().map(|row| {
            let game = PlayedGame {
                home: row.home,
                away: row.away,
                home_score: row.home_score,
                away_score: row.away_score,
                ball: row.ball_img,
                ball_name: row.ball_name,
                dato: row.dato,
            };
            let home_id: i32 = row.home_id;

            let home_win = game.home_score > game.away_score;
            {
                let home = if home_id == p1 {&mut map.0} else {&mut map.1};
                if home.2.is_empty() {
                    home.2 = game.home.clone();
                }
                home.0 += game.home_score;
                if home_win { home.1 += 1 }
            }
            {
                let away = if home_id == p1 {&mut map.1} else {&mut map.0};
                if away.2.is_empty() {
                    away.2 = game.away.clone();
                }
                away.0 += game.away_score;
                if !home_win { away.1 += 1 }
            }

            game
        })
        .collect();

    // map is formatted like this:
    //      p1  goals   p1 wins     p1 name
    //      p2  goals   p2 wins     p2 name

    let mut context = create_context("pvp");

    context.insert("pvp", &pvp);
    context.insert("map", &map);
    respond_page("pvp", context)
}
