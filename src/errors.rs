use crate::*;

#[catch(404)]
fn page_not_found<'a>() -> ContRes<'a> {
    respond_page("404", create_context("404"))
}

#[catch(400)]
fn bad_request<'a>() -> ContRes<'a> {
    respond_page("400", create_context("400"))
}

#[catch(500)]
fn server_error<'a>() -> ContRes<'a> {
    respond_page("500", create_context("500"))
}
