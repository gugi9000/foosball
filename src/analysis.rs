use ::*;
use rocket::response::Responder;

#[get("/analysis")]
fn analysis<'a>() -> Res<'a> {
    TERA.render("pages/analysis.html", create_context("analysis")).respond()
}