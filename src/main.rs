mod archive;
use crate::archive::zip::reader;
use archive::zip::reader::CentralDirectoryFileHeader;
use atty::Stream;
use clap::CommandFactory;
use clap::Parser;
use core::panic;
use std::{
    collections::HashSet,
    fs,
    io::{self, Read, Result, Write},
    path::{Path, PathBuf},
};

/// Simple program to delete the contents extracted from the archive.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to archive file.
    #[arg(short, long)]
    path: Option<String>,

    /// Mode 1: file 2: directory 3: file and directory   
    #[arg(long, short, default_value_t = 3)]
    mode: u8,

    /// Delete files interactively.
    #[arg(long, short)]
    interactive: bool,

    /// Character code used for encoding when Bit 11 of general purpose bit flag is 0.
    #[arg(long, short, default_value = "utf8")]
    encoding: String,

    /// Deletes directories that will be empty after a file is deleted.
    #[arg(long, short)]
    recursive: bool,

    /// List archive contents.
    #[arg(long, short)]
    list: bool,
}

const ALLOWED_ENCODINGS: &'static [&'static str] = &["utf8", "cp932"];
const ALLOWED_CODES: &'static [u8] = &[1, 2, 3];

fn main() -> Result<()> {
    let args = Args::parse();
    // let args = Args {
    //     path: Some("archive.zip".to_string()),
    //     mode: 3,
    //     interactive: true,
    //     recursive: true,
    //     encoding: "cp932".to_string(),
    //     list: false,
    // };
    let archive_path = if args.path.is_none() {
        if is_stdin(args.path.as_ref()) {
            PathBuf::from(read_from_stdin().unwrap())
        } else {
            // Print help.
            let mut cmd = Args::command();
            let _ = cmd.print_help();
            std::process::exit(1);
        }
    } else {
        PathBuf::from(args.path.unwrap())
    };

    // Validate arguments.
    assert!(ALLOWED_ENCODINGS.contains(&args.encoding.to_lowercase().as_ref()));
    assert!(ALLOWED_CODES.contains(&args.mode));

    let paths_to_delete = match archive_path.extension().unwrap().to_string_lossy().as_ref() {
        "zip" => {
            let mut reader = reader::ZipFileReader::new(&archive_path, args.encoding.to_string());
            reader.seek_end_of_central_directory_record().unwrap();
            let headers = reader.read_central_directory_file_header();
            let mut codes = unpack_mode(args.mode);
            codes.sort();
            let mut paths_to_delete = Vec::new();
            for code in &codes {
                let search_path = match code {
                    1 => archive_path.parent().unwrap().to_path_buf(),
                    2 => Path::new(&archive_path.parent().unwrap())
                        .join(archive_path.file_stem().unwrap()),
                    _ => panic!("invalid mode."),
                };
                let content_paths = search_zip_content_path_to_delete(&headers, &search_path);
                paths_to_delete.extend(content_paths);
            }
            paths_to_delete.sort();
            paths_to_delete
        }
        "rar" => search_rar_content_path_to_delete(
            &archive_path,
            &args.encoding,
            &archive_path.parent().unwrap().to_path_buf(),
        ),
        _ => panic!("unsupported file type: {}", archive_path.to_string_lossy()),
    };

    if paths_to_delete.is_empty() {
        println!("Archive contents are not found.");
        println!("Skip removing.");
        return Ok(());
    }

    println!("The following files will be Removed:");
    for delete_dir in &paths_to_delete {
        println!("\t{}", delete_dir.to_string_lossy());
    }

    if args.list {
        println!("Skip removing.");
        return Ok(());
    }

    print!("Do you want to continue? [Y/n] ");
    std::io::stdout().flush().unwrap();

    let mut buffer;
    if args.interactive {
        buffer = String::new();
        io::stdin()
            .read_line(&mut buffer)
            .expect("Failed to read line");
    } else {
        buffer = String::from("y");
        println!("{}", buffer);
    }

    if buffer.trim().to_lowercase() == "y" {
        for path in &paths_to_delete {
            remove_file(path);
        }

        println!("Remove empty directory recursively.");

        if args.recursive {
            let mut ancestor_paths_to_delete = HashSet::new();
            let parent = archive_path.parent().unwrap();
            for path in &paths_to_delete {
                for ancestor in path.ancestors() {
                    if ancestor_paths_to_delete.contains(&ancestor) || parent == ancestor {
                        break;
                    }
                    if ancestor.is_dir() {
                        ancestor_paths_to_delete.insert(ancestor);
                    }
                }
            }
            let mut ancestor_paths_to_delete_sort_by_depth =
                Vec::from_iter(ancestor_paths_to_delete);
            sort_path_by_depth(&mut ancestor_paths_to_delete_sort_by_depth);

            for path in ancestor_paths_to_delete_sort_by_depth {
                if !path.read_dir().unwrap().next().is_none() {
                    println!(
                        "\t{} is not empty. Skip removing.",
                        path.to_string_lossy().into_owned()
                    );
                    continue;
                }
                remove_file(path);
            }
        }
    } else {
        println!("Abort.");
    }
    Ok(())
}

/// Detect stdin.
fn is_stdin(input: Option<&String>) -> bool {
    let is_request = match input {
        Some(i) if i == "-" => true,
        _ => false,
    };
    let is_pipe = !atty::is(Stream::Stdin);
    is_request || is_pipe
}

/// Read from stdin.
fn read_from_stdin() -> Result<String> {
    let mut buf = String::new();
    let stdin = io::stdin();
    let mut handle = stdin.lock();
    handle.read_to_string(&mut buf)?;
    Ok(buf.trim().to_string())
}

/// Unpack mode to codes.
fn unpack_mode(mode: u8) -> Vec<u32> {
    let two: u8 = 2;
    let mut code = 1;
    let mut codes = Vec::new();
    while mode >= two.pow(code - 1) {
        if two.pow(code - 1) <= mode % two.pow(code) {
            codes.push(code);
        }
        code += 1;
    }
    codes
}

/// Remove file.
fn remove_file<P: AsRef<Path>>(path: P) {
    let p = path.as_ref();
    if p.is_dir() {
        match fs::remove_dir(p) {
            Ok(_) => {
                println!("\tRemoved: {}.", p.to_string_lossy().into_owned());
            }
            Err(e) => eprintln!(
                "Failed to remove {}: {}",
                p.to_string_lossy().into_owned(),
                e
            ),
        }
    } else {
        match fs::remove_file(p) {
            Ok(_) => {
                println!("\tRemoved: {}.", p.to_string_lossy().into_owned());
            }
            Err(e) => eprintln!(
                "Failed to remove {}: {}",
                p.to_string_lossy().into_owned(),
                e
            ),
        }
    }
}

/// Search path to delete.
fn search_rar_content_path_to_delete<P: AsRef<Path>>(
    zip_path: P,
    encoding: &str,
    search_path: P,
) -> Vec<PathBuf> {
    panic!("Not Implemented.");
}

/// Search path to delete.
fn search_zip_content_path_to_delete<P: AsRef<Path>>(
    headers: &Vec<CentralDirectoryFileHeader>,
    search_path: P,
) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for header in headers {
        let content_path = search_path
            .as_ref()
            .join(normalize_file_name(&header.file_name));
        if content_path.exists()
            && content_path.is_file()
            && content_path.metadata().unwrap().len() == u64::from(header.uncompressed_size)
        {
            paths.push(content_path);
        }
    }
    paths
}

/// Normalize zip content file name.  
/// e.g.) `../A/../A/./B.txt => A/A/B.txt`
fn normalize_file_name(file_name: &str) -> String {
    if cfg!(windows) {
        file_name
            .replace("/", "\\")
            .replace("..\\", "")
            .replace(".\\", "")
    } else {
        file_name
            .replace("\\", "/")
            .replace("../", "")
            .replace("./", "")
    }
}

/// Sort path by depth
fn sort_path_by_depth<P: AsRef<Path>>(paths: &mut Vec<P>) {
    let separator = if cfg!(windows) { "\\" } else { "/" };
    paths.sort_by(|a, b| {
        b.as_ref()
            .to_string_lossy()
            .into_owned()
            .matches(separator)
            .count()
            .cmp(
                &a.as_ref()
                    .to_string_lossy()
                    .into_owned()
                    .matches(separator)
                    .count(),
            )
            .then(a.as_ref().cmp(&b.as_ref()))
    });
}

#[cfg(test)]
mod tests {
    use crate::{normalize_file_name, unpack_mode};

    #[test]
    fn unpack_all_delete_mode() {
        let mut codes = Vec::new();
        codes.push(1);
        codes.push(2);
        assert_eq!(codes, unpack_mode(3));
    }

    #[test]
    fn trim_dot_slash_in_path() {
        assert_eq!("A/A/B.txt", normalize_file_name("../A/../A/./B.txt"));
    }
}
