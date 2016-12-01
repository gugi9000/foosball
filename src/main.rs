extern crate pencil;

use pencil::Pencil;
use pencil::{Request, PencilResult, Response};
use pencil::method::Get;
use pencil::HTTPError;
use std::collections::BTreeMap;

fn index_template(request: &mut Request) -> PencilResult {
    let mut context = BTreeMap::new();
    context.insert("heading".to_string(), "Top 4".to_string());
    context.insert("body".to_string(), "Alle ratings".to_string());
    return request.app.render_template("index.html", &context);
}

fn page_not_found(_: HTTPError) -> PencilResult {
    let mut response = Response::from("Uh-ohh, 404 :O");
    response.status_code = 404;
    Ok(response)
}

// fn hello(_: &mut Request) -> PencilResult {
//    Ok(Response::from("Hello World!"))
// }


fn main() {
    let mut app = Pencil::new("");
    app.register_template("index.html");
    app.get("/", "index_template", index_template);
    //    app.route("/", &[Get], "hello", hello);
    app.enable_static_file_handling();
    app.httperrorhandler(404, page_not_found);
    app.run("127.0.0.1:5000");
}
