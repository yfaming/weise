use anyhow::Result;
use std::process::{Child, Command};

pub struct ChromeDriverProcess {
    port: u16,
    process: Child,
}

impl ChromeDriverProcess {
    pub fn server_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }
    pub fn kill(&mut self) -> Result<()> {
        self.process.kill().map_err(|e| e.into())
    }
}

impl Drop for ChromeDriverProcess {
    fn drop(&mut self) {
        self.process.kill().unwrap()
    }
}

pub fn start_chromedriver(port: u16) -> Result<ChromeDriverProcess> {
    // chromedriver --port=4444
    let p = Command::new("chromedriver")
        .arg(format!("--port={}", port))
        .spawn()?;
    Ok(ChromeDriverProcess { port, process: p })
}
