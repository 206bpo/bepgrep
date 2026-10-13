use regex::Regex;
use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write, stdout},
    panic,
    path::PathBuf,
    str::FromStr,
};

use rayon::iter::{ParallelBridge, ParallelIterator};
#[derive(Debug)]
struct Config {
    query: String,
    is_dir: bool,
    search_multithreaded: bool,
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
            search_multithreaded: false,
            path: PathBuf::new(),
            search_for_regex: false,
        };

        for a in args {
            if a.starts_with("-") && a.len() == 2 {
                let flag = a.to_owned().pop().unwrap();
                match flag {
                    'e' => cfg.search_for_regex = true,
                    'd' => cfg.is_dir = true,
                    'r' => cfg.search_multithreaded = true,
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
        if config.search_multithreaded {
            search_directory_multithreaded(config.path, &config.query, config.search_for_regex);
        } else {
            search_directory(config.path, &config.query, config.search_for_regex);
        }
    } else {
        find_and_print_occurances_from_filepath(
            config.path,
            &config.query,
            config.search_for_regex,
        );
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
//line numbering is messed up, because some lines are read multiple times.
const FLUSHAT: usize = 16 * 1024;
const CHUNKSIZE: usize = 8 * 1024;
fn find_and_print_occurances_from_filepath(file_path: PathBuf, query: &str, search_regex: bool) {
    let mut file = match get_file_as_buffer(&file_path) {
        Some(f) => f,
        None => {
            return;
        }
    };
    let mut buf_writer: Vec<u8> = Vec::with_capacity(FLUSHAT);
    let mut file_chunk: Vec<u8> = vec![0; CHUNKSIZE];
    let mut old_file_chunk = file_chunk.clone();
    let reg_pattern = match search_regex {
        true => Some(Regex::from_str(query).unwrap()),
        _ => None,
    };
    let mut line_number: u32 = 1;

    while file.read(&mut file_chunk).is_ok() {
        if file_chunk.len() == 0 {
            return;
        }
        if file_chunk[0] == 0 {
            return;
        }
        old_file_chunk.append(&mut file_chunk);
        let text = match String::from_utf8(old_file_chunk.clone()) {
            Ok(s) => s,
            Err(_) => {
                continue;
            }
        };
        let lines = text.split_terminator("\n");
        for line in lines {
            let hit = match &search_regex {
                true => reg_pattern.as_ref().unwrap().is_match(line),
                false => line.contains(query),
            };
            if hit {
                let _ = writeln!(
                    buf_writer,
                    "[{}:{}]: {}",
                    file_path.display(),
                    line_number,
                    line
                );
                if buf_writer.len() >= FLUSHAT {
                    flush_out(&mut buf_writer);
                }
            }
            line_number += 1;
        }

        old_file_chunk = file_chunk.clone();
        file_chunk = [0_u8; CHUNKSIZE].to_vec();
    }
    flush_out(&mut buf_writer);
}

fn search_directory_multithreaded(path: PathBuf, query: &str, regex: bool) {
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
                search_directory_multithreaded(p, query, regex);
            } else if t.is_file() {
                find_and_print_occurances_from_filepath(p, query, regex);
            }
        });
}

fn search_directory(path: PathBuf, query: &str, regex: bool) {
    let entries = match fs::read_dir(&path) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.filter_map(Result::ok) {
        let p = entry.path();
        let t = entry.file_type().unwrap();
        if t.is_symlink() {
            return;
        }
        if t.is_dir() {
            search_directory(p, &query, regex);
        } else if t.is_file() {
            find_and_print_occurances_from_filepath(p, query, regex);
        }
    }
}

fn flush_out(out: &mut Vec<u8>) {
    let stdout = io::stdout();
    let mut lock = stdout.lock();
    let _ = lock.write_all(out);
    out.clear();
}
