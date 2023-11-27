use eyre::{Report, Result};
use once_cell::sync::Lazy;
use reqwest::blocking::{ClientBuilder, Response};
use reqwest::IntoUrl;
use std::fs::File;
use std::path::Path;

use crate::Context;

#[derive(Debug)]
pub struct Client {
    reqwest: reqwest::blocking::Client,
}

pub static HTTP: Lazy<Client> = Lazy::new(|| Client::new(&Context::new()).unwrap());

impl Client {
    pub fn new(ctx: &Context) -> Result<Self> {
        let reqwest = ClientBuilder::new()
            .user_agent(&ctx.user_agent)
            .gzip(true)
            .build()?;
        Ok(Self { reqwest })
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> Result<Response> {
        let url = url.into_url().unwrap();
        debug!("GET {}", url);
        let resp = self.reqwest.get(url).send()?;
        debug!("{} {}", resp.status(), resp.url());
        resp.error_for_status_ref()?;
        Ok(resp)
    }

    pub fn download_file<U: IntoUrl>(&self, url: U, path: &Path) -> Result<()> {
        let url = url.into_url()?;
        debug!("Downloading {} to {}", &url, path.display());
        let mut resp = self.get(url)?;
        let mut file = File::create(path)?;
        resp.copy_to(&mut file)?;
        Ok(())
    }
}

pub fn error_code(e: &Report) -> Option<u16> {
    if let Some(err) = e.downcast_ref::<reqwest::Error>() {
        err.status().map(|s| s.as_u16())
    } else {
        None
    }
}
