#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use]
extern crate rocket;

extern crate rocket_download_response;

use std::path::Path;

use rocket::*;

use rocket_download_response::DownloadResponse;

#[get("/")]
async fn download() -> DownloadResponse {
    let path = Path::join(Path::new("examples"), Path::join(Path::new("images"), "image(貓).jpg"));

    DownloadResponse::from_file(path, None::<String>, None).await
}

#[launch]
fn launch() -> _ {
    rocket::ignite().mount("/", routes![download]).launch()
}
