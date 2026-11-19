//! Đầu vào hỏng/ngẫu nhiên (bản mã từ máy chủ, tệp trên đĩa bị sửa): chỉ trả Err, không panic, không chấp nhận nhầm.
//! Bổ sung cho `fuzz/fuzz_targets/vault_open.rs` (cần nightly + cargo-fuzz).
#![allow(clippy::unwrap_used)]

use tsuscan_vault::envelope::{self, Kind};
use tsuscan_vault::{Key32, hlc::Hlc, keys, stream};

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // xorshift64*: đủ cho sinh đột biến xác định.
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn mutate(rng: &mut Rng, src: &[u8]) -> Vec<u8> {
    let mut v = src.to_vec();
    match rng.below(4) {
        0 => {
            let i = rng.below(v.len());
            v[i] ^= 1 << rng.below(8);
        }
        1 => v.truncate(rng.below(v.len())),
        2 => {
            let extra = 1 + rng.below(300);
            v.extend((0..extra).map(|_| rng.next() as u8));
        }
        _ => {
            let (a, b) = (rng.below(v.len()), rng.below(v.len()));
            v.swap(a, b);
            if v == src {
                v.push(0);
            }
        }
    }
    v
}

#[test]
fn mutated_envelopes_and_streams_never_accepted() {
    let key = Key32::from_bytes([3; 32]);
    let env = envelope::seal(&key, Kind::Record, b"ctx", b"Ho ten: Nguyen Van A").unwrap();
    let data: Vec<u8> = (0..70_000u32).map(|i| (i % 7) as u8).collect();
    let mut st = Vec::new();
    stream::encrypt(&key, b"fk|1", data.len() as u64, &data[..], &mut st).unwrap();
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for _ in 0..3000 {
        let m = mutate(&mut rng, &env);
        assert!(envelope::open(&key, Kind::Record, b"ctx", &m).is_err());
    }
    for _ in 0..300 {
        let m = mutate(&mut rng, &st);
        let mut out = Vec::new();
        assert!(stream::decrypt(&key, b"fk|1", &m[..], &mut out).is_err());
    }
}

#[test]
fn random_bytes_and_text_do_not_panic() {
    let key = Key32::from_bytes([4; 32]);
    let mut rng = Rng(42);
    for _ in 0..5000 {
        let n = rng.below(200);
        let mut b: Vec<u8> = (0..n).map(|_| rng.next() as u8).collect();
        if n >= 4 && rng.below(2) == 0 {
            // Đầu hợp lệ để đi sâu vào bộ phân tích.
            b[..4].copy_from_slice(if rng.below(2) == 0 {
                b"TS\x01\x03"
            } else {
                b"TS\x01\x01"
            });
        }
        let _ = envelope::open(&key, Kind::Record, b"", &b);
        let _ = keys::unwrap(&key, "dk|x", &b);
        let mut out = Vec::new();
        assert!(stream::decrypt(&key, b"", &b[..], &mut out).is_err());
        let s = String::from_utf8_lossy(&b);
        let _ = Hlc::decode(&s);
        assert!(keys::RecoveryPhrase::parse(&s).is_err());
    }
}
