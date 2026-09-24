use regex::{Regex, regex};
use std::{
    env,
    fs::{self, DirEntry, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write},
    panic,
    path::PathBuf,
    str::FromStr,
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
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

fn get_file_as_buffer(path: &PathBuf) -> BufReader<File> {
    let file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            panic!("Something went wrong while reading the file: {e:?}");
        }
    };
    BufReader::new(file)
}

fn find_and_print_occurances_from_pipe(query: &str, search_regex: bool) {
    let mut inp = String::with_capacity(128 * 1024);
    let stdin = io::stdin();
    stdin.lock().read_to_string(&mut inp).unwrap();
    let input: Vec<&str> = inp.split_terminator("\n").collect();

    for el in input {
        if !search_regex {
            if el.contains(query) {
                println!("{}", el.trim());
            }
        } else {
            let reg = Regex::from_str(query).unwrap().find(&el);
            match reg {
                Some(_) => {
                    println!("{}", el.trim());
                }
                None => {
                    continue;
                }
            };
        }
    }
}

fn print_occurances(file_path: PathBuf, query: &str, search_regex: bool) {
    let mut file = get_file_as_buffer(&file_path);
    let stdout = io::stdout();
    let mut buf_writer = BufWriter::with_capacity(128 * 1024, stdout.lock());
    let mut str_buf = String::with_capacity(1000);
    let mut line: u32 = 0;
    loop {
        match file.read_line(&mut str_buf) {
            Ok(0) => {
                break;
            }
            Ok(_) => {}
            Err(_) => return,
        }
        if !search_regex {
            if str_buf.contains(query) {
                writeln!(
                    buf_writer,
                    "[{}:{}]\t{}",
                    file_path.display(),
                    line,
                    str_buf.trim()
                );
            }
        } else {
            let reg = Regex::from_str(query).unwrap().find(&str_buf);
            match reg {
                Some(_) => {
                    writeln!(
                        buf_writer,
                        "[{}:{}]\t{}",
                        file_path.display(),
                        line,
                        str_buf.trim()
                    );
                }
                None => {
                    str_buf.clear();
                    line += 1;
                    continue;
                }
            };
        }
        str_buf.clear();
        line += 1;
    }
    buf_writer.flush();
}

fn search_directory(path: PathBuf, query: &str, regex: bool) {
    let entries: Vec<DirEntry> = fs::read_dir(&path)
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let _: Vec<_> = entries
        .par_iter()
        .map(|element| {
            if element.path().is_dir() {
                search_directory(element.path(), query, regex);
            } else {
                print_occurances(element.path(), query, false);
            }
        })
        .collect();
}
