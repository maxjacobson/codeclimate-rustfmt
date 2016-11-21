use std::path::Path;
use std::io::prelude::*;
use std::fs::File;
extern crate rustc_serialize;

extern crate serde;
extern crate serde_json;
use serde_json::{Map, Value};

extern crate walkdir;
extern crate xml;
use rustc_serialize::json;
use walkdir::WalkDir;
use std::process::Command;

macro_rules! println_stderr(
    ($($arg:tt)*) => { {
        let r = writeln!(&mut ::std::io::stderr(), $($arg)*);
        r.expect("failed printing to stderr");
    } }
);

// TODO: move config out into its own file, probably
// TODO: switch to serde for this as well
#[derive(RustcDecodable)]
struct Config {
    enabled: bool,
    include_paths: Vec<String>,
}

impl Config {
    fn read() -> Result<Config, String> {
        let vanilla = Config { enabled: true, include_paths: vec![] };

        if !Path::new("/config.json").is_file() { return Ok(vanilla) }

        let file = File::open("/config.json");

        let mut s = String::new();

        match file {
            Ok(mut f) => {
                match f.read_to_string(&mut s) {
                    Ok(_) => {
                        println_stderr!("Config read as: {}", s);
                        match json::decode(&s) {
                            Ok(d) => Ok(d),
                            Err(_) => Err(String::from("Could not parse config file")),
                        }
                    },
                    Err(_) => {
                        Err(String::from("Could not read config file"))
                    },
                }
            },
            Err(_) => {
                Err(String::from("Could not open config file"))
            },
        }
    }

    fn files(&self) -> Vec<String> {
        let mut ret_files: Vec<String> = vec![];

        for include_path in &self.include_paths {
            let path = Path::new(include_path);
            if path.is_file() && path.ends_with(".rs") {
                // FIXME: lol
                ret_files.push(path.to_owned().to_str().unwrap().to_owned());
            } else if path.is_dir() {
                for file in &self.crawl_dir(include_path) {
                    ret_files.push(file.to_owned());
                }
            }
        }

        ret_files
    }

    fn crawl_dir(&self, path: &str) -> Vec<String> {
        let mut ret_files: Vec<String> = vec![];

        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            // FIXME: Lol this is not good:
            let filename = entry.path().to_owned().to_str().unwrap().to_owned();
            if filename.ends_with(".rs") {
                ret_files.push(filename);
            }
        }

        ret_files
    }
}

fn main() {
    let config = match Config::read() {
        Ok(c) => c,
        Err(e) => {
            println_stderr!("Could not read config because: {}", e);
            Config { enabled: true, include_paths: vec![] }
        }
    };

    if !config.enabled {
        println_stderr!("Engine not enabled, what's going on?");
        return;
    }

    for path in config.files() {
        println_stderr!("Checking: {}", path);

        // TODO: extract to some object probably...
        //       (the responsibility of getting the issues for a given filename)
        // TODO: or find a way to invoke it directly without shelling out...
        let output = Command::new("rustfmt")
            .arg("--write-mode")
            .arg("checkstyle")
            .arg(path)
            .output()
            .expect("failed to execute process");

        let stdout = String::from_utf8_lossy(output.stdout.as_slice());
        let elem: Result<xml::Element, xml::BuilderError> = stdout.parse();

        match elem {
            Ok(element) => {
                for child in element.children {
                    match child {
                        xml::Xml::ElementNode(child_element) => {
                            for error in child_element.get_children("error", None) {
                                // TODO: add severity, it seems like rustfmt supports this
                                let filename = child_element.get_attribute("name", None).unwrap().to_owned();
                                let line_number: u64 = error.get_attribute("line", None).unwrap().parse().unwrap();
                                let description = error.get_attribute("message", None).unwrap().to_owned();
                                let mut issue = Map::new();
                                issue.insert(String::from("type"), Value::String(String::from("issue")));
                                issue.insert(String::from("check_name"), Value::String(String::from("rustfmt")));
                                issue.insert(String::from("description"), Value::String(description));
                                issue.insert(String::from("categories"), Value::Array(vec![Value::String(String::from("Style"))])); // is this always the case?

                                let mut location = Map::new();
                                location.insert(String::from("path"), Value::String(filename));
                                let mut location_lines = Map::new();
                                location_lines.insert(String::from("begin"), Value::U64(line_number));
                                location_lines.insert(String::from("end"), Value::U64(line_number)); // NOTE: same line number here, not sure if that's ever not right
                                location.insert(String::from("lines"), Value::Object(location_lines));

                                issue.insert(String::from("location"), Value::Object(location));


                                let encoded_issue = serde_json::to_string(&issue).unwrap();

                                print!("{}\0", encoded_issue);
                            }
                        },
                        _ => {},
                    }
                }
            },
            Err(_) => {
                println_stderr!("No bueno!");
            },
        }
    }
}
