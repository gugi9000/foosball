use rocket::get;

use crate::*;

#[get("/ratingsdev")]
pub fn ratingsdev() -> ResHtml {
    let mut context = create_context("analysis");
    context.insert("ace_egg_modifier", &CONFIG.ace_egg_modifier);
    context.insert("streak_modifier", &CONFIG.streak_modifier);

    respond_page("ratingsdev", context)
}

use std::collections::BTreeMap;

#[get("/data/ratingsdev.tsv")]
pub fn developmenttsv() -> String {
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
            let date = format!(
                "{}{}{}T{}",
                &date[0..4],
                &date[5..7],
                &date[8..10],
                &date[11..16]
            );
            let ratings_for_date = ratings_history.entry(date).or_default();
            ratings_for_date.insert(id, rating);
        }
    }
    data.push('\n');

    let mut cache = HashMap::new();

    for (date, player_ratings) in ratings_history.into_iter() {
        let mut line = date;
        for (id, _) in &names {
            line.push('\t');
            let rating = if let Some(rating) = player_ratings.get(id).copied() {
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

    if data.len() < 10 {
        "date\tNoone\n20190101T00:00\t0.0".to_owned()
    } else {
        data
    }
}
