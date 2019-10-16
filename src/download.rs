use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;
use reqwest::header::{HeaderMap, ETAG, IF_NONE_MATCH, USER_AGENT};
use reqwest::Client;
use serde_json::{from_reader, to_writer_pretty};
use tiny_fail::{ErrorMessageExt, Fail};

const TIMEOUT_SECS: u64 = 10;
const DUMP_URL: &str = "https://www.edsm.net/dump/systemsPopulated.json";
const DUMP_FILE: &str = "systemsPopulated.json.gz";

pub fn download() -> Result<(), Fail> {
    let etags = EtagStoreage::new("./.cache.json");
    let downloader = Downloader::new(etags)?;

    downloader.download()?;

    Ok(())
}

struct Downloader {
    get_client: Client,
    etags: EtagStoreage,
}

impl Downloader {
    pub fn new(etags: EtagStoreage) -> Result<Downloader, Fail> {
        let mut default_headers = HeaderMap::new();
        default_headers.insert(
            USER_AGENT,
            format!(
                "EDSM Dumps Downloader/{}",
                option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version")
            )
            .parse()
            .unwrap(),
        );

        let get_client = Client::builder()
            .default_headers(default_headers.clone())
            .connect_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))
            .gzip(true)
            .build()?;

        Ok(Downloader { get_client, etags })
    }

    pub fn download(&self) -> Result<(), Fail> {
        let mut req = self.get_client.get(DUMP_URL);

        if let Some(etag) = self.etags.get(DUMP_URL)? {
            req = req.header(IF_NONE_MATCH, etag);
        }

        let mut res = req.send()?.error_for_status()?;

        if res.status().as_u16() == 304 {
            return Ok(());
        }

        eprintln!("Downloading update...");
        let f = File::create(DUMP_FILE)?;
        let mut w = GzEncoder::new(f, Compression::best());

        res.copy_to(&mut w)?;

        w.flush()?;

        // save ETag
        if let Some(etag) = res.headers().get(ETAG) {
            let etag = etag.to_str().err_msg("can't parse ETag as string")?;
            self.etags.save(DUMP_URL, etag)?;
        } else {
            self.etags.remove(DUMP_URL)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EtagStoreage {
    path: PathBuf,
}

impl EtagStoreage {
    pub fn new<P: AsRef<Path>>(path: P) -> EtagStoreage {
        EtagStoreage {
            path: path.as_ref().to_owned(),
        }
    }

    pub fn get(&self, url: &str) -> Result<Option<String>, Fail> {
        if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            let mut table: BTreeMap<String, String> =
                from_reader(f).err_msg("can't parse ETag file")?;

            Ok(table.remove(url))
        } else {
            Ok(None)
        }
    }

    pub fn save(&self, url: &str, etag: &str) -> Result<(), Fail> {
        let mut table: BTreeMap<String, String> = if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            from_reader(f).err_msg("can't parse ETag file")?
        } else {
            BTreeMap::new()
        };

        table.insert(url.to_owned(), etag.to_owned());

        let mut f =
            File::create(&self.path).err_msg(format!("can't create file: {:?}", self.path))?;
        to_writer_pretty(&mut f, &table).err_msg("can't encode ETag file")?;

        Ok(())
    }

    pub fn remove(&self, url: &str) -> Result<(), Fail> {
        let mut table: BTreeMap<String, String> = if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            from_reader(f).err_msg("can't parse ETag file")?
        } else {
            BTreeMap::new()
        };

        table.remove(url);

        let mut f =
            File::create(&self.path).err_msg(format!("can't create file: {:?}", self.path))?;
        to_writer_pretty(&mut f, &table).err_msg("can't encode ETag file")?;

        Ok(())
    }
}
