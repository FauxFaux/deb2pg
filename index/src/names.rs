use std::cmp::{max, min};

const BLOCK_SIZE: u64 = 1024 * 1024 * 1024;
const MIN_SHARD_NO: u8 = 2;
const SHARD_NO_TEXT_OFFSET: u8 = 8;

fn make_shard_magic(size: u64) -> u8 {
    if 0 == size {
        return MIN_SHARD_NO;
    }

    min(9, max(MIN_SHARD_NO as u64, (size as f64).log10() as u64)) as u8
}

fn text_otherwise_bin(text: bool) -> &'static str {
    if text {
        "text"
    } else {
        "bin"
    }
}

pub fn magic_offset_only(len: u64, text: bool) -> u8 {
    make_shard_magic(len) - MIN_SHARD_NO + if text { SHARD_NO_TEXT_OFFSET } else { 0 }
}

pub fn name_for_magic(magic: u8) -> String {
    assert_lt!(magic, 16);
    format!(
        "{}-{}",
        text_otherwise_bin(magic >= SHARD_NO_TEXT_OFFSET),
        magic % SHARD_NO_TEXT_OFFSET + MIN_SHARD_NO
    )
}

/// Returns the base filename, and the magic value (0-15) added to a position.
pub fn magic_offset(len: u64, text: bool) -> (String, u8) {
    let shard_magic = make_shard_magic(len);
    let shard_id = shard_magic - MIN_SHARD_NO + if text { SHARD_NO_TEXT_OFFSET } else { 0 };
    (name_for_magic(shard_id), shard_id)
}

pub fn filename_for(pos: u64) -> (String, u32) {
    let mut magic = (pos % 16) as u8;
    let real_pos = pos - (magic as u64);
    let text = if magic >= SHARD_NO_TEXT_OFFSET {
        magic -= SHARD_NO_TEXT_OFFSET;
        true
    } else {
        false
    };

    magic += MIN_SHARD_NO;

    let file_number = real_pos / BLOCK_SIZE;
    let file_pos = (real_pos % BLOCK_SIZE) as u32;

    let file_name = format!(
        "{}-{}.{:010}.cfp",
        text_otherwise_bin(text),
        magic,
        file_number
    );
    (file_name, file_pos)
}

// TODO: this is awful
pub fn addendum_from_path(path: &str) -> (u8, u64) {
    // text-5.0000000000000000000000.idx
    let text = if path.starts_with("text-") {
        true
    } else if path.starts_with("bin-") {
        false
    } else {
        panic!("path must start with 'text-' or 'bin-', not {}", path);
    };

    let mut it = path.chars().skip(if text { 5 } else { 4 });
    let size_raw = it.next().expect("num") as u8 - '2' as u8;
    assert_ge!(size_raw, 2 - 2);
    assert_le!(size_raw, 9 - 2);
    assert_eq!('.', it.next().unwrap());

    (
        size_raw + 2,
        // TODO: NO JUST NO WHY
        it.take("0000000000000000000000".len())
            .collect::<String>()
            .parse::<u64>()
            .expect("second num") * 1024 * 1024 * 1024 + (size_raw as u64) + if text {
            8
        } else {
            0
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn number() {
        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;

        assert_eq!(2, make_shard_magic(0));
        assert_eq!(2, make_shard_magic(1));
        assert_eq!(2, make_shard_magic(999));
        assert_eq!(3, make_shard_magic(1000));
        assert_eq!(3, make_shard_magic(1001));
        assert_eq!(3, make_shard_magic(1 * KB));
        assert_eq!(4, make_shard_magic(10 * KB));
        assert_eq!(5, make_shard_magic(100 * KB));
        assert_eq!(6, make_shard_magic(1 * MB));
        assert_eq!(6, make_shard_magic(2 * MB));
        assert_eq!(7, make_shard_magic(10 * MB));
        assert_eq!(8, make_shard_magic(100 * MB));
        assert_eq!(9, make_shard_magic(1 * GB));
        assert_eq!(9, make_shard_magic(10 * GB));
        assert_eq!(9, make_shard_magic(100 * GB));
        assert_eq!(9, make_shard_magic(1000 * GB));
    }

    #[test]
    fn filename() {
        assert_eq!(
            ("bin-2.0000000000000000000000".to_string(), 16),
            filename_for(16)
        );
        assert_eq!(
            ("bin-3.0000000000000000000000".to_string(), 16),
            filename_for(17)
        );
        assert_eq!(
            ("bin-8.0000000000000000000000".to_string(), 16),
            filename_for(22)
        );
        assert_eq!(
            ("bin-9.0000000000000000000000".to_string(), 16),
            filename_for(23)
        );

        assert_eq!(
            ("text-2.0000000000000000000000".to_string(), 16),
            filename_for(24)
        );
        assert_eq!(
            ("text-3.0000000000000000000000".to_string(), 16),
            filename_for(25)
        );

        assert_eq!(
            ("text-8.0000000000000000000000".to_string(), 16),
            filename_for(30)
        );
        assert_eq!(
            ("text-9.0000000000000000000000".to_string(), 16),
            filename_for(31)
        );

        assert_eq!(
            ("bin-2.0000000000000000000000".to_string(), 32),
            filename_for(32)
        );
        assert_eq!(
            ("text-9.0000000000000000000000".to_string(), 32),
            filename_for(32 + 15)
        );

        assert_eq!(
            ("bin-2.0000000000000000000001".to_string(), 16),
            filename_for(1024 * 1024 * 1024 + 16)
        );
        assert_eq!(
            ("text-9.0000000000000000000001".to_string(), 16),
            filename_for(1024 * 1024 * 1024 + 16 + 15)
        );

        assert_eq!(
            ("bin-2.0000000000000000000017".to_string(), 16),
            filename_for(17 * 1024 * 1024 * 1024 + 16)
        );
    }

    #[test]
    fn from_path() {
        assert_eq!((2, 0), addendum_from_path("bin-2.0000000000000000000000"));
        assert_eq!((2, 8), addendum_from_path("text-2.0000000000000000000000"));
        assert_eq!(
            (2, 17 * 1024 * 1024 * 1024),
            addendum_from_path("bin-2.0000000000000000000017")
        );
        assert_eq!(
            (2, 17 * 1024 * 1024 * 1024 + 8),
            addendum_from_path("text-2.0000000000000000000017")
        );

        assert_eq!(
            (3, 17 * 1024 * 1024 * 1024 + 9),
            addendum_from_path("text-3.0000000000000000000017")
        );
    }
}
