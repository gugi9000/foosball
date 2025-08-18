use rocket::get;

use crate::*;

#[get("/static/<file..>")]
pub async fn static_handler(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/").join(file)).await.ok()
}

#[get("/favicon.ico")]
pub async fn favicon_handler() -> Option<NamedFile> {
    static_handler(PathBuf::new().join("dynateam.ico")).await
}

#[get("/robots.txt")]
pub async fn robots_handler() -> Option<NamedFile> {
    static_handler(PathBuf::new().join("robots.txt")).await
}
