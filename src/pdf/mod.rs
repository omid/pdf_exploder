extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;
extern crate rand;
extern crate hyper_multipart_rfc7578 as hyper_multipart;

use std::fs::{self, create_dir_all, OpenOptions};
use std::process::Command;
use std::thread;

use std::io::Write;
use self::rand::{thread_rng, Rng};
use self::futures::{Future, Stream};
use self::hyper::{Client, Request, Method};
use self::hyper_tls::HttpsConnector;
use self::tokio_core::reactor::Core;
use self::hyper_multipart::client::multipart;

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

  pub fn download(&mut self, ) {
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

  pub fn generate_images(&self, transparent: bool) -> thread::JoinHandle<()> {
    let filename = format!("tmp/{}/pdf", self.pdf_tmp_file);

    let mut alpha = "remove";

    if transparent {
      alpha = "on";
    }

    let command = format!("convert {} -alpha {} tmp/{}/slide.png", filename, alpha, self.pdf_tmp_file);

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

  pub fn send_slides(&self, callback: String) {
    let mut core = Core::new().expect("Cannot create Core instance");
    let client: Client<_, multipart::Body> = Client::configure()
      .connector(HttpsConnector::new(4, &core.handle()).unwrap())
      .body::<multipart::Body>()
      .build(&core.handle());

    for i in 0..self.texts.len() {
      let uri = callback.parse().expect("Cannot parse the URL");

      let mut req = Request::new(Method::Post, uri);

      let mut form = multipart::Form::default();

      form.add_text("current", (i + 1).to_string());
      form.add_text("total", self.texts.len().to_string());
      form.add_text("slideText", self.texts[i].clone());
      form.add_file("file", format!("tmp/{}/slide-{}.png", self.pdf_tmp_file, i)).expect("Cannot find file");
      form.set_body(&mut req);

      // @TODO make ask async
      core.run(client.request(req)).expect("Oops");
      println!("After upload slide");
    }

    println!("After upload");
  }

  pub fn send_ack(&self, callback: String) {
    let mut core = Core::new().expect("Cannot create Core instance");
    let client: Client<_, multipart::Body> = Client::configure()
      .connector(HttpsConnector::new(4, &core.handle()).unwrap())
      .body::<multipart::Body>()
      .build(&core.handle());

    let uri = callback.parse().expect("Cannot parse the URL");

    let mut req = Request::new(Method::Get, uri);

    // @TODO build request
    // {
    //   'success': state,
    //   'message': message
    // }
    // 'headers': {
    //   'Content-Type': 'application/json'
    // }

    core.run(client.request(req)).expect("Oops");

    println!("After ack");
  }

  pub fn cleanup(&self) {
    let filename = format!("tmp/{}", self.pdf_tmp_file);
    fs::remove_dir_all(filename).expect("Couldn't cleanup");
    println!("After delete");
  }
}
