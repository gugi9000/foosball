use ::*;

#[get("/analysis")]
fn analysis<'a>() -> ContRes<'a> {
    respond_page("analysis", create_context("analysis"))
}

#[get("/ratingsdev")]
fn ratingsdev<'a>() -> ContRes<'a> {
    let mut context = create_context("analysis");
    context.add("ace_egg_modifier", &CONFIG.ace_egg_modifier);
    context.add("streak_modifier", &CONFIG.streak_modifier);
   
    respond_page("ratingsdev", context)
}

use std::collections::BTreeMap;

#[get("/data/ratingsdev.tsv")]
fn developmenttsv() -> String {
    // HACK This seems a bit weird, but it works
    let mut ratings_history = BTreeMap::<String, HashMap<i32, f64>>::new();

    ratings::update_new_ratings();

    let mut data = "date".to_owned();
    let mut names = Vec::new();

    for (&id, player) in PLAYERS.lock().unwrap().iter().filter(|&(_, p)| p.kampe > 0) {
        data.push('\t');
        data.push_str(&player.name);
        names.push((id, player.name.clone()));

        for &(ref date, rating) in &player.score_history {
            let date = format!("{}{}{}T{}", &date[0..4], &date[5..7], &date[8..10], &date[11..16]);
            let ratings_for_date = ratings_history.entry(date).or_insert_with(HashMap::new);
            ratings_for_date.insert(id, rating);
        }
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
        conn.prepare("select ball_id, sum(home_score+away_score) as goals, count(ball_id) as balls, (select name from balls where ball_id = balls.id), (select img from balls where ball_id = balls.id) FROM games WHERE dato > date('now','start of month') GROUP BY ball_id order by balls desc , goals desc")
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
