use ::*;

#[get("/analysis")]
fn analysis<'a>() -> ContRes<'a> {
    respond_page("analysis", create_context("analysis"))
}

#[get("/ratingsdev")]
fn ratingsdev<'a>() -> ContRes<'a> {
    respond_page("ratingsdev", create_context("analysis"))
}

use std::collections::BTreeMap;

#[get("/data/ratingsdev.tsv")]
fn developmenttsv() -> String {
    // HACK This seems a bit weird, but it works
    let mut ratings_history = BTreeMap::<String, HashMap<i32, f64>>::new();

    for (&id, player) in PLAYERS.lock().unwrap().iter() {
        for &(ref date, ref rating) in &player.ratings_history {
            let date = format!("{}{}{}T{}", &date[0..4], &date[5..7], &date[8..10], &date[11..16]);
            let ratings_for_date = ratings_history.entry(date).or_insert_with(HashMap::new);
            ratings_for_date.insert(id, rating.get_rating());
        }
    }

    let names: Vec<_> = PLAYERS.lock().unwrap().iter().map(|(&id, p)| (id, p.name.clone())).collect();

    let mut data = "date".to_owned();

    for &(_, ref name) in &names {
        data.push('\t');
        data.push_str(name);
    }
    data.push('\n');

    let mut cache = HashMap::new();

    for (date, player_ratings) in ratings_history.into_iter() {
        let mut line = date;
        for &(ref id, _) in &names {
            line.push('\t');
            let rating = if let Some(rating) = player_ratings.get(id).map(|&f| f) {
                cache.insert(id, rating);
                rating
            } else {
                *cache.entry(id).or_insert(0.)
            };
            line.push_str(&format!("{:.1}", rating));
        }
        data.push_str(&line);
        data.push('\n');
    }

    data
}

#[derive(Debug, Serialize)]
struct Ballstats {
    name: String,
    games: i32,
    goals: i32,
    img: String,
}

#[get("/analysis/balls")]
fn ballstats<'a>() -> ContRes<'a> {
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

    let mut context = create_context("analysis");

    context.add("ballstats", &ballstats);
    respond_page("ballstats", context)
}

#[derive(Debug, Serialize)]
struct Homeawaystats {
    homewins: i32,
    awaywins: i32,
    homegoals: i32,
    awaygoals: i32,
}
#[get("/analysis/homeaway")]
fn homeaway<'a>() -> ContRes<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("select (select count(id) from games where home_score > away_score) as homewins, (select count(id) from games where home_score < away_score) as awaywins, (select sum(home_score) ) as homegoals, (select sum(away_score) ) as awaygoals from games;")
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
    let mut context = create_context("analysis");

    context.add("homeawaystats", &homeawaystats);
    respond_page("homeawaystats", context)
}

#[get("/analysis/pvp")]
fn pvpindex<'a>() -> ContRes<'a> {
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

    respond_page("pvpindex", context)
}

#[get("/analysis/pvp/<p1>/<p2>")]
fn pvp<'a>(p1: i32, p2: i32) -> ContRes<'a> {
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



    let mut context = create_context("analysis");

    context.add("pvp", &pvp);
    context.add("map", &map);
    respond_page("pvp", context)
}
