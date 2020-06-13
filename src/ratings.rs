use crate::*;

use diesel::sql_types::Integer;

pub fn update_new_ratings(conn: &DbConn, players: &mut PlayersMap) {
    for g in get_games(conn) {
        let away_rating = players[&g.away].rating.clone();
        let home_rating = players[&g.home].rating.clone();

        {
            let home_player = players.get_mut(&g.home).unwrap();
            home_player.duel(g.dato.clone(), away_rating, g.home_win);
            if g.ace {
                if g.home_win {
                    home_player.aces += 1;
                } else {
                    home_player.eggs += 1;
                }
            }
        }
        {
            let away_player = players.get_mut(&g.away).unwrap();
            away_player.duel(g.dato, home_rating, !g.home_win);
            if g.ace {
                if g.home_win {
                    away_player.eggs += 1;
                } else {
                    away_player.aces += 1;
                }
            }
        }
    }
}

fn get_and_update_new_ratings(conn: &DbConn, players: &mut PlayersMap) -> Vec<PlayerData> {
    update_new_ratings(conn, players);

    let mut ps: Vec<_> = players.values().map(PlayerRating::to_data).filter(|p| p.kampe > 0).collect();
    ps.sort_by(|a, b| if b.rating.score < a.rating.score {
        Less
    } else {
        Greater
    });
    ps
}

#[get("/")]
pub fn root<'a>(conn: DbConn, players: Players) -> ContRes<'a> {
    let mut players = players.lock().unwrap();

    let mut context = create_context("root");
    context.insert("players", &get_and_update_new_ratings(&conn, &mut *players));

    let results: Vec<model::PlayedGameQuery> =
        diesel::sql_query("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, 
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score,
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id) as ball_img, 
                      (SELECT name FROM balls b WHERE ball_id = b.id) AS ball_name, dato FROM games g ORDER BY dato DESC LIMIT 5
                     ")
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

    context.insert("games", &games);
    
    respond_page("root", context)
}

#[derive(Debug, Serialize, Queryable, QueryableByName)]
struct Homeawaystats {
    #[sql_type = "Integer"]
    homewins: i32,
    #[sql_type = "Integer"]
    awaywins: i32,
    #[sql_type = "Integer"]
    homegoals: i32,
    #[sql_type = "Integer"]
    awaygoals: i32,
}

#[get("/ratings")]
pub fn ratings<'a>(conn: DbConn, players: Players) -> ContRes<'a> {
    let mut context = create_context("rating");
    {
        let mut players = players.lock().unwrap();
        context.insert("players", &get_and_update_new_ratings(&conn, &mut *players));
    }
    context.insert("ace_egg_modifier", &CONFIG.ace_egg_modifier);
    context.insert("streak_modifier", &CONFIG.streak_modifier);
    let homeawaystats: Vec<Homeawaystats> = 
        diesel::sql_query("
        select (select count(id) from games where dato > date('now', 'start of month') AND home_score > away_score) AS homewins,
        (select count(id) from games where dato > date('now', 'start of month') and home_score < away_score) as awaywins,
        (select coalesce(sum(home_score),0) ) as homegoals, (select coalesce(sum(away_score),0) ) as awaygoals from games
        where dato > date('now', 'start of month')
        ").load(&*conn).unwrap();

    println!("Stats: {:?}",homeawaystats);

    context.insert("homeawaystats", &homeawaystats);

    respond_page("ratings", context)
}

#[get("/reset/ratings")]
pub fn reset(conn: DbConn, players: Players) -> Redirect {
    players.reset(&conn);
    Redirect::to("/")
}
