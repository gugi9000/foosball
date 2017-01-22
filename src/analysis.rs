use ::*;
use rocket::response::Responder;

#[get("/analysis")]
fn analysis<'a>() -> Res<'a> {
    TERA.render("pages/analysis.html", create_context("analysis")).respond()
}

#[get("/ratingsdev")]
fn ratingsdev<'a>() -> Res<'a> {
    TERA.render("pages/ratingsdev.html", create_context("analysis")).respond()
}

#[derive(Debug, Serialize)]
struct Ballstats {
    name: String,
    games: i32,
    goals: i32,
    img: String,
}

#[get("/analysis/balls")]
fn ballstats<'a>() -> Res<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("select ball_id, sum(home_score+away_score) as goals, count(ball_id) as balls, (select name from balls where ball_id = balls.id), (select img from balls where ball_id = balls.id) FROM games WHERE dato > datetime('now', '-90 day') GROUP BY ball_id order by balls desc , goals desc")
            .unwrap();
    let ballstats: Vec<_> = stmt.query_map(&[], |row| {
        Ballstats { 
            name: row.get(3),
            img: row.get(4),
            games: row.get(2),
            goals: row.get(1),
        }
    })
    .unwrap()
    .map(Result::unwrap)
    .collect();

    let mut context = create_context("ballstats");

    context.add("ballstats", &ballstats);
    TERA.render("pages/ballstats.html", context).respond()
}

#[derive(Debug, Serialize)]
struct Homeawaystats {
    homewins: i32,
    awaywins: i32,
    homegoals: i32,
    awaygoals: i32,
}
#[get("/analysis/homeaway")]
fn homeaway<'a>() -> Res<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("select (select count(id) from games where home_score > away_score) as homewins, (select count(id) from games where home_score < away_score) as awaywins, (select sum(home_score) where home_score < away_score ) as homegoals, (select sum(away_score) where home_score < away_score ) as awaygoals from games;")
            .unwrap();
    let homeawaystats: Vec<_> = stmt.query_map(&[], |row| {
        Homeawaystats { 
            homewins: row.get(0),
            awaywins: row.get(1),
            homegoals: row.get(2),
            awaygoals: row.get(3),
        }
    })
    .unwrap()
    .map(Result::unwrap)
    .collect();
    let mut context = create_context("homeawaystats");

    context.add("homeawaystats", &homeawaystats);
    TERA.render("pages/homeawaystats.html", context).respond()
}

#[get("/analysis/pvp")]
fn pvpindex<'a>() -> Res<'a> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name FROM players order by name").unwrap();
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
                      and dato > datetime('now', '-90 day') ORDER BY dato DESC")
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