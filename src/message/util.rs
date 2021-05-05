use std::io::{prelude::*, Error, ErrorKind};

/// Compute the log-base-two of the next power of two: 8 -> 3, 9 -> 4.
///
pub fn ceil_log2(x: usize) -> usize {
    let mut n = 0;
    while 1 << n < x {
        n += 1
    }
    n
}

/// Read a usize out of the given stream.
///
pub fn read_usize<R: Read>(stream: &mut R) -> std::io::Result<usize> {
    Ok(usize::from_le_bytes(read_bytes_array(stream)?))
}

/// Read the given number of bytes from a stream, into a vec.
///
pub fn read_bytes_vec<R: Read>(stream: &mut R, size: usize) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0; size];
    read_bytes_into(stream, &mut buffer)?;
    Ok(buffer)
}

/// Read the given (const) number of bytes from a stream, into an array.
///
pub fn read_bytes_array<R: Read, const SIZE: usize>(stream: &mut R) -> std::io::Result<[u8; SIZE]> {
    let mut buffer = [0; SIZE];
    read_bytes_into(stream, &mut buffer)?;
    Ok(buffer)
}

/// Fill up the given buffer by reading bytes from a stream.
///
pub fn read_bytes_into<R: Read>(stream: &mut R, buffer: &mut [u8]) -> std::io::Result<()> {
    let mut cursor = 0;
    while cursor < buffer.len() {
        let bytes_read = stream.read(&mut buffer[cursor..])?;
        if bytes_read == 0 {
            return Err(Error::new(ErrorKind::InvalidData, "Read returned 0 bytes"));
        } else {
            cursor += bytes_read;
        }
    }
    Ok(())
}
