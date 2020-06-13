table! {
    balls (id) {
        id -> Integer,
        name -> Text,
        img -> Text,
    }
}

table! {
    games (id) {
        id -> Integer,
        home_id -> Integer,
        away_id -> Integer,
        home_score -> Integer,
        away_score -> Integer,
        dato -> Text,
        ball_id -> Integer,
    }
}

table! {
    players (id) {
        id -> Integer,
        name -> Text,
    }
}

joinable!(games -> balls (ball_id));

allow_tables_to_appear_in_same_query!(
    balls,
    games,
    players,
);
