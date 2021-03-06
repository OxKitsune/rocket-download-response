/*!
# Download Response for Rocket Framework

This crate provides a response struct used for client downloading.

See `examples`.
*/

pub extern crate mime;
extern crate mime_guess;
extern crate percent_encoding;
extern crate rocket;
#[macro_use]
extern crate educe;

use std::fs::File;
use std::io::{Cursor, ErrorKind, Read};
use std::path::Path;
use std::rc::Rc;

use mime::Mime;
use percent_encoding::{AsciiSet, CONTROLS};

use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};

const FRAGMENT_PERCENT_ENCODE_SET: &AsciiSet =
    &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

const PATH_PERCENT_ENCODE_SET: &AsciiSet =
    &FRAGMENT_PERCENT_ENCODE_SET.add(b'#').add(b'?').add(b'{').add(b'}');

#[derive(Educe)]
#[educe(Debug)]
enum DownloadResponseData {
    Static(&'static [u8]),
    Vec(Vec<u8>),
    // I cannot easily change this to support the new rocket
    // Reader {
    //     #[educe(Debug(ignore))]
    //     data: Box<dyn Read + 'static>,
    //     content_length: Option<u64>,
    // },
    File(Rc<Path>),
}

#[derive(Debug)]
pub struct DownloadResponse {
    file_name: Option<String>,
    content_type: Option<Mime>,
    data: DownloadResponseData,
}

impl DownloadResponse {
    /// Create a `DownloadResponse` instance from a `&'static [u8]`.
    pub fn from_static<S: Into<String>>(
        data: &'static [u8],
        file_name: Option<S>,
        content_type: Option<Mime>,
    ) -> DownloadResponse {
        let file_name = file_name.map(|file_name| file_name.into());

        let data = DownloadResponseData::Static(data);

        DownloadResponse {
            file_name,
            content_type,
            data,
        }
    }

    /// Create a `DownloadResponse` instance from a `Vec<u8>`.
    pub fn from_vec<S: Into<String>>(
        vec: Vec<u8>,
        file_name: Option<S>,
        content_type: Option<Mime>,
    ) -> DownloadResponse {
        let file_name = file_name.map(|file_name| file_name.into());

        let data = DownloadResponseData::Vec(vec);

        DownloadResponse {
            file_name,
            content_type,
            data,
        }
    }

    // /// Create a `DownloadResponse` instance from a reader.
    // pub fn from_reader<R: Read + 'static, S: Into<String>>(
    //     reader: R,
    //     file_name: Option<S>,
    //     content_type: Option<Mime>,
    //     content_length: Option<u64>,
    // ) -> DownloadResponse {
    //     let file_name = file_name.map(|file_name| file_name.into());

    //     let data = DownloadResponseData::Reader {
    //         data: Box::new(reader),
    //         content_length,
    //     };

    //     DownloadResponse {
    //         file_name,
    //         content_type,
    //         data,
    //     }
    // }

    /// Create a `DownloadResponse` instance from a path of a file.
    pub async fn from_file<P: Into<Rc<Path>>, S: Into<String>>(
        path: P,
        file_name: Option<S>,
        content_type: Option<Mime>,
    ) -> DownloadResponse {
        let path = path.into();
        let file_name = file_name.map(|file_name| file_name.into());

        let data = DownloadResponseData::File(path);

        DownloadResponse {
            file_name,
            content_type,
            data,
        }
    }
}

macro_rules! file_name {
    ($s:expr, $res:expr) => {
        if let Some(file_name) = $s.file_name {
            if file_name.is_empty() {
                $res.raw_header("Content-Disposition", "attachment");
            } else {
                $res.raw_header(
                    "Content-Disposition",
                    format!(
                        "attachment; filename*=UTF-8''{}",
                        percent_encoding::percent_encode(
                            file_name.as_bytes(),
                            PATH_PERCENT_ENCODE_SET
                        )
                    ),
                );
            }
        }
    };
}

macro_rules! content_type {
    ($s:expr, $res:expr) => {
        if let Some(content_type) = $s.content_type {
            $res.raw_header("Content-Type", content_type.to_string());
        }
    };
}

impl<'r> Responder<'r, 'static> for DownloadResponse {
    fn respond_to(self, _: &Request<'_>) -> response::Result<'static> {
        let mut response = Response::build();

        match self.data {
            DownloadResponseData::Static(data) => {
                file_name!(self, response);
                content_type!(self, response);

                response.sized_body(data.len(), Cursor::new(data));
            }
            DownloadResponseData::Vec(data) => {
                file_name!(self, response);
                content_type!(self, response);

                response.sized_body(data.len(), Cursor::new(data));
            }
            // DownloadResponseData::Reader {
            //     data,
            //     content_length,
            // } => {
            //     file_name!(self, response);
            //     content_type!(self, response);

            //     if let Some(content_length) = content_length {
            //         response.raw_header("Content-Length", content_length.to_string());
            //     }

            //     response.streamed_body(data);
            // }
            DownloadResponseData::File(path) => {
                if let Some(file_name) = self.file_name {
                    if file_name.is_empty() {
                        response.raw_header("Content-Disposition", "attachment");
                    } else {
                        response.raw_header(
                            "Content-Disposition",
                            format!(
                                "attachment; filename*=UTF-8''{}",
                                percent_encoding::percent_encode(
                                    file_name.as_bytes(),
                                    PATH_PERCENT_ENCODE_SET,
                                )
                            ),
                        );
                    }
                } else if let Some(file_name) =
                    path.file_name().map(|file_name| file_name.to_string_lossy())
                {
                    response.raw_header(
                        "Content-Disposition",
                        format!(
                            "attachment; filename*=UTF-8''{}",
                            percent_encoding::percent_encode(
                                file_name.as_bytes(),
                                PATH_PERCENT_ENCODE_SET,
                            )
                        ),
                    );
                } else {
                    response.raw_header("Content-Disposition", "attachment");
                }

                if let Some(content_type) = self.content_type {
                    response.raw_header("Content-Type", content_type.to_string());
                } else if let Some(extension) = path.extension() {
                    if let Some(extension) = extension.to_str() {
                        let content_type = mime_guess::from_ext(extension).first_or_octet_stream();

                        response.raw_header("Content-Type", content_type.to_string());
                    }
                }

                let mut file = File::open(path).map_err(|err| {
                    if err.kind() == ErrorKind::NotFound {
                        Status::NotFound
                    } else {
                        Status::InternalServerError
                    }
                })?;

                let mut buffer = vec![0; file.metadata().unwrap().len() as usize];
                file.read(&mut buffer).map_err(| err| {
                    Status::InternalServerError
                })?;

                response.sized_body(buffer.len(), Cursor::new(buffer));
            }
        }

        response.ok()
    }
}
