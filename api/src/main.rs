#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate rocket_contrib;

extern crate presentation;

use presentation::Presentation;
use presentation::RequestBody;
use rocket_contrib::json::Json;

#[post("/convert", format = "application/json", data = "<request_body>")]
fn convert_presentation(request_body: Json<RequestBody>) -> String {
    let mut presentation = Presentation::new(request_body.downloadData.url.clone());

    presentation.extract(request_body.into_inner())
}

fn main() {
    rocket::ignite().mount("/", routes![convert_presentation]).launch();
}
