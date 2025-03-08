// core file trait that anything involving reading or writing implements

pub trait File {
    /// Read from the file into the buffer
    fn read(&mut self, buf: &mut [u8]) -> usize;

    /// Write from the buffer into the file
    fn write(&mut self, buf: &[u8]) -> usize;

    /// Seek to a position in the file
    fn seek(&mut self, pos: usize) -> usize;
}