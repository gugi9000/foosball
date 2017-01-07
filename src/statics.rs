use ::*;
//use rocket::response::Responder;

#[get("/static/<file..>")]
fn static_handler(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).ok()
}

#[get("/favicon.ico")]
fn favicon_handler() -> Option<NamedFile> {
    static_handler(PathBuf::new().join("dynateam.ico"))
}