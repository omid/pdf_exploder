extern crate uuid;
extern crate reqwest;
extern crate threadpool;
#[macro_use] extern crate log;
#[macro_use] extern crate serde_derive;

use std::fs::{rename, create_dir_all, remove_dir_all, File};
use std::process::Command;
use std::collections::HashMap;
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
    pub fn extract(data: RequestBody) -> String {
        info!("Extraction Started");

        let mut presentation = Presentation {
            presentation_file: data.downloadData.url.clone(),
            presentation_tmp_file: String::new(),
            texts: vec![],
            number_of_pages: 0,
        };

        // Download Presentation file
        presentation.download();
        presentation.convert_to_pdf(data.downloadData._type);
        presentation.extract_number_of_pages();
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

        if generate_image_thread.panic_count() > 0 || extract_texts_thread.panic_count() > 0 {
            return presentation.error_happened(data.uploadData.callback.url.clone());
        }

        // Send requests
        let upload_slide_requests = presentation.send_slides(data.uploadData.url.clone());
        upload_slide_requests.join();

        if upload_slide_requests.panic_count() > 0 {
            return presentation.error_happened(data.uploadData.callback.url.clone());
        }

        presentation.send_ack("success", "message", data.uploadData.callback.url.clone());

        // Cleanup
        presentation.cleanup();

        info!("Extraction Finished.");

        format!("Successfully extracted {} slides", presentation.number_of_pages)
    }

    fn error_happened(&self, callback_url: String) -> String {
        error!("Error happened in the extraction");

        self.send_ack("error", "message", callback_url);

        // Cleanup
        self.cleanup();

        format!("Error happened in the extraction")
    }

    fn download(&mut self) {
        info!("> Download Started");

        self.presentation_tmp_file = Uuid::new_v4().to_string();

        create_dir_all(format!("tmp/{}", self.presentation_tmp_file)).expect("Failed to create dir");

        let client = Client::new();
        let mut response = client.get(&self.presentation_file).send().unwrap();

        info!(">> URL: {}", self.presentation_file);

        let mut buffer = File::create(format!("tmp/{}/presentation", self.presentation_tmp_file)).unwrap();
        response.copy_to(&mut buffer).unwrap();

        info!("> Download Finished.");
    }

    fn convert_to_pdf(&self, _type: String) {
        info!("> Convert to PDF Started");

        let supported_formats = ["ppt", "pptx", "odp"];

        let filename = format!("tmp/{}/presentation", self.presentation_tmp_file);
        let pdf_filename = format!("tmp/{}/pdf.pdf", self.presentation_tmp_file);

        if supported_formats.contains(&_type.as_str()) {
            let command = format!(
                "unoconv -f pdf -o \"{}\" \"{}\"",
                pdf_filename,
                filename,
            );

            info!(">> Conversion command: {}", command);

            let out = Command::new("sh").arg("-c").arg(&command).output().expect(
                "Could not extract presentation pages of presentation",
            );

            let output = String::from_utf8_lossy(&out.stdout);

            info!(">> Output: {}", output);

        } else if _type == "pdf" {
            rename(&filename, &pdf_filename).unwrap();

            info!(">>> Rename PDF file from {} to {}", filename, pdf_filename);
        }

        info!("> Convert to PDF Finished.");
    }

    fn extract_number_of_pages(&mut self) {
        info!("> Extract Number of Pages Started");

        let filename = format!("tmp/{}/pdf.pdf", self.presentation_tmp_file);

        let command = format!(
            "pdfinfo {} | grep --binary-files=text Pages | cut -f 2 -d \":\"",
            filename,
        );

        info!(">> Extract Number of Pages command: {}", command);

        let out = Command::new("sh").arg("-c").arg(&command).output().expect(
            "Could not extract presentation pages of presentation",
        );

        let output = String::from_utf8_lossy(&out.stdout);

        info!(">> Output: {}", output);

        self.number_of_pages = output.trim().parse::<usize>().unwrap();

        info!(">> Number of pages: {}", self.number_of_pages);

        // pre-fill texts vector
        self.texts = vec!["".to_string(); self.number_of_pages];

        info!("> Extract Number of Pages Finished");
    }

    fn extract_pages(&self) {
        info!("> Extract Pages Started");

        let filename = format!("tmp/{}/pdf.pdf", self.presentation_tmp_file);

        let command = format!(
            "pdfseparate {} tmp/{}/%d.pdf",
            filename,
            self.presentation_tmp_file,
        );

        info!(">> Extract Pages command: {}", command);

        let out = Command::new("sh").arg("-c").arg(&command).output().expect(
            "Could not extract presentation pages of presentation",
        );

        let output = String::from_utf8_lossy(&out.stdout);

        info!(">> Output: {}", output);

        info!("> Extract Pages Finished.");
    }

    fn generate_images(&self, transparent: bool) -> ThreadPool {
        info!("> Generate Images Started");

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

            info!(">> Generate Images: command {} for slide {}", command, i);

            pool.execute(move|| {
                Command::new("sh").arg("-c").arg(&command).output().expect(
                    "Cannot convert PDF to PNG",
                );

                info!(">> Generate Images: slide {}", i);
            });
        };

        info!("> Generate Images continued in the background...");

        pool
    }

    fn extract_texts(& mut self) -> ThreadPool {
        info!("> Extract Texts Started");

        let pool = ThreadPool::new(10);
        let (tx, rx) = channel();

        for i in 1..(self.number_of_pages+1) {
            let filename = format!("tmp/{}/{}.pdf", self.presentation_tmp_file, i);
            let command = format!("pdftotext -layout {} -", filename);

            info!(">> Generate Texts: command {} for slide {}", command, i);

            let tx = tx.clone();
            pool.execute(move|| {
                let out = Command::new("sh").arg("-c").arg(&command).output().expect(
                    &format!("Cannot extract texts, executed command was \"{}\"", command),
                );

                let text = String::from_utf8_lossy(&out.stdout);

                tx.send((i, text.trim().to_string())).expect("channel will be waiting");

                info!(">> Generate Texts: slide {}", i);
            });
        };

        rx.iter().take(self.number_of_pages).for_each(|(i, text)| {
            self.texts.insert(i-1, text);
        });

        info!("> Generate Texts continued in the background...");

        pool
    }

    fn send_slides(&self, callback: String) -> ThreadPool {
        info!("> Send Slides Started");

        let pool = ThreadPool::new(10);

        for i in 0..(self.number_of_pages) {
            println!("callback: {} for tmp/{}/{}.png", callback, self.presentation_tmp_file, i+1);

            let form = multipart::Form::new()
                .text("current", (i + 1).to_string())
                .text("total", self.number_of_pages.to_string())
                .text("slideText", self.texts[i].clone())
                .file("file", format!("tmp/{}/{}.png", self.presentation_tmp_file, i+1))
                .expect("Cannot find the PNG file.");

            let dummy_cl = callback.clone();
            let client = Client::new();

            pool.execute(move|| {
                let out = client.post(&dummy_cl)
                    .multipart(form)
                    .send()
                    .unwrap()
                    .text()
                    .unwrap();

                info!(">> Callback response for {} at {} is {}", i+1, dummy_cl, &out);
            });
        }

        info!("> Send Slides continued in the background...");

        pool
    }

    fn send_ack(&self, state: &str, message: &str, callback: String) {
        info!("> Send ACK Started");

        let client = Client::new();

        let mut body = HashMap::new();
        body.insert("success", state);
        body.insert("message", message);

        let out = client.get(&callback)
            .json(&body)
            .send()
            .unwrap()
            .text()
            .unwrap();

        info!(">> Callback body: {:?}", body);
        info!(">> Callback response: {}", &out);

        info!("> Send ACK Finished.");
    }

    fn cleanup(&self) {
        info!("> Cleanup Started");

        let filename = format!("tmp/{}", self.presentation_tmp_file);
        remove_dir_all(filename).expect("Couldn't cleanup");

        info!("> Cleanup Finished.");
    }
}
