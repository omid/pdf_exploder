#[macro_use] extern crate lambda_runtime as lambda;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;
extern crate simple_logger;

use lambda::error::HandlerError;

use std::error::Error;

extern crate presentation;

use presentation::Presentation;
//use presentation::RequestBody;

#[derive(Deserialize, Clone)]
struct CustomEvent {
    #[serde(rename = "firstName")]
    first_name: String,
}

#[derive(Serialize, Clone)]
struct CustomOutput {
    message: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    simple_logger::init_with_level(log::Level::Info)?;
    lambda!(my_handler);

    Ok(())
}

fn my_handler(e: CustomEvent, c: lambda::Context) -> Result<CustomOutput, HandlerError> {
    if e.first_name == "" {
        error!("Empty first name in request {}", c.aws_request_id);
        return Err(c.new_error("Empty first name"));
    }

    let mut presentation = Presentation::new(e.first_name.clone());

//    presentation.extract(request_body.into_inner())

    Ok(CustomOutput {
        message: format!("Hello, {}!", e.first_name),
    })
}
