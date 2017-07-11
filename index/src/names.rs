use std::cmp::{min, max};

const BLOCK_SIZE: u64 = 1024 * 1024 * 1024;

fn make_shard_magic(size: u64) -> u8 {
    if 0 == size {
        return 2;
    }

    min(9, max(2, (size as f64).log10() as u64)) as u8
}

fn text_otherwise_bin(text: bool) -> &'static str {
    if text { "text" } else { "bin" }
}

/// Returns the base filename, and the magic value (0-15) added to a position.
pub fn magic_offset(len: u64, text: bool) -> (String, u8) {
    let shard_magic = make_shard_magic(len);
    let shard_name = format!("{}-{}", text_otherwise_bin(text), shard_magic);
    let shard_id = shard_magic - 2 + if text { 8 } else { 0 };
    (shard_name, shard_id)
}

pub fn filename_for(pos: u64) -> (String, u32) {
    let mut magic = pos % 16;
    let real_pos = pos - magic;
    let text = if magic >= 8 {
        magic -= 8;
        true
    } else {
        false
    };

    magic += 2;

    let file_number = real_pos / BLOCK_SIZE;
    let file_pos = (real_pos % BLOCK_SIZE) as u32;

    let file_name = format!("{}-{}.{:022}", text_otherwise_bin(text), magic, file_number);
    (file_name, file_pos)
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
        assert_eq!(("bin-2.0000000000000000000000".to_string(), 16), filename_for(16));
        assert_eq!(("bin-3.0000000000000000000000".to_string(), 16), filename_for(17));
        assert_eq!(("bin-8.0000000000000000000000".to_string(), 16), filename_for(22));
        assert_eq!(("bin-9.0000000000000000000000".to_string(), 16), filename_for(23));

        assert_eq!(("text-2.0000000000000000000000".to_string(), 16), filename_for(24));
        assert_eq!(("text-3.0000000000000000000000".to_string(), 16), filename_for(25));

        assert_eq!(("text-8.0000000000000000000000".to_string(), 16), filename_for(30));
        assert_eq!(("text-9.0000000000000000000000".to_string(), 16), filename_for(31));

        assert_eq!(("bin-2.0000000000000000000000".to_string(), 32), filename_for(32));
        assert_eq!(("text-9.0000000000000000000000".to_string(), 32), filename_for(32 + 15));

        assert_eq!(("bin-2.0000000000000000000001".to_string(), 16), filename_for(1024 * 1024 * 1024 + 16));
        assert_eq!(("text-9.0000000000000000000001".to_string(), 16), filename_for(1024 * 1024 * 1024 + 16 + 15));

        assert_eq!(("bin-2.0000000000000000000017".to_string(), 16), filename_for(17 * 1024 * 1024 * 1024 + 16));

    }
}
