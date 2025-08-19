use rocket::{http::ContentType, response::{self, Responder}, Request};

#[derive(Debug, Clone, PartialEq)]
pub struct RawTextTsv<R>(pub R);

impl<'r, 'o: 'r, R: Responder<'r, 'o>> Responder<'r, 'o> for RawTextTsv<R> {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        (ContentType::new("text", "tab-separated-values").with_params(("charset", "utf-8")), self.0).respond_to(req)
    }
}
