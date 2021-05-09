use clap::{load_yaml, App, ArgMatches};
use std::cell::RefCell;
use crate::configuration_properties::InternalConfiguration;
use std::borrow::Borrow;

#[derive(Clone, Debug)]
pub struct Cli {
    args: Option<ArgMatches>,
}

impl Cli {
    pub fn new() -> Self {
        Cli { args: None }
    }
}

impl Cli {
    pub fn get_matches(mut self) -> ArgMatches {
        return if self.args.is_none() {
            let yaml = load_yaml!("cli.yml");
            let matches = App::from(yaml).get_matches();
            self.args.insert(matches.clone());
            return matches;
        } else {
            self.args.unwrap()
        };
    }
}