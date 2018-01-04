use std::env;

extern crate pdf_exploder;

use pdf_exploder::pdf::Pdf;

fn main() {
  let args: Vec<String> = env::args().collect();
  let pdf_file: &String = &args[1];

  let mut pdf = Pdf::new(pdf_file.to_owned());

  // Download PDF file
  pdf.download();

  // Generate images
  pdf.generate_images();

  // Generate texts
  pdf.extract_texts();

  // Send requests
  pdf.send_result();

  println!("{:#?}", pdf);
}
