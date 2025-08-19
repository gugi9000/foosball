use rocket::get;
use std::io::Write;

use crate::{ext::RawTextTsv, *};

#[get("/ratingsdev")]
pub fn ratingsdev() -> ResHtml {
    let mut context = create_context("analysis");
    context.insert("ace_egg_modifier", &CONFIG.ace_egg_modifier);
    context.insert("streak_modifier", &CONFIG.streak_modifier);

    respond_page("ratingsdev", context)
}

use std::collections::BTreeMap;

#[get("/data/ratingsdev.tsv")]
pub fn developmenttsv() -> RawTextTsv<Vec<u8>> {
    // HACK This seems a bit weird, but it works
    let mut ratings_history = BTreeMap::<_, HashMap<i32, f64>>::new();

    ratings::update_new_ratings();

    let mut out_text = b"date".to_vec(); 
    let w = &mut out_text;
    let mut names = Vec::new();

    for (&id, player) in PLAYERS.lock().unwrap().iter().filter(|&(_, p)| p.kampe > 0) {
        write!(w, "\t{}", player.name).unwrap();
        names.push((id, player.name.clone()));

        for &(date, rating) in &player.score_history {
            let ratings_for_date = ratings_history.entry(date).or_default();
            ratings_for_date.insert(id, rating);
        }
    }
    writeln!(w).unwrap();

    let mut cache = HashMap::new();

    for (date, player_ratings) in ratings_history {
        write!(w, "{}", date.format("%Y%m%dT%H:%M")).unwrap();
        for &(id, _) in &names {
            write!(w, "\t").unwrap();
            let rating = if let Some(rating) = player_ratings.get(&id).copied() {
                cache.insert(id, rating);
                rating
            } else {
                *cache.entry(id).or_insert(0.)
            };
            write!(w, "{rating:.1}").unwrap();
        }
        writeln!(w).unwrap();
    }

    RawTextTsv(if out_text.len() < 10 {
        b"date\tNoone\n20190101T00:00\t0.0".to_vec()
    } else {
        out_text
    })
}
