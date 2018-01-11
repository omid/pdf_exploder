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
  let generate_image_thread = pdf.generate_images();

  // Generate texts
  let extract_texts_thread = pdf.extract_texts();

  generate_image_thread.join().expect("Oops");
  pdf.texts = extract_texts_thread.join().unwrap();

  // Send requests
//  pdf.send_result();

  // Cleanup
  pdf.cleanup();

//  println!("{:#?}", pdf.texts);
}
