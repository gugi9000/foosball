use ::*;

#[derive(Debug, Serialize)]
struct Ballstats {
    name: String,
    games: i32,
    goals: i32,
    img: String,
}

#[get("/balls")]
fn balls<'a>() -> ContRes<'a> {
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
    let mut context = create_context("balls");
    context.add("ballstats", &ballstats);

    respond_page("balls", context)
}

#[get("/ball/<ball>")]
fn ball<'a>(ball:String) -> ContRes<'a> {
    let conn = lock_database();
    let mut stmt =
        conn.prepare("SELECT \
            (SELECT name FROM players p WHERE p.id = g.home_id) AS home, \
            (SELECT name FROM players p WHERE p.id = g.away_id) AS away, home_score, away_score, ball_id,  \
            (SELECT img FROM balls b WHERE ball_id = b.id), \
            (SELECT name FROM balls b WHERE ball_id = b.id) AS ballname, \
            dato \
            FROM games g  \
            WHERE ballname = ?1 AND dato > date('now','start of month') \
            ORDER BY ID DESC")
            .unwrap();
    let games: Vec<_> = stmt.query_map(&[&ball], |row| {
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

    let mut context = create_context("balls");
    if games.len() != 0 {
        println!("Ukendt bold: {}",ball);
    }

    context.add("games", &games);
    context.add("ball", &ball);
    respond_page("ball", context)
}

#[get("/newball")]
fn newball<'a>() -> ContRes<'a> {
    let conn = lock_database();
    let mut stmt = conn.prepare("SELECT id, name, img from balls ORDER BY name ASC").unwrap();

    let mut balls = Vec::new();

    for ball in stmt.query_map(&[], |row| (row.get::<_, i32>(0), row.get::<_, String>(1), row.get::<_, String>(2))).unwrap() {
        let (id, name, img) = ball.unwrap();
        balls.push(Ball{id: id, name: name, img: img});
    }

    let mut context = create_context("balls");
    context.add("balls", &balls);
    
    respond_page("newball", context)
}

#[derive(FromForm)]
struct NewBallQuery {
    name: String,
    img: String,
    secret: String,
    #[allow(dead_code)]
    submit: IgnoreField
}

#[post("/newball/submit", data = "<f>")]
fn submit_newball<'r>(f: Form<NewBallQuery>) -> Resp<'r> {
    let NewBallQuery{name, img, secret, ..} = f.into_inner();

    let mut context = create_context("balls");
    if secret != CONFIG.secret {
        context.add("fejl", &"Det indtastede kodeord er forkert 💩");
    } else if name.is_empty() {
        context.add("fejl", &"Den indtastede bold er ikke lovlig 😜");
    } else {
        let n = lock_database().execute("INSERT INTO balls (name, img) VALUES (?, ?)", &[&img, &name]).unwrap();

        if n == 0 {
            context.add("fejl", &"Den indtastede bold eksisterer allerede 💩");
        } else {
            reset_ratings();
            return Resp::red(Redirect::to("/"));
        }
    }
    Resp::cont(respond_page("newball_fejl", context))
}
