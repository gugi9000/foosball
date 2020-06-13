use diesel;
use diesel::prelude::*;
use diesel::sql_types::{Text, Integer};
use diesel::SqliteConnection;
use crate::schema::*;

#[table_name = "players"]
#[derive(Debug, Serialize, AsChangeset, Queryable, QueryableByName)]
pub struct Player {
    pub id: i32,
    pub name: String,
}

impl Player {
    pub fn read_all(connection: &SqliteConnection) -> Vec<Player> {
        players::table.load(connection).unwrap()
    }
    pub fn read_all_ordered(connection: &SqliteConnection) -> Vec<Player> {
        players::table.order(players::name.asc()).load(connection).unwrap()
    }
}
#[table_name = "players"]
#[derive(Debug, Queryable, Insertable)]
pub struct InsertablePlayer {
    pub name: String,
}
impl InsertablePlayer {
    pub fn insert(self, connection: &SqliteConnection) -> usize {
        diesel::insert_into(players::table).values(self).execute(connection).unwrap()
    }
}

#[table_name = "balls"]
#[derive(Debug, Serialize, AsChangeset, Queryable)]
pub struct Ball {
    pub id: i32,
    pub name: String,
    pub img: String,
}

impl Ball {
    pub fn read_all(connection: &SqliteConnection) -> Vec<Ball> {
        balls::table.load(connection).unwrap()
    }
    pub fn read_all_ordered(connection: &SqliteConnection) -> Vec<Ball> {
        balls::table.order(balls::name.asc()).load(connection).unwrap()
    }
}
#[table_name = "balls"]
#[derive(Clone, Debug, Queryable, Insertable)]
pub struct InsertableBall {
    pub name: String,
    pub img: String,
}
impl InsertableBall {
    pub fn insert(self, connection: &SqliteConnection) -> usize {
        diesel::insert_into(balls::table).values(self).execute(connection).unwrap()
    }
}

#[table_name = "games"]
#[derive(Debug, AsChangeset, Queryable)]
pub struct GameForScores {
    pub home_id: i32,
    pub away_id: i32,
    pub home_score: i32,
    pub away_score: i32,
    pub dato: String,
}

sql_function! {
    fn datetime(a: Text) -> Text;
}

sql_function! {
    #[sql_name = "datetime"]
    fn datetime2(a: Text, b: Text) -> Text;
}

impl GameForScores {
    pub fn read_all(connection: &SqliteConnection, last_date: &str) -> Vec<GameForScores> {
        use games::*;
        table.select((home_id, away_id, home_score, away_score, dato)).filter(dato.gt(datetime(last_date))).order(dato.asc()).load(connection).unwrap()
    }
    pub fn read_all_from_start_of_month(connection: &SqliteConnection) -> Vec<GameForScores> {
        use games::*;
        table.select((home_id, away_id, home_score, away_score, dato)).filter(dato.gt(datetime2("now","start of month"))).order(dato.asc()).load(connection).unwrap()
    }
}

pub type Named = Player;


#[derive(Debug, Queryable, QueryableByName)]
pub struct PlayedGameQueryWithHomeId {
    #[sql_type = "Text"]
    pub home: String,
    #[sql_type = "Text"]
    pub away: String,
    #[sql_type = "Integer"]
    pub home_score: i32,
    #[sql_type = "Integer"]
    pub away_score: i32,
    #[sql_type = "Integer"]
    pub ball_id: i32,
    #[sql_type = "Text"]
    pub ball_img: String,
    #[sql_type = "Text"]
    pub ball_name: String,
    #[sql_type = "Text"]
    pub dato: String,
    #[sql_type = "Integer"]
    pub home_id: i32,
}

#[derive(Debug, Queryable, QueryableByName)]
pub struct PlayedGameQuery {
    #[sql_type = "Text"]
    pub home: String,
    #[sql_type = "Text"]
    pub away: String,
    #[sql_type = "Integer"]
    pub home_score: i32,
    #[sql_type = "Integer"]
    pub away_score: i32,
    #[sql_type = "Integer"]
    pub ball_id: i32,
    #[sql_type = "Text"]
    pub ball_img: String,
    #[sql_type = "Text"]
    pub ball_name: String,
    #[sql_type = "Text"]
    pub dato: String,
}