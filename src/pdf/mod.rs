use std::fs;
use std::process::Command;
//use std::str::Split;
//use std::thread;

#[derive(Debug)]
pub struct Pdf<'a> {
  pdf_file: String,
  texts: Vec<&'a str>,
}

impl<'a> Pdf<'a>  {
  pub fn new(pdf_file: String) -> Pdf<'a> {
    Pdf {
      pdf_file,
      texts: vec![],
    }
  }

  pub fn download(&self) {}

  pub fn generate_images(&self, filename: &str) {
//  thread::spawn(|| {
    fs::create_dir_all("tmp").expect("Failed to create dir");

    Command::new("sh")
      .arg("-c")
      .arg(format!("convert {} tmp/{}.png", self.pdf_file, filename))
      .spawn()
      .expect("Cannot convert PDF to PNG");
//  });
  }

  pub fn extract_texts(&self) {
    let command = format!("pdftotext -layout {} -", self.pdf_file);

//  let generate_texts = thread::spawn(move || {
    let texts = Command::new("sh")
      .arg("-c")
      .arg(&command)
      .output()
      .expect(&format!("Cannot extract texts, executed command was \"{}\"", command));

    let dummy_texts = String::from_utf8_lossy(&texts.stdout);
    self.texts = dummy_texts.trim().split('\x0c').collect();
//});
  }

  pub fn send_result(&self) {}
}