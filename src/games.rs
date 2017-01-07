use ::*;
use rand::Rng;
use rocket::response::Responder;

#[get("/newgame")]
fn newgame<'a>() -> Res<'a> {
    TERA.render("pages/newgame.html", newgame_con()).respond()
}

fn newgame_con() -> Context {
    let conn = DB_CONNECTION.lock().unwrap();
    let mut stmt = conn.prepare("SELECT id, name FROM players").unwrap();
    let mut names: Vec<_> = stmt.query_map(&[], |row| {
            Named {
                id: row.get(0),
                name: row.get(1)
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();

    names.sort_by(|_, _| *rand::thread_rng().choose(&[Greater, Less]).unwrap());

    let mut ballstmt = conn.prepare("SELECT id, name, img FROM balls").unwrap();
    let balls: Vec<_> = ballstmt.query_map(&[], |row| {
            Ball {
                id: row.get(0),
                name: row.get(1),
                img: row.get(2),
            }
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();


    let mut context = create_context("newgame");

    context.add("names", &names);
    context.add("balls", &balls);
    context
}

struct NewGame {
    home: i32,
    away: i32,
    home_score: i32,
    away_score: i32,
    ball: i32,
    secret: String,
}

impl<'a> FromForm<'a> for NewGame {
    type Error = &'a str;
    fn from_form_string(form_string: &'a str) -> Result<Self, Self::Error> {
        let mut home = FromFormValue::default();
        let mut away = FromFormValue::default();
        let mut home_score = FromFormValue::default();
        let mut away_score = FromFormValue::default();
        let mut ball = FromFormValue::default();
        let mut secret = FromFormValue::default();
        for (k, v) in FormItems(form_string) {
            match k {
                "home" => home = Some(i32::from_form_value(v)?),
                "away" => away = Some(i32::from_form_value(v)?),
                "home_score" => home_score = Some(i32::from_form_value(v)?),
                "away_score" => away_score = Some(i32::from_form_value(v)?),
                "ball" => ball = Some(i32::from_form_value(v)?),
                "secret" => secret = Some(String::from_form_value(v)?),
                _ => (),
            }
        }
        Ok(NewGame {
            home: home.ok_or("no `home` found")?,
            away: away.ok_or("no `away` found")?,
            home_score: home_score.ok_or("no `home_score` found")?,
            away_score: away_score.ok_or("no `away_score` found")?,
            ball: ball.ok_or("no `ball` found")?,
            secret: secret.ok_or("no `secret` found")?,
        })
    }
}

#[post("/newgame/submit", data = "<f>")]
fn submit_newgame<'a>(f: Form<NewGame>) -> Res<'a> {
    let conn = DB_CONNECTION.lock().unwrap();
    let f = f.into_inner();

    if f.secret != CONFIG.secret {
        let mut context = newgame_con();
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
        return TERA.render("pages/newgame_fejl.html", context).respond();
    }

    if !(f.home_score == 10 || f.away_score == 10) || f.home_score == f.away_score ||
       f.home == f.away {
        let mut context = newgame_con();

        context.add("fejl",
                       &"Den indtastede kamp er ikke lovlig 😜");
        return TERA.render("pages/newgame_fejl.html", context).respond();
    }

    let res = conn.execute("INSERT INTO games (home_id, away_id, home_score, away_score, dato, \
                            ball_id) VALUES (?, ?, ?, ?, datetime('now'), ?)",
                           &[&f.home, &f.away, &f.home_score, &f.away_score, &f.ball]);
    println!("{:?}", res);

    Redirect::to("/").respond()
}

#[get("/games")]
fn games<'a>() -> Res<'a> {
    let conn = DB_CONNECTION.lock().unwrap();
    let mut stmt =
        conn.prepare("SELECT (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
                      (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, \
                      away_score, ball_id, (SELECT img FROM balls b WHERE ball_id = b.id), \
                      (SELECT name FROM balls b WHERE ball_id = b.id), dato FROM games g ORDER BY ID DESC")
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

    let mut context = create_context("games");

    context.add("games", &games);
    TERA.render("pages/games.html", context).respond()
}
