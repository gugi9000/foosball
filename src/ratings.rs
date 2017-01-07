use ::*;
use rocket::response::Responder;

#[get("/")]
fn root<'a>() -> Res<'a> {
    ratings::rating()
}

#[get("/rating")]
fn rating<'a>() -> Res<'a> {
    let mut players = PLAYERS.lock().unwrap();

    for g in get_games() {
        let away_rating = players[&g.away].rating.0.clone();
        let home_rating = players[&g.home].rating.0.clone();

        {
            let home_player = players.get_mut(&g.home).unwrap();
            home_player.duel(&RATER, away_rating, g.home_win);
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
            away_player.duel(&RATER, home_rating, !g.home_win);
            if g.ace {
                if g.home_win {
                    away_player.eggs += 1;
                } else {
                    away_player.aces += 1;
                }
            }
        }
    }

    let mut ps: Vec<_> = players.values().collect();
    ps.sort_by(|a, b| if b.rating.get_rating() < a.rating.get_rating() {
        Less
    } else {
        Greater
    });
    ps.retain(|a| a.kampe != 0);
    let mut context = create_context("rating");
    context.add("players", &ps);

    TERA.render("pages/rating.html", context).respond()
}

#[get("/reset/ratings")]
fn reset() -> Redirect {
    reset_ratings();
    Redirect::to("/")
}