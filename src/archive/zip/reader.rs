use core::panic;
use std::fs::{File, Metadata};
use std::io::{BufReader, Read, Result, Seek, SeekFrom};
use std::path::Path;
use std::vec;

pub struct ZipFileReader {
    metadata: Metadata,
    reader: BufReader<File>,
    encoding: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CentralDirectoryFileHeader {
    pub file_name: String,
    pub uncompressed_size: u32,
    pub general_purpose_bit_flag: [u8; 2],
}

impl ZipFileReader {
    const END_OF_CENTRAL_DIR_SIGNATURE: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const CENTRAL_DIRECTORY_ENTRY_SIGNATURE: [u8; 4] = [0x50, 0x4B, 0x01, 0x02];

    pub fn new<P: AsRef<Path>>(path: P, encoding: String) -> ZipFileReader {
        let file = File::open(path).unwrap();
        ZipFileReader {
            metadata: file.metadata().unwrap(),
            reader: BufReader::new(file),
            encoding: encoding,
        }
    }

    pub fn seek_end_of_central_directory_record(&mut self) -> Result<()> {
        let file_size = self.metadata.len();
        self.reader.seek(SeekFrom::Start(file_size))?;
        self.reader.seek_relative(-18)?;
        // Since the length of file comment is variable, search by following.
        self.reader.seek_relative(-4)?;
        let mut comment_length = 0;
        while (comment_length + self.reader.stream_position()?) != file_size {
            let mut buf = [0u8; 4];
            while buf != Self::END_OF_CENTRAL_DIR_SIGNATURE {
                let mut offset = 0;
                self.reader.read(&mut buf)?;
                for (i, val) in buf.iter().rev().enumerate() {
                    if Self::END_OF_CENTRAL_DIR_SIGNATURE
                        [Self::END_OF_CENTRAL_DIR_SIGNATURE.len() - 1 - i]
                        != *val
                    {
                        offset += 1;
                    }
                }
                self.reader.seek_relative(-i64::try_from(offset).unwrap())?;
            }

            self.reader.seek_relative(16)?;

            comment_length = {
                let mut buf = [0u8; 2];
                self.reader.read(&mut buf)?;
                u64::from(u16::from_le_bytes(buf))
            };
        }
        self.reader.seek_relative(-22)?;
        Ok(())
    }

    pub fn read_central_directory_file_header(&mut self) -> Vec<CentralDirectoryFileHeader> {
        self.reader.seek_relative(10).unwrap();

        let total_number_of_central_directory_records = {
            let mut buf = [0u8; 2];
            self.reader.read(&mut buf).unwrap();
            u16::from_le_bytes(buf) as usize
        };

        let mut central_directory_reocrds: Vec<CentralDirectoryFileHeader> =
            Vec::with_capacity(total_number_of_central_directory_records);

        self.reader.seek_relative(4).unwrap();

        let offset_of_start_of_central_directory = {
            let mut buf = [0u8; 4];
            self.reader.read(&mut buf).unwrap();
            u64::from(u32::from_le_bytes(buf))
        };

        self.reader
            .seek(SeekFrom::Start(offset_of_start_of_central_directory))
            .unwrap();

        let mut comment_length = 0;
        for _n in 0..total_number_of_central_directory_records {
            self.reader.seek_relative(comment_length).unwrap();

            let mut buf = [0u8; 4];
            self.reader.read(&mut buf).unwrap();

            assert_eq!(buf, Self::CENTRAL_DIRECTORY_ENTRY_SIGNATURE);

            self.reader.seek_relative(4).unwrap();

            let general_purpose_bit_flag = {
                let mut buf = [0u8; 2];
                self.reader.read(&mut buf).unwrap();
                buf
            };

            self.reader.seek_relative(14).unwrap();

            let uncompressed_size = {
                let mut buf = [0u8; 4];
                self.reader.read(&mut buf).unwrap();
                u32::from_le_bytes(buf)
            };

            let file_name_length = {
                let mut buf = [0u8; 2];
                self.reader.read(&mut buf).unwrap();
                u16::from_le_bytes(buf) as usize
            };

            let extra_field_length = {
                let mut buf = [0u8; 2];
                self.reader.read(&mut buf).unwrap();
                i64::from(u16::from_le_bytes(buf))
            };

            comment_length = {
                let mut buf = [0u8; 2];
                self.reader.read(&mut buf).unwrap();
                i64::from(u16::from_le_bytes(buf))
            };

            self.reader.seek_relative(12).unwrap();

            let file_name = {
                let mut buf = vec![0u8; file_name_length];
                self.reader.read_exact(&mut buf).unwrap();
                if Self::is_utf8(general_purpose_bit_flag) {
                    String::from_utf8(buf).unwrap()
                } else {
                    match self.encoding.as_ref() {
                        "utf8" => match String::from_utf8(buf.clone()) {
                            Ok(v) => v,
                            Err(e) => {
                                if cfg!(windows) {
                                    // TODO Consider locale.
                                    encoding_rs::SHIFT_JIS.decode(&buf).0.into_owned()
                                } else {
                                    panic!("{}", e);
                                }
                            }
                        },
                        "cp932" => encoding_rs::SHIFT_JIS.decode(&buf).0.into_owned(),
                        _ => panic!("invalid encoding: {}", self.encoding),
                    }
                }
            };

            self.reader.seek_relative(extra_field_length).unwrap();

            central_directory_reocrds.push(CentralDirectoryFileHeader {
                file_name,
                uncompressed_size,
                general_purpose_bit_flag,
            })
        }

        central_directory_reocrds
    }

    fn is_utf8(general_purpose_bit_flag: [u8; 2]) -> bool {
        (general_purpose_bit_flag[0] >> 5) & 1 == 1
    }
}
