use regex::Regex;
use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    panic,
    path::PathBuf,
    str::FromStr,
};

use rayon::iter::{ParallelBridge, ParallelIterator};
#[derive(Debug)]
struct Config {
    query: String,
    is_dir: bool,
    path: PathBuf,
    search_for_regex: bool,
}

enum UserError {
    ArgErr,
    UnknownFlag,
}

impl Config {
    fn new(args: Vec<String>) -> Config {
        match Config::parse(args) {
            Ok(c) => c,
            Err(e) => match e {
                UserError::UnknownFlag => panic!("unknown flag"),
                UserError::ArgErr => panic!("wrong amount of arguemnts"),
            },
        }
    }
    // TODO
    //
    // can not search for anything that is two charachter long and starts with a - as it gets
    // treated like a flag;
    fn parse(args: Vec<String>) -> Result<Config, UserError> {
        let mut cfg: Config = Config {
            query: String::new(),
            is_dir: false,
            path: PathBuf::new(),
            search_for_regex: false,
        };

        for a in args {
            if a.starts_with("-") && a.len() == 2 {
                let flag = a.to_owned().pop().unwrap();
                match flag {
                    'e' => cfg.search_for_regex = true,
                    'r' => cfg.is_dir = true,
                    _ => {
                        return Err(UserError::UnknownFlag);
                    }
                }
            } else {
                if cfg.query.is_empty() {
                    cfg.query = a;
                } else {
                    cfg.path = PathBuf::from(a);
                    break;
                }
            }
        }
        Ok(cfg)
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let config: Config = Config::new(args);
    if config.path.to_str() == Some("") {
        find_and_print_occurances_from_pipe(&config.query, config.search_for_regex);
    } else if config.is_dir {
        search_directory(config.path, &config.query, config.search_for_regex);
    } else {
        print_occurances(config.path, &config.query, config.search_for_regex);
    }
}

// TODO
//
// instead of printing errors to stdio they should be ptinted to stderr;
fn get_file_as_buffer(path: &PathBuf) -> Option<BufReader<File>> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            println!(
                "Could not open {:?} because an error occured: \n {:?}",
                &path, e
            );
            return None;
        }
    };
    Some(BufReader::new(file))
}

// TODO
//
// large inputs with long lines will cause a OOM error (need to replaxe stdin.lock().lines() like i did in print_occurances)
fn find_and_print_occurances_from_pipe(query: &str, search_regex: bool) {
    let stdin = io::stdin();
    let inp = stdin.lock().lines();
    let reg_pattern = match search_regex {
        true => Some(Regex::from_str(query).unwrap()),
        _ => None,
    };

    for el in inp {
        let line = match el {
            Ok(s) => s,
            Err(_) => continue,
        };
        if !search_regex {
            if line.contains(query) {
                println!("{}", line.trim());
            }
        } else {
            match reg_pattern.as_ref().unwrap().find(&line) {
                Some(_) => {
                    println!("{}", line.trim());
                }
                None => {
                    continue;
                }
            };
        }
    }
}

// TODO
//
// current way of buffered line reading messes with formatting when printing occurances to Terminal;
// files smaller than the 1000 byte limit never get checked at all;
//
// if the remainder of a file is smaller than 1000 bytes it never gets searched;
//
// if an occurance is split by the 1000 byte limit of the buffer it gets lost and will never be found;
//
// Line numbering does not work with 1000 byte buffer because currently it counts the
// amount of times a 1000 byte buffer is read and not the actual line;
//
// because stdout.lock() blocks the stdout every other thread has to wait for the stdout to be free
// again so multithreading does not have any effect because it cant proccess files in parallel;
fn print_occurances(file_path: PathBuf, query: &str, search_regex: bool) {
    let mut file = match get_file_as_buffer(&file_path) {
        Some(f) => f,
        None => {
            return;
        }
    };
    let stdout = io::stdout();
    let mut buf_writer = BufWriter::with_capacity(128 * 1024, stdout.lock());
    let mut str_buf: [u8; 1000] = [0; 1000];
    let reg_pattern = match search_regex {
        true => Some(Regex::from_str(query).unwrap()),
        _ => None,
    };
    let mut line: u32 = 0;

    while file.read_exact(&mut str_buf).is_ok() {
        let text = String::from_utf8_lossy(&str_buf);
        if !search_regex {
            if text.contains(query) {
                let _ = write!(
                    buf_writer,
                    "[{}:{}]\t{}",
                    file_path.display(),
                    line,
                    text.trim()
                );
            }
        } else {
            let reg = reg_pattern.as_ref().unwrap().find(&text);
            match reg {
                Some(_) => {
                    let _ = write!(
                        buf_writer,
                        "[{}:{}]\t{}",
                        file_path.display(),
                        line,
                        text.trim()
                    );
                }
                None => {
                    line += 1;
                    continue;
                }
            };
        }
        line += 1;
    }
    let _ = buf_writer.flush();
}

fn search_directory(path: PathBuf, query: &str, regex: bool) {
    let entries = match fs::read_dir(&path) {
        Ok(e) => e,
        Err(_) => return,
    };

    entries
        .filter_map(Result::ok)
        .par_bridge()
        .for_each(|entry| {
            let p = entry.path();
            let t = entry.file_type().unwrap();
            if t.is_symlink() {
                return;
            }
            if t.is_dir() {
                search_directory(p, query, regex);
            } else if t.is_file() {
                print_occurances(p, query, regex);
            }
        });
}
