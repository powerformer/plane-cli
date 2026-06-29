#![allow(dead_code)]

#[path = "../src/app.rs"]
mod app;
#[path = "../src/cli.rs"]
mod cli;
#[path = "../src/config/mod.rs"]
mod config;
#[path = "../src/output.rs"]
mod output;
#[path = "../src/skill.rs"]
mod skill;

#[path = "unit/cli.rs"]
mod cli_cases;
#[path = "unit/config.rs"]
mod config_cases;
