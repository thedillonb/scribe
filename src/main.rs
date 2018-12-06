extern crate getopts;

use getopts::Options;
use std::env;
use std::process;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path;
use std::result;

macro_rules! fail {
    ($e:expr) => {{
        eprintln!("{}", $e.to_string());
        process::exit(1);
    }};
}

fn print_usage(program: &str, opts: Options) {
    let brief = format!("Usage: {} [options] FILE", program);
    print!("{}", opts.usage(&brief));
}

fn rotate_files(output_file_path: &path::Path, max_rotations: u32) {
    let extension = output_file_path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|x| format!(".{}", x))
        .unwrap_or_else(|| String::from(""));

    let file_base = output_file_path
        .file_stem()
        .and_then(std::ffi::OsStr::to_str)
        .expect("file does not have a stem!");

    let dir = output_file_path
        .parent()
        .expect("file should have had a parent");

    for rotation in (1..=max_rotations).rev() {
        let to_file = dir.join(format!("{}.{}{}", file_base, rotation.to_string(), extension));

        if rotation == max_rotations {
            if to_file.exists() {
                if let Err(e) = fs::remove_file(&to_file) {
                    eprintln!("Failed to remove {}: {}", to_file.display(), e.to_string());
                }
            }
        }

        let from_file = if rotation == 1 {
            output_file_path.to_path_buf()
        } else {
            dir.join(format!("{}.{}{}", file_base, (rotation - 1).to_string(), extension))
        };

        if from_file.exists() {
            if let Err(e) = fs::rename(&from_file, &to_file) {
                eprintln!("Failed to rotate {} to {}: {}", from_file.display(), to_file.display(), e.to_string());
            }
        }
    }
}

fn open_for_write(filename: &path::Path) -> result::Result<fs::File, io::Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).append(true);
    options.open(&filename)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("", "max-file-size", "The maximum size, in bytes, of the output file before it is rotated.", "BYTES");
    opts.optopt("", "max-rotations", "The maximum number of file rotations before discarding.", "NUM_FILES");
    opts.optflag("h", "help", "prints this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(f) => {
            print!("{}\n\n", f.to_string());
            print_usage(&program, opts);
            process::exit(1);
        }
    };

    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    let output_file = if !matches.free.is_empty() {
        matches.free[0].clone()
    } else {
        eprint!("Missing file argument!\n\n");
        print_usage(&program, opts);
        process::exit(1);
    };

    let max_file_size: u64 = match matches.opt_get_default("max-file-size", 1024 * 1024 * 5) {
        Ok(n) => n,
        Err(e) => fail!(e)
    };

    let max_rotations: u32 = match matches.opt_get_default("max-rotations", 5) {
        Ok(n) => n,
        Err(e) => fail!(e)
    };

    let output_file_path = path::Path::new(&output_file);

    let file_info = match output_file_path.metadata() {
        Ok(m) => open_for_write(&output_file_path).map(|f| (m.len(), f)),
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => fs::File::create(&output_file_path).map(|f| (0, f)),
        Err(e) => Err(e)
    };

    let (mut file_size, mut file) = match file_info {
        Ok(res) => res,
        Err(e) => fail!(e)
    };

    let stdin = io::stdin();
    let mut buffer: [u8; 1024 * 8] = [0; 1024 * 8];
    let mut bytes_read: usize;

    loop {
        if file_size > max_file_size {
            if let Err(e) = file.sync_all() {
                fail!(e);
            }

            if max_rotations > 0 {
                drop(file);

                rotate_files(&output_file_path, max_rotations);

                file = fs::File::create(&output_file_path)
                    .expect("unable to open output file after rotation");
            } else {
                file.set_len(0)
                    .and_then(|_| file.seek(io::SeekFrom::Start(0)))
                    .expect("failed to reset current log file");
            }

            file_size = 0;
        }

        {
            let mut lock = stdin.lock();
            bytes_read = match lock.read(&mut buffer) {
                Ok(n) => n,
                Err(e) => fail!(e)
            };
        }

        if bytes_read == 0 {
            if let Err(e) = file.sync_all() {
                fail!(e);
            }

            break;
        }

        if let Err(e) = file.write_all(&buffer[0..bytes_read]) {
            fail!(e);
        }

        file_size += bytes_read as u64;
    }
}
