#![allow(dead_code)]

mod analysis;
mod app;
mod app_paths;
mod config;
mod curriculum;
mod gui;
mod mcm;
mod memory;
mod session;
mod skills;
mod storage;
mod teacher;
mod telemetry;
mod util;

use anyhow::Result;

fn main() -> Result<()> {
    app::run()
}
