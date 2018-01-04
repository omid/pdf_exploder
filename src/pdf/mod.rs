extern crate futures;
extern crate hyper;
extern crate tokio_core;

use std::fs;
use std::process::Command;
//use std::thread;

use std::io::{self, Write};
use futures::{Future, Stream};
use hyper::Client;
use tokio_core::reactor::Core;

#[derive(Debug)]
pub struct Pdf {
  pdf_file: String,
  texts: Vec<String>,
}

impl Pdf  {
  pub fn new(pdf_file: String) -> Pdf {
    Pdf {
      pdf_file,
      texts: vec![],
    }
  }

  pub fn download(&self) {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());

    let uri = self.pdf_file.parse()?;
    let work = client.get(uri).and_then(|res| {
      res.body().for_each(|chunk| {
        io::stdout()
          .write_all(&chunk)
          .map_err(From::from)
      })
    });
  }

  pub fn generate_images(&self) {
    // thread::spawn(|| {
      fs::create_dir_all("tmp").expect("Failed to create dir");

      let pdf_file_splitted: Vec<&str> = self.pdf_file.split("/").collect();
      let filename = pdf_file_splitted[pdf_file_splitted.len() - 1];

      Command::new("sh")
        .arg("-c")
        .arg(format!("convert {} tmp/{}.png", self.pdf_file, filename))
        .spawn()
        .expect("Cannot convert PDF to PNG");
    // });
  }

  pub fn extract_texts(&mut self) {
    let command = format!("pdftotext -layout {} -", self.pdf_file);

    // let generate_texts = thread::spawn(move || {
    let texts = Command::new("sh")
      .arg("-c")
      .arg(&command)
      .output()
      .expect(&format!("Cannot extract texts, executed command was \"{}\"", command));

    let dummy_texts = String::from_utf8_lossy(&texts.stdout);
    for i in dummy_texts.trim().split('\x0c') {
      self.texts.push(String::from(i));
    }
    // });
  }

  pub fn send_result(&self) {}
}
