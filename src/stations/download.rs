use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, FixedOffset};
use flate2::write::GzEncoder;
use flate2::Compression;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::header::{HeaderMap, HeaderValue, ETAG, IF_NONE_MATCH, LAST_MODIFIED, USER_AGENT};
use reqwest::Client;
use serde_json::{from_reader, to_writer_pretty};
use tiny_fail::{ErrorMessageExt, Fail};

const TIMEOUT_SECS: u64 = 10;
const BAR_TICK_SIZE: u64 = 32 * 1024;


pub struct Downloader {
    get_client: Client,
    head_client: Client,
    etags: EtagStoreage,
}

impl Downloader {
    pub fn new() -> Result<Downloader, Fail> {
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

        let head_client = Client::builder()
            .default_headers(default_headers.clone())
            .connect_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))
            .gzip(false)
            .build()?;

        Ok(Downloader {
            get_client,
            head_client,
            etags:EtagStoreage::new("./.cache.json"),
        })
    }

    pub fn download(&self, file_name: &str, url: &str) -> Result<Option<DateTime<FixedOffset>>, Fail> {
        // check update and get size
        let spin_style = ProgressStyle::default_spinner().template("{spinner} {msg}");

        let bar = ProgressBar::new_spinner();
        bar.set_style(spin_style.clone());
        bar.enable_steady_tick(100);
        bar.set_message("Checking update");

        let mut req = self.head_client.get(url);

        if let Some(etag) = self.etags.get(url)? {
            req = req.header(IF_NONE_MATCH, etag);
        }

        let res = req.send()?.error_for_status()?;

        let last_mod = res
            .headers()
            .get(LAST_MODIFIED)
            .map(HeaderValue::to_str)
            .transpose()?
            .map(DateTime::parse_from_rfc2822)
            .transpose()?;

        if res.status().as_u16() == 304 {
            bar.finish_and_clear();
            return Ok(last_mod);
        }

        let size = res.content_length();
        bar.finish_and_clear();

        // download
        let bar = if let Some(size) = size {
            let bar = ProgressBar::new(size);
            bar.set_style(ProgressStyle::default_bar().template("{msg} [{bar:40.white/black}] {bytes}/{total_bytes}, {bytes_per_sec}, {eta_precise}"));
            bar
        } else {
            let bar = ProgressBar::new_spinner();
            bar.set_style(spin_style);
            bar
        };
        bar.set_draw_delta(BAR_TICK_SIZE);
        bar.set_message("Coneccting");

        let req = self.get_client.get(url);

        let mut res = req.send()?.error_for_status()?;

        bar.set_message("Downloading");
        let f = File::create(file_name)?;
        let mut w = ProgressWriter::new(GzEncoder::new(f, Compression::best()), bar);

        res.copy_to(&mut w)?;
        let bar = w.finalize()?;

        // save ETag
        bar.set_message("Saving cache info");
        if let Some(etag) = res.headers().get(ETAG) {
            let etag = etag.to_str().err_msg("can't parse ETag as string")?;
            self.etags.save(url, etag)?;
        } else {
            self.etags.remove(url)?;
        }

        bar.finish_with_message("Downloaded");
        Ok(last_mod)
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

struct ProgressWriter<W: Write> {
    inner: W,
    prog: ProgressBar,
}

impl<W: Write> ProgressWriter<W> {
    fn new(inner: W, prog: ProgressBar) -> ProgressWriter<W> {
        ProgressWriter { inner, prog }
    }

    fn finalize(mut self) -> Result<ProgressBar, io::Error> {
        self.inner.flush()?;
        self.prog.tick();
        Ok(self.prog)
    }
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.prog.inc(n as u64);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
