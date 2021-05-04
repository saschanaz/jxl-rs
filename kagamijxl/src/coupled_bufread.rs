use std::io::BufRead;

pub struct CoupledBufRead<T: BufRead> {
    buf_read: T,
    data: Box<[u8]>,
}

impl<T> CoupledBufRead<T>
where
    T: BufRead,
{
    pub fn new(buf_read: T) -> CoupledBufRead<T> {
        CoupledBufRead {
            buf_read,
            data: Box::from([]),
        }
    }

    pub fn fill_buf(&mut self) -> Result<&[u8], std::io::Error> {
        self.data = Box::from(self.buf_read.fill_buf()?);
        Ok(self.data.as_ref())
    }

    pub fn consume_all(&mut self) {
        self.buf_read.consume(self.data.len());
    }

    pub fn data(&self) -> &[u8] {
        self.data.as_ref()
    }
}
