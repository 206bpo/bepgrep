use std::{
    env,
    fs::{self, File},
    io::{self, BufRead, BufReader, BufWriter, Write},
    panic,
    path::PathBuf,
    sync::Arc,
    thread,
};

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
        if arg_len == 3 && &args[0] == "dir" {
            let path = PathBuf::from(&args[2]);
            Ok(Config {
                query: args[1].clone(),
                is_dir: true,
                path,
            })
        } else if arg_len == 2 {
            let path = PathBuf::from(&args[1]);
            Ok(Config {
                query: args[0].clone(),
                is_dir: false,
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
    if config.is_dir {
        let res = search_directory(config.path, Arc::from(config.query));
        for entry in res {
            print!("{}", entry);
        }
    } else {
        let mut file = get_file_as_buffer(&config.path);
        print_occurances(&mut file, &config.query);
    }
}

fn print_occurances(file: &mut BufReader<File>, query: &str) {
    let stdout = io::stdout();
    let mut buf_writer = BufWriter::with_capacity(128 * 1024, stdout.lock());
    let mut str_buf = String::with_capacity(1000);
    loop {
        match file.read_line(&mut str_buf) {
            Ok(0) => {
                break;
            }
            Ok(_) => {}
            Err(e) => panic!("Fuck: {}", e),
        }
        if str_buf.contains(query) {
            write!(buf_writer, "{}", str_buf);
        }
        str_buf.clear();
    }
}

fn get_occurances_in_file(path: &PathBuf, query: &Arc<String>) -> Option<Vec<String>> {
    let mut str_buf = String::with_capacity(1000);
    let mut res: Vec<String> = Vec::new();
    let mut file = get_file_as_buffer(path);
    loop {
        match file.read_line(&mut str_buf) {
            Ok(0) => {
                break;
            }
            Ok(_) => {}
            Err(_) => {
                return None;
            }
        }
        if str_buf.contains(query.as_str()) {
            res.push(str_buf.clone());
        }
        str_buf.clear();
    }
    Some(res)
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

fn search_directory(path: PathBuf, query: Arc<String>) -> Vec<String> {
    let mut results: Vec<String> = Vec::new();
    let mut child_handles = Vec::new();

    for entry in fs::read_dir(&path).unwrap() {
        let entry = entry.unwrap();
        let entry_path = entry.path();

        if entry_path.is_dir() {
            let query = Arc::clone(&query);
            let handle = thread::spawn(move || search_directory(entry_path, query));
            child_handles.push(handle);
        } else {
            let extender = get_occurances_in_file(&entry_path, &query);
            match get_occurances_in_file(&entry_path, &query) {
                Some(x) => results.extend(x),
                None => {}
            };
        }
    }

    for handle in child_handles {
        results.extend(handle.join().unwrap());
    }
    results
}
