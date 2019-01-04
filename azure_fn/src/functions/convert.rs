use azure_functions::func;
use azure_functions::bindings::{HttpRequest, HttpResponse};

extern crate presentation;

use presentation::Presentation;
use presentation::RequestBody;

// The func attribute marks this fn as the function to be used by Azure Functions.
#[func]
// See https://docs.microsoft.com/en-us/azure/azure-functions/functions-triggers-bindings#supported-bindings
// See also https://github.com/peterhuene/azure-functions-rs/blob/master/README.md#azure-functions-bindings
#[binding(name="request", auth_level="anonymous")]
// The function will just check for a name parameter in the querystring
// or for a JSON Body structure in the body of the request.
pub fn convert(request: &HttpRequest) -> HttpResponse {
    // Logs the request on the Azure Functions Host.
    info!("Request: {:?}", request);

    // checking the body
    if let Ok(request_body) = request.body().as_json::<RequestBody>() {
        Presentation::extract(request_body);

        return r#"{
	        "message": "OK"
        }"#.into()
    }

    return r#"{
	        "message": "Cannot generate slides without a proper input!"
        }"#.into()
}
