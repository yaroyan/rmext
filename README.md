## Usage

```bash
# Execute against .zip files under a specific directory.
# Windows
while read -r f; do echo "$f" ; rmext -p "$f" -m 3 -r -e cp932; done < <(find -iname "*.zip")
# Linux
while read -r f; do echo "$f" ; rmext -p "$f" -m 3 -r; done < <(find -iname "*.zip")
```

## Zip file structure

### Local file header

| Offset | Bytes | Description |
|:-|:-|:-|
| 0 | 4 | Local file header signature = 0x04034b50 (PK♥♦ or "PK\3\4")
| 4 | 2 | Version needed to extract (minimum)
| 6 | 2 | General purpose bit flag
| 8 | 2 | Compression method; e.g. none = 0, DEFLATE = 8 (or "\0x08\0x00")
| 10 | 2 | File last modification time
| 12 | 2 | File last modification date
| 14 | 4 | CRC-32 of uncompressed data
| 18 | 4 | Compressed size (or 0xffffffff for ZIP64)
| 22 | 4 | Uncompressed size (or 0xffffffff for ZIP64)
| 26 | 2 | File name length (n)
| 28 | 2 | Extra field length (m)
| 30 | n | File name
| 30 + n | m | Extra field

### Central directory file header

| Offset | Bytes | Description |
|:-|:-|:-|
| 0 | 4 | Central directory file header signature = 0x02014b50
| 4 | 2 | Version made by
| 6 | 2 | Version needed to extract (minimum)
| 8 | 2 | General purpose bit flag
| 10 | 2 | Compression method
| 12 | 2 | File last modification time
| 14 | 2 | File last modification date
| 16 | 4 | CRC-32 of uncompressed data
| 20 | 4 | Compressed size (or 0xffffffff for ZIP64)
| 24 | 4 | Uncompressed size (or 0xffffffff for ZIP64)
| 28 | 2 | File name length (n)
| 30 | 2 | Extra field length (m)
| 32 | 2 | File comment length (k)
| 34 | 2 | Disk number where file starts (or 0xffff for ZIP64)
| 36 | 2 | Internal file attributes
| 38 | 4 | External file attributes
| 42 | 4 | Relative offset of local file header (or 0xffffffff for ZIP64). This is the number of bytes between the start of the first disk on which the file occurs, and the start of the local file header. This allows software reading the central directory to locate the position of the file inside the ZIP file.
| 46 | n | File name
| 46 + n | m | Extra field
| 46 + n + m | k | File comment

### End of central directory record (EOCD)

| Offset | Bytes | Description |
|:-|:-|:-|
| 0 | 4 | End of central directory signature = 0x06054b50
| 4 | 2 | Number of this disk (or 0xffff for ZIP64)
| 6 | 2 | Disk where central directory starts (or 0xffff for ZIP64)
| 8 | 2 | Number of central directory records on this disk (or 0xffff for ZIP64)
| 10 | 2 | Total number of central directory records (or 0xffff for ZIP64)
| 12 | 4 | Size of central directory (bytes) (or 0xffffffff for ZIP64)
| 16 | 4 | Offset of start of central directory, relative to start of archive (or 0xffffffff for ZIP64)
| 20 | 2 | Comment length (n)
| 22 | n | Comment

### Reference

https://en.wikipedia.org/wiki/ZIP_(file_format)