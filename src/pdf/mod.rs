extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate rand;

use std::fs::{self, create_dir_all, OpenOptions};
use std::process::Command;
use std::thread;

use std::io::Write;
use self::rand::{thread_rng, Rng};
use self::futures::{Future, Stream};
use self::hyper::Client;
use self::tokio_core::reactor::Core;

#[derive(Debug)]
pub struct Pdf {
  pub pdf_file: String,
  pub pdf_tmp_file: String,
  pub texts: Vec<String>,
}

impl Pdf {
  pub fn new(pdf_file: String) -> Pdf {
    Pdf {
      pdf_file,
      pdf_tmp_file: String::new(),
      texts: vec![],
    }
  }

  pub fn download(&mut self) {
    let uri = self.pdf_file.parse().expect("Cannot parse the URL");

    let mut core = Core::new().expect("Cannot create Core instance");
    let client = Client::new(&core.handle());

    self.pdf_tmp_file = thread_rng().gen_ascii_chars().take(20).collect();

    create_dir_all(format!("tmp/{}", self.pdf_tmp_file)).expect("Failed to create dir");

    let filename = format!("tmp/{}/pdf", self.pdf_tmp_file);

    let work = client.get(uri).and_then(|res| {
      res.body().for_each(|chunk| {
        let mut file = OpenOptions::new()
          .append(true)
          .create(true)
          .open(&filename)
          .expect("Cannot create file");

        file.write_all(&chunk).map_err(From::from)
      })
    });

    core.run(work).expect("Oops");
    println!("After download");
  }

  pub fn generate_images(&self) -> thread::JoinHandle<()> {
    let filename = format!("tmp/{}/pdf", self.pdf_tmp_file);

    let command = format!("convert {} tmp/{}/slide.png", filename, self.pdf_tmp_file);

    thread::spawn(move || {
      Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("Cannot convert PDF to PNG");

      println!("After convert");
    })
  }

  pub fn extract_texts(&mut self) -> thread::JoinHandle<Vec<String>> {
    let filename = format!("tmp/{}/pdf", self.pdf_tmp_file);
    let command = format!("pdftotext -layout {} -", filename);

    thread::spawn(move || {
      let texts = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect(&format!("Cannot extract texts, executed command was \"{}\"", command));

      let dummy_texts = String::from_utf8_lossy(&texts.stdout);
      println!("After extract");
      dummy_texts.trim().split('\x0c').map(|s| s.to_string()).collect()
    })
  }

  pub fn send_result(&self) {}

  pub fn cleanup(&self) {
    let filename = format!("tmp/{}", self.pdf_tmp_file);
    fs::remove_dir_all(filename).expect("Couldn't cleanup");
    println!("After delete");
  }
}
