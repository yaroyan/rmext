use std::fs::{File, Metadata};
use std::io::BufReader;

pub struct RarFileReader {
    metadata: Metadata,
    reader: BufReader<File>,
}
