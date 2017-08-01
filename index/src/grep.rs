use twoway;

use std::str;
use std::io;
use std::io::Read;

/// fooBARbaz
/// search: BAR
/// block size: 4
/// "fooB" -> false.
/// retain 2 characters?
/// "oBAR" -> 2, plus 4 - 2 -> 4?
/// Worst example ever.
pub fn reader_contains<R: Read>(needle: &[u8], haystack: R) -> io::Result<Option<u64>> {
    reader_contains_external_buf(needle, haystack, &mut [0u8; 16 * 1024])
}

#[inline]
fn reader_contains_external_buf<R: Read>(
    needle: &[u8],
    mut haystack: R,
    buf: &mut [u8],
) -> io::Result<Option<u64>> {
    if needle.is_empty() {
        return Ok(Some(0));
    }

    let mut total_read = 0u64;

    assert!(needle.len() < buf.len());

    let mut fill = 0;

    loop {
        while fill < needle.len() {
            fill += match haystack.read(&mut buf[fill..])? {
                0 => break,
                bytes => bytes,
            };
            //            println!("fill: {} '{}'", fill, str::from_utf8(&buf[..fill]).unwrap());
        }

        if let Some(buf_pos) = twoway::find_bytes(&buf[..fill], needle) {
            return Ok(Some(buf_pos as u64 + total_read));
        }

        if fill < needle.len() {
            return Ok(None);
        }

        for i in 0..(needle.len() - 1) {
            buf[i] = buf[fill - needle.len() + i + 1];
        }

        total_read += fill as u64 - needle.len() as u64 + 1;

        fill = needle.len() - 1;
    }
}

#[cfg(test)]
mod tests {
    use std::io;
    use super::reader_contains;
    use super::reader_contains_external_buf;
    const MSG: &str = "Inches aren't very granular.";

    #[test]
    fn single_block() {

        assert_eq!(
            Some(0),
            reader_contains("Inches".as_bytes(), cursor(MSG)).unwrap()
        );
        assert_eq!(
            Some(2),
            reader_contains("ches".as_bytes(), cursor(MSG)).unwrap()
        );
        assert_eq!(
            Some(25),
            reader_contains("ar.".as_bytes(), cursor(MSG)).unwrap()
        );
        assert_eq!(
            None,
            reader_contains("inch".as_bytes(), cursor(MSG)).unwrap()
        );
    }

    #[test]
    fn reduced_buf() {
        for buf_size in 5..(MSG.len() + 5) {
            let mut buf = [0u8; 64];
            let buf = &mut buf[..buf_size];
            assert_eq!(
                Some(0),
                reader_contains_external_buf("Inch".as_bytes(), cursor(MSG), buf).unwrap()
            );

            zero(buf);
            assert_eq!(
                Some(2),
                reader_contains_external_buf("ches".as_bytes(), cursor(MSG), buf).unwrap()
            );

            zero(buf);
            assert_eq!(
                Some(25),
                reader_contains_external_buf("ar.".as_bytes(), cursor(MSG), buf).unwrap()
            );
        }
    }

    fn zero(buf: &mut [u8]) {
        for i in 0..buf.len() {
            buf[i] = 0;
        }
    }

    fn cursor(s: &str) -> io::Cursor<&str> {
        io::Cursor::new(s)
    }
}
