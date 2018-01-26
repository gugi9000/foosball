use ::*;

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

pub fn get_and_update_new_ratings() -> Vec<PlayerData> {
    update_new_ratings();
    let players = PLAYERS.lock().unwrap();

    let mut ps: Vec<_> = players.values().map(PlayerRating::to_data).filter(|p| p.kampe > 0).collect();
    ps.sort_by(|a, b| if b.rating.get_score() < a.rating.get_score() {
        Less
    } else {
        Greater
    });
    ps
}

#[get("/")]
fn root<'a>() -> ContRes<'a> {
    let mut context = create_context("root");
    context.add("players", &get_and_update_new_ratings());

    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g ORDER BY dato DESC LIMIT 5")
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

    context.add("games", &games);

     respond_page("root", context)
}

#[get("/ratings")]
fn ratings<'a>() -> ContRes<'a> {
    let mut context = create_context("rating");
    context.add("players", &get_and_update_new_ratings());

    respond_page("ratings", context)
}

#[get("/reset/ratings")]
fn reset() -> Redirect {
    reset_ratings();
    Redirect::to("/")
}
