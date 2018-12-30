extern crate uuid;
extern crate reqwest;
extern crate threadpool;
#[macro_use] extern crate serde_derive;

use std::fs::{create_dir_all, remove_dir_all, File};
use std::process::Command;
use std::collections::HashMap;
use std::io::Write;
use std::sync::mpsc::channel;

use threadpool::ThreadPool;
use uuid::Uuid;
use reqwest::{Client, multipart};

#[derive(Deserialize, Clone)]
pub struct DownloadData {
    #[serde(rename = "type")]
    pub _type: String,
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct Callback {
    pub url: String,
}

#[derive(Deserialize, Clone)]
pub struct UploadData {
    pub url: String,
    pub callback: Callback,
}

#[derive(Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct ConversionParams {
    pub preserveTransparency: Option<bool>,
}

#[derive(Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct RequestBody {
    pub downloadData: DownloadData,
    pub uploadData: UploadData,
    pub conversionParams: Option<ConversionParams>,
}

#[derive(Debug, Clone)]
pub struct Presentation {
    pub presentation_file: String,
    pub presentation_tmp_file: String,
    pub texts: Vec<String>,
    pub number_of_pages: usize,
}

impl Presentation {
    pub fn new(presentation_file: String) -> Presentation {
        Presentation {
            presentation_file,
            presentation_tmp_file: String::new(),
            texts: vec![],
            number_of_pages: 0,
        }
    }

    pub fn extract(&mut self, data: RequestBody) -> String {
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

    pub fn download(&mut self) {
        self.presentation_tmp_file = Uuid::new_v4().to_string();

        create_dir_all(format!("tmp/{}", self.presentation_tmp_file)).expect("Failed to create dir");

        let client = Client::new();
        let response = client.get(&self.presentation_file).send();

        let mut buffer = File::create(format!("tmp/{}/presentation", self.presentation_tmp_file)).unwrap();
        let text = response.unwrap().text().unwrap();
        buffer.write(text.as_bytes()).expect("Cannot download the file");

        self.extract_number_of_pages();

        // pre-fill texts vector
        self.texts = vec!["".to_string(); self.number_of_pages];

        println!("After download");
    }

    fn extract_number_of_pages(&mut self) {
        let filename = format!("tmp/{}/presentation", self.presentation_tmp_file);

        let command = format!(
            "pdfinfo {} | grep --binary-files=text Pages | cut -f 2 -d \":\"",
            filename,
        );

        println!("{}", command);

        let out = Command::new("sh").arg("-c").arg(&command).output().expect(
            "Could not extract presentation pages of presentation",
        );

        println!("After number of page command");

        let output = String::from_utf8_lossy(&out.stdout);

        self.number_of_pages = output.trim().parse::<usize>().unwrap();
    }

    pub fn extract_pages(&self) {
        let filename = format!("tmp/{}/presentation", self.presentation_tmp_file);

        let command = format!(
            "pdfseparate {} tmp/{}/%d.pdf",
            filename,
            self.presentation_tmp_file,
        );

        Command::new("sh").arg("-c").arg(&command).output().expect(
            "Could not extract presentation pages of presentation",
        );

        println!("After convert");
    }

    pub fn generate_images(&self, transparent: bool) -> ThreadPool {
        let mut alpha = "";

        if transparent {
            alpha = "-transp";
        }

        let pool = ThreadPool::new(10);

        for i in 1..(self.number_of_pages+1) {
            let filename = format!("tmp/{}/{}", self.presentation_tmp_file, i);

            let command = format!(
                "pdftocairo -singlefile -png -r 150 {}.pdf {} {}",
                filename,
                alpha,
                filename,
            );

            pool.execute(move|| {
                Command::new("sh").arg("-c").arg(&command).output().expect(
                    "Cannot convert PDF to PNG",
                );

                println!("After convert {}", i);
            });
        };

        pool
    }

    pub fn extract_texts(&mut self) -> ThreadPool {
        let pool = ThreadPool::new(10);
        let (tx, rx) = channel();

        for i in 1..(self.number_of_pages+1) {
            let filename = format!("tmp/{}/{}.pdf", self.presentation_tmp_file, i);
            let command = format!("pdftotext -layout {} -", filename);

            println!("{}", command);

            let tx = tx.clone();
            pool.execute(move|| {
                let out = Command::new("sh").arg("-c").arg(&command).output().expect(
                    &format!("Cannot extract texts, executed command was \"{}\"", command),
                );

                let text = String::from_utf8_lossy(&out.stdout);
                println!("After extract");

                tx.send((i, text.trim().to_string())).expect("channel will be waiting");

                println!("After text extract {}", i);
            });
        };

        rx.iter().take(self.number_of_pages).for_each(|(i, text)| {
            self.texts.insert(i-1, text);
        });

        pool
    }

    pub fn send_slides(&self, callback: String) -> ThreadPool {
        let pool = ThreadPool::new(10);

        println!("callback: {}", callback);
        for i in 0..(self.number_of_pages) {
            let form = multipart::Form::new()
                .text("current", (i + 1).to_string())
                .text("total", self.number_of_pages.to_string())
                .text("slideText", self.texts[i].clone())
                .file("file", format!("tmp/{}/{}.png", self.presentation_tmp_file, i+1))
                .expect("Cannot find the PNG file.");

            let dummy_cl = callback.clone();
            let client = Client::new();

            pool.execute(move|| {
                client.post(&dummy_cl)
                    .multipart(form)
                    .send()
                    .unwrap();
            });

            println!("After upload slide");
        }

        println!("After upload");

        pool
    }

    pub fn send_ack(&self, state: &str, message: &str, callback: String) {
        let client = Client::new();

        let mut body = HashMap::new();
        body.insert("success", state);
        body.insert("message", message);

        client.get(&callback)
            .json(&body)
            .send()
            .unwrap();

        println!("After ack");
    }

    pub fn cleanup(&self) {
        let filename = format!("tmp/{}", self.presentation_tmp_file);
        remove_dir_all(filename).expect("Couldn't cleanup");
        println!("After delete");
    }
}
