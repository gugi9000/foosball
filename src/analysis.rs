use ::*;
use rocket::response::Responder;

#[get("/analysis")]
fn analysis<'a>() -> Res<'a> {
    TERA.render("pages/analysis.html", create_context("analysis")).respond()
}

#[get("/analysis/balls")]
fn balls<'a>() -> Res<'a> {
    TERA.render("pages/analysis.html", create_context("analysis")).respond()
}

#[get("/analysis/homeaway")]
fn homeaway<'a>() -> Res<'a> {
    TERA.render("pages/analysis.html", create_context("analysis")).respond()
}

#[get("/analysis/pvp")]
fn pvpindex<'a>() -> Res<'a> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name FROM players").unwrap();
    let names: Vec<_> = stmt.query_map(&[], |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    let mut context = create_context("analysis");
    context.add("names", &names);

    TERA.render("pages/pvpindex.html", context).respond()
}

#[get("/analysis/pvp/<p1>/<p2>")]
fn pvp<'a>(p1: i32, p2: i32) -> Res<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato, home_id FROM games g \
                      WHERE (home_id = ?1 and away_id = ?2) or (home_id = ?2 and away_id = ?1) \
                      ORDER BY dato DESC")
            .unwrap();
    
    let mut map = ((0, 0, "".to_owned()), (0, 0, "".to_owned()));

    let pvp: Vec<_> = stmt.query_map(&[&p1, &p2], |row| {
            let game = PlayedGame {
                home: row.get(0),
                away: row.get(1),
                home_score: row.get(2),
                away_score: row.get(3),
                ball: row.get(5),
                ball_name: row.get(6),
                dato: row.get(7),
            };
            let home_id: i32 = row.get(8);
            
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
        .unwrap()
        .map(Result::unwrap)
        .collect();

    print!("{:?}", map);


//      p1  mål     p1 vundne   p1 navn     p1 antal kampe
//      p2  mål     p2 vundne   p2 navn     p2 antal kampe



    let mut context = create_context("pvp");

    context.add("pvp", &pvp);
    context.add("map", &map);
    TERA.render("pages/pvp.html", context).respond()
}