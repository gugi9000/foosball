-- Your SQL goes here
CREATE TABLE players (
    id   INTEGER NOT NULL
                 PRIMARY KEY,
    name TEXT    UNIQUE
                 NOT NULL
);
CREATE TABLE balls (
    id   INTEGER NOT NULL
                 PRIMARY KEY,
    name TEXT    UNIQUE
                 NOT NULL,
    img  TEXT    NOT NULL
);
CREATE TABLE games (
    id         INTEGER NOT NULL
                       PRIMARY KEY,
    home_id    INTEGER NOT NULL
                       REFERENCES players (id) ON DELETE RESTRICT
                                               ON UPDATE CASCADE,
    away_id    INTEGER NOT NULL
                       REFERENCES players (id) ON DELETE RESTRICT
                                               ON UPDATE CASCADE,
    home_score INTEGER NOT NULL,
    away_score INTEGER NOT NULL,
    dato       TEXT    NOT NULL,
    ball_id    INTEGER NOT NULL
                       REFERENCES balls (id) ON DELETE RESTRICT
                                             ON UPDATE CASCADE
);
