use std::io::{Read, Seek, Write};

pub fn read(img_file: &str, block: u32, buf: &mut [u8])  -> Result<(), ()> {
    // write to img_file
    println!("Reading from {} block {}", img_file, block);
    // open img_file
    let mut file = std::fs::File::open(img_file).unwrap();
    // seek to block
    file.seek(std::io::SeekFrom::Start(block as u64 * 512)).unwrap();
    // read into buf
    file.read_exact(buf).unwrap();
    Ok(())
}

pub fn write(img_file: &str, block: u32, buf: &[u8]) -> Result<(), ()> {
    // open img_file
    let mut file = std::fs::OpenOptions::new().write(true).open(img_file).unwrap();
    // seek to block
    file.seek(std::io::SeekFrom::Start(block as u64 * 512)).unwrap();
    // write buf
    file.write_all(buf).unwrap();
    Ok(())
}
