use std::io::BufRead;

use crate::coupled_bufread::CoupledBufRead;

pub struct ContiguousBuffer<T: BufRead> {
    contiguous: Vec<u8>,
    buffer: CoupledBufRead<T>,
    position: usize,
}

impl<T> ContiguousBuffer<T>
where
    T: BufRead,
{
    pub fn new(unread: Vec<u8>, buffer: T) -> Self {
        ContiguousBuffer {
            contiguous: unread,
            buffer: CoupledBufRead::new(buffer),
            position: 0,
        }
    }

    fn vec(&self) -> &[u8] {
        if self.contiguous.is_empty() {
            self.buffer.data()
        } else {
            &self.contiguous
        }
    }

    fn copy_unread(&mut self) {
        if self.contiguous.is_empty() {
            // copy before getting more buffer
            // if self.contiguous is non-empty it means it's already copied
            // if it's fully consumed the [self.position..] is empty so no-op
            self.contiguous.extend(&self.buffer.data()[self.position..]);
            self.position = 0;
        }
        self.buffer.consume_all();
    }

    pub fn more_buf(&mut self) -> Result<(), std::io::Error> {
        self.copy_unread();
        let data = self.buffer.fill_buf()?;
        if data.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "No more buffer",
            ));
        }

        if !self.contiguous.is_empty() {
            // we have unconsumed data, so copy it to make it contiguous
            self.contiguous.extend(data);
        }

        Ok(())
    }

    pub fn consume(&mut self, amount: usize) {
        let new_position = self.position + amount;
        let vec = self.vec();
        assert!(vec.len() >= new_position);
        if vec.len() == new_position && !self.contiguous.is_empty() {
            self.contiguous.clear();
            self.position = self.buffer.data().len();
        } else {
            self.position = new_position;
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.vec()[self.position..]
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.as_slice().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn take_unread(mut self) -> Vec<u8> {
        self.copy_unread();
        self.contiguous
    }
}

#[cfg(test)]
mod tests {
    use std::io::BufReader;

    use super::ContiguousBuffer;

    #[test]
    fn consume_all() {
        let vec = vec![1, 2, 3];
        let mut buffer = ContiguousBuffer::new(Vec::new(), &vec[..]);
        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [1, 2, 3]);

        buffer.consume(1);
        assert_eq!(buffer.as_slice(), [2, 3]);

        buffer.consume(2);
        assert_eq!(buffer.as_slice(), []);
    }

    #[test]
    fn consume_and_more() {
        let vec = vec![1, 2, 3, 4];
        let reader = BufReader::with_capacity(2, &vec[..]);
        let mut buffer = ContiguousBuffer::new(Vec::new(), reader);
        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [1, 2]);

        buffer.consume(2);
        assert_eq!(buffer.as_slice(), []);

        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [3, 4]);
    }

    #[test]
    fn partial_consume() {
        let vec = vec![1, 2, 3, 4, 5];
        let reader = BufReader::with_capacity(2, &vec[..]);
        let mut buffer = ContiguousBuffer::new(Vec::new(), reader);
        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [1, 2]);

        buffer.consume(1);
        assert_eq!(buffer.as_slice(), [2]);

        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [2, 3, 4]);

        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [2, 3, 4, 5]);
    }

    #[test]
    fn partial_consume_and_more() {
        let vec = vec![1, 2, 3, 4, 5];
        let reader = BufReader::with_capacity(2, &vec[..]);
        let mut buffer = ContiguousBuffer::new(Vec::new(), reader);
        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [1, 2]);

        buffer.consume(1);
        assert_eq!(buffer.as_slice(), [2]);

        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [2, 3, 4]);

        buffer.consume(3);
        assert_eq!(buffer.as_slice(), []);

        buffer.more_buf().unwrap();
        assert_eq!(buffer.as_slice(), [5]);
    }
}
