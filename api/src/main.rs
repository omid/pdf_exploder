#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate serde_derive;
#[macro_use] extern crate rocket;
extern crate rocket_contrib;

extern crate presentation;

use presentation::Presentation;
use rocket_contrib::json::Json;

#[derive(Deserialize)]
struct DownloadData {
    #[serde(rename = "type")]
    _type: String,
    url: String,
}

#[derive(Deserialize)]
struct Callback {
    url: String,
}

#[derive(Deserialize)]
struct UploadData {
    url: String,
    callback: Callback,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ConversionParams {
    preserveTransparency: Option<bool>,
}

#[derive(Deserialize)]
#[allow(non_snake_case)]
struct ConvertData {
    downloadData: DownloadData,
    uploadData: UploadData,
    conversionParams: Option<ConversionParams>,
}

#[post("/convert", format = "application/json", data = "<data>")]
fn convert_presentation(data: Json<ConvertData>) -> String {
    let mut presentation = Presentation::new(data.downloadData.url.clone());

    // Download Presentation file
    presentation.download();
    presentation.extract_pages();

    // Generate images
    let conversion_params = data.conversionParams.as_ref().unwrap_or(&ConversionParams {
        preserveTransparency: Option::Some(false),
    });
    let preserve_transparency = conversion_params.preserveTransparency.unwrap_or(false);

    let generate_image_thread = presentation.generate_images(preserve_transparency);
    let extract_texts_thread = presentation.extract_texts();

    generate_image_thread.join();
    extract_texts_thread.join();

    // Send requests
    let upload_slide_requests = presentation.send_slides(data.uploadData.url.clone());
    upload_slide_requests.join();

    presentation.send_ack("success", "message", data.uploadData.callback.url.clone());

    // Cleanup
    presentation.cleanup();

    format!("Successfully extracted {} slides", presentation.number_of_pages)
}

fn main() {
    rocket::ignite().mount("/", routes![convert_presentation]).launch();
}
