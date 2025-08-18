use rocket::get;

use crate::*;

pub fn update_new_ratings() {
    let mut players = PLAYERS.lock().unwrap();

    for g in get_games() {
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

fn get_and_update_new_ratings() -> Vec<PlayerData> {
    update_new_ratings();
    let players = PLAYERS.lock().unwrap();

    let mut ps: Vec<_> = players.values().map(PlayerRating::to_data).filter(|p| p.kampe > 0).collect();
    ps.sort_by(|a, b| if b.rating.score < a.rating.score {
        Less
    } else {
        Greater
    });
    ps
}

#[get("/")]
pub fn root<'a>() -> ResHtml<'a> {
    let mut context = create_context("root");
    context.insert("players", &get_and_update_new_ratings());

    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, 
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score,
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), 
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g ORDER BY dato DESC LIMIT 5
                     ")
            .unwrap();
    let games: Vec<_> = stmt.query_map((), |row| {
        Ok(PlayedGame {
            home: row.get(0)?,
            away: row.get(1)?,
            home_score: row.get(2)?,
            away_score: row.get(3)?,
            ball: row.get(5)?,
            ball_name: row.get(6)?,
            dato: row.get(7)?,
        })
    })
    .unwrap()
    .map(Result::unwrap)
    .collect();

    context.insert("games", &games);
    
    respond_page("root", context)
}

#[derive(Debug, Serialize)]
struct Homeawaystats {
    homewins: i32,
    awaywins: i32,
    homegoals: i32,
    awaygoals: i32,
}

#[get("/ratings")]
pub fn ratings<'a>() -> ResHtml<'a> {
    let mut context = create_context("rating");
    context.insert("players", &get_and_update_new_ratings());
    context.insert("ace_egg_modifier", &CONFIG.ace_egg_modifier);
    context.insert("streak_modifier", &CONFIG.streak_modifier);
    let conn = lock_database();
    let mut stmt =
        conn.prepare("
        select (select count(id) from games where dato > date('now', 'start of month') AND home_score > away_score) AS homewins,
        (select count(id) from games where dato > date('now', 'start of month') and home_score < away_score) as awaywins,
        (select coalesce(sum(home_score),0) ) as homegoals, (select coalesce(sum(away_score),0) ) as awaygoals from games
        where dato > date('now', 'start of month')
        ").unwrap();

    let homeawaystats: Vec<_> = stmt.query_map((), |row| {
        Ok(Homeawaystats {
            homewins: row.get(0)?,
            awaywins: row.get(1)?,
            homegoals: row.get(2)?,
            awaygoals: row.get(3)?,
        })
    })
    .unwrap()
    .map(Result::unwrap)
    .collect();
    println!("Stats: {:?}",homeawaystats);

    context.insert("homeawaystats", &homeawaystats);

    respond_page("ratings", context)
}

#[get("/reset/ratings")]
pub fn reset() -> Redirect {
    reset_ratings();
    Redirect::to("/")
}
