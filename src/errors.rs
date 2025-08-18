use rocket::catch;

use crate::*;

#[catch(404)]
pub fn page_not_found<'a>() -> ResHtml {
    respond_page("404", create_context("404"))
}

#[catch(400)]
pub fn bad_request<'a>() -> ResHtml {
    respond_page("400", create_context("400"))
}

#[catch(500)]
pub fn server_error<'a>() -> ResHtml {
    respond_page("500", create_context("500"))
}
