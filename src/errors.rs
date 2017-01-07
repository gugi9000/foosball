use ::*;

#[error(404)]
fn page_not_found<'a>() -> Res<'a> {
    TERA.render("pages/404.html", create_context("404")).respond()
}

#[error(400)]
fn bad_request<'a>() -> Res<'a> {
    TERA.render("pages/400.html", create_context("400")).respond()
}

#[error(500)]
fn server_error<'a>() -> Res<'a> {
    TERA.render("pages/500.html", create_context("500")).respond()
}
