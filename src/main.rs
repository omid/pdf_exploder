#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
#[macro_use] extern crate serde_derive;

extern crate pdf;

use rocket_contrib::{Json};
use pdf::pdf::Pdf;

#[derive(Serialize, Deserialize)]
struct DownloadData {
  #[serde(rename="type")]
  _type: String,
  url: String
}

#[derive(Serialize, Deserialize)]
struct Callback {
  url: String,
  method: String
}

#[derive(Serialize, Deserialize)]
struct UploadData {
  url: String,
  callback: Callback
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct ConversionParams {
  preserveTransparency: Option<bool>
}

#[derive(Serialize, Deserialize)]
#[allow(non_snake_case)]
struct ConvertPDFData {
  downloadData: DownloadData,
  uploadData: UploadData,
  conversionParams: Option<ConversionParams>
}

#[post("/convert", format = "application/json", data = "<data>")]
fn convert_pdf(data: Json<ConvertPDFData>) -> &'static str {
  let mut pdf = Pdf::new(data.downloadData.url.clone());

  // Download PDF file
  pdf.download();

  // Generate images
  let conversion_params = data.conversionParams.as_ref().unwrap_or(&ConversionParams { preserveTransparency: Option::Some(false) });
  let preserve_transparency = conversion_params.preserveTransparency.unwrap_or(false);

  let generate_image_thread = pdf.generate_images(preserve_transparency);

  // Generate texts
  let extract_texts_thread = pdf.extract_texts();

  generate_image_thread.join().expect("Couldn't extract images");
  pdf.texts = extract_texts_thread.join().expect("Couldn't extract texts");

  // Send requests
  let upload_slide_requests = pdf.send_slides(data.uploadData.url.clone());

  // @TODO when all requests are finished, send another request
  // upload_slide_requests.join().expect("Couldn't upload all slides");
  // pdf.send_ack(data.uploadData.callback.url.clone());

  // Cleanup #
  pdf.cleanup();

  "Successfully converted image files"
}

fn main() {
  rocket::ignite().mount("/", routes![convert_pdf]).launch();
}
