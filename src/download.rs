use std::collections::BTreeMap;
use std::fmt;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use progress::{Bar, SpinningCircle};
use reqwest::header::{HeaderMap, IF_NONE_MATCH, USER_AGENT, ETAG};
use reqwest::Client;
use serde_json::{from_reader, to_writer_pretty};
use tiny_fail::{ErrorMessageExt, Fail};

const TIMEOUT_SECS: u64 = 10;
const PROGRESS_SIZE: usize = 1024 * 1024;

pub fn download() -> Result<(), Fail> {
    let etags = EtagStoreage::new("./.cache.json");
    let downloader = Downloader::new(etags)?;

    let target = Target {
        url: "https://www.edsm.net/dump/systemsPopulated.json".to_owned(),
    };
    downloader.download(&target)?;

    Ok(())
}

struct Downloader {
    head_client: Client,
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

        let head_client = Client::builder()
            .default_headers(default_headers)
            .connect_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))
            .gzip(false)
            .build()?;

        Ok(Downloader {
            get_client,
            head_client,
            etags,
        })
    }

    pub fn download(&self, target: &Target) -> Result<(), Fail> {
        // read size and update check
        let mut req = self.head_client.head(target.url());

        if let Some(etag) = self.etags.get(target)? {
            req = req.header(IF_NONE_MATCH, etag);
        }

        let res = req.send()?;

        if res.status().as_u16() == 304 {
            return Ok(());
        }

        let res = res.error_for_status()?;
        let size = res.content_length();

        // download
        let req = self.get_client.get(target.url());
        let mut res = req.send()?.error_for_status()?;

        let f = File::create(target.name()?)?;
        let mut w = ProgressWriter::new(f, size, target.name()?);

        res.copy_to(&mut w)?;

        w.flush()?;
        w.done();

        // save ETag
        if let Some(etag) = res.headers().get(ETAG) {
            let etag = etag.to_str().err_msg("can't parse ETag as string")?;
            self.etags.save(target, etag)?;
        } else {
            self.etags.remove(target)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
struct ProgressWriter<W: Write> {
    inner: BufWriter<W>,
    progress: Progress,
}

impl<W: Write> ProgressWriter<W> {
    fn new(inner: W, size: Option<u64>, name: &str) -> ProgressWriter<W> {
        ProgressWriter {
            inner: BufWriter::new(inner),
            progress: Progress::new(size, name),
        }
    }

    fn done(self) {
        self.progress.done();
    }
}

impl<W: Write> Write for ProgressWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let n = self.inner.write(buf)?;
        self.progress.add(n);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

enum Progress {
    Bar {
        bar: Bar,
        current: usize,
        total: usize,
        percent: i32,
    },
    Spin {
        spin: SpinningCircle,
        current: usize,
    },
}

impl Progress {
    fn new(size: Option<u64>, name: &str) -> Progress {
        match size {
            Some(size) => {
                let mut bar = Bar::new();
                bar.set_job_title(name);
                Progress::Bar {
                    bar,
                    current: 0,
                    total: size as usize,
                    percent: 0,
                }
            }
            None => {
                let mut spin = SpinningCircle::new();
                spin.set_job_title(name);
                Progress::Spin { spin, current: 0 }
            }
        }
    }

    fn add(&mut self, amt: usize) {
        match self {
            Progress::Bar {
                bar,
                current,
                total,
                percent,
            } => {
                *current += amt;
                let r = (*current as f64) / (*total as f64);
                let p = (100.0 * r) as i32;
                if p != *percent {
                    bar.reach_percent(p);
                    *percent = p;
                }
            }
            Progress::Spin { spin, current } => {
                *current += amt;
                let n = *current / PROGRESS_SIZE;
                *current %= PROGRESS_SIZE;
                for _ in 0..n {
                    spin.tick();
                }
            }
        }
    }

    fn done(self) {
        match self {
            Progress::Bar { mut bar, .. } => bar.jobs_done(),
            Progress::Spin { spin, .. } => spin.jobs_done(),
        }
    }
}

impl fmt::Debug for Progress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Progress::Bar { .. } => write!(f, "Progress::Bar"),
            Progress::Spin { .. } => write!(f, "Progress::Spin"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Target {
    url: String,
}

impl Target {
    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    pub fn name(&self) -> Result<&str, Fail> {
        self.url()
            .split('/')
            .last()
            .err_msg("target URL should have name part, but not")
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

    pub fn get(&self, target: &Target) -> Result<Option<String>, Fail> {
        if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            let mut table: BTreeMap<String, String> =
                from_reader(f).err_msg("can't parse ETag file")?;

            Ok(table.remove(target.url()))
        } else {
            Ok(None)
        }
    }

    pub fn save(&self, target: &Target, etag: &str) -> Result<(), Fail> {
        let mut table: BTreeMap<String, String> = if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            from_reader(f).err_msg("can't parse ETag file")?
        } else {
            BTreeMap::new()
        };

        table.insert(target.url().to_owned(), etag.to_owned());

        let mut f =
            File::create(&self.path).err_msg(format!("can't create file: {:?}", self.path))?;
        to_writer_pretty(&mut f, &table).err_msg("can't encode ETag file")?;

        Ok(())
    }

    pub fn remove(&self, target: &Target) -> Result<(), Fail> {
        let mut table: BTreeMap<String, String> = if self.path.exists() {
            let f = File::open(&self.path).err_msg(format!("can't open file: {:?}", self.path))?;
            from_reader(f).err_msg("can't parse ETag file")?
        } else {
            BTreeMap::new()
        };

        table.remove(target.url());

        let mut f =
            File::create(&self.path).err_msg(format!("can't create file: {:?}", self.path))?;
        to_writer_pretty(&mut f, &table).err_msg("can't encode ETag file")?;

        Ok(())
    }
}
