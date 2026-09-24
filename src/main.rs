use std::{
    env,
    fs::{self, DirEntry, File},
    io::{self, BufRead, BufReader, BufWriter, Read, Write, stdin},
    panic,
    path::PathBuf,
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

struct Config {
    query: String,
    is_dir: bool,
    path: PathBuf,
}

enum UserError {
    NotEnoughArguments,
    TooManyArguments,
}

impl Config {
    fn new(args: &[String]) -> Config {
        match Config::parse(args) {
            Ok(c) => c,
            Err(_) => panic!("wrong amount of arguments"),
        }
    }
    fn parse(args: &[String]) -> Result<Config, UserError> {
        let arg_len = args.len();
        if arg_len == 2 {
            let path = PathBuf::from(&args[0]);
            Ok(Config {
                query: args[1].clone(),
                is_dir: path.is_dir(),
                path,
            })
        } else if arg_len > 3 {
            Err(UserError::TooManyArguments)
        } else {
            Err(UserError::NotEnoughArguments)
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let config: Config = Config::new(&args);
    if config.path.to_str() == Some("-") {
        find_and_print_occurances(&config.query);
    } else if config.is_dir {
        search_directory(config.path, &config.query);
    } else {
        print_occurances(config.path, &config.query);
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

fn find_and_print_occurances(query: &str) {
    let mut inp = String::with_capacity(128 * 1024);
    let stdin = io::stdin();
    stdin.lock().read_to_string(&mut inp).unwrap();
    let input: Vec<&str> = inp.split_terminator("\n").collect();

    for el in input {
        if el.contains(query) {
            println!("{}", el);
        }
    }
}

fn print_occurances(file_path: PathBuf, query: &str) {
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
        if str_buf.contains(query) {
            writeln!(
                buf_writer,
                "[{}:{}]{}",
                file_path.display().to_string(),
                line,
                str_buf
            );
        }
        str_buf.clear();
        line += 1;
    }
}

fn search_directory(path: PathBuf, query: &str) {
    let entries: Vec<DirEntry> = fs::read_dir(&path)
        .unwrap()
        .filter_map(Result::ok)
        .collect();

    let _: Vec<_> = entries
        .par_iter()
        .map(|element| {
            if element.path().is_dir() {
                search_directory(element.path(), query);
            } else {
                print_occurances(element.path(), query);
            }
        })
        .collect();
}
