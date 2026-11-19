//! Sinh `conformance/vault-v1.json` (ADR-0009 mục 3.7). Nonce/muối cố định ⇒ đầu ra xác định.
//! Chạy: `cargo run -p tsuscan-vault --example gen_conformance > conformance/vault-v1.json`
//! Đổi định dạng ⇒ tăng phiên bản + sinh lại CÙNG PR (AGENTS.md B.3).
#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use serde_json::{Value, json};
use tsuscan_vault::envelope::{self, Kind};
use tsuscan_vault::keys::{self, KdfParams, RecoveryPhrase};
use tsuscan_vault::{Key32, stream};

fn hex(b: &[u8]) -> String {
    Key32::hex_of(b)
}

fn seq(n: usize, mul: usize) -> Vec<u8> {
    (0..n).map(|i| ((i * mul + 7) % 256) as u8).collect()
}

fn main() {
    let key = Key32::from_bytes(core::array::from_fn(|i| i as u8));
    let nonce: [u8; 24] = core::array::from_fn(|i| 0xa0 + i as u8);
    let mut envelopes = Vec::new();
    let cases: [(&str, Kind, &str, Vec<u8>); 5] = [
        ("record-empty", Kind::Record, "rec|doc", Vec::new()),
        (
            "record-vi",
            Kind::Record,
            "rec|doc|0011",
            "Hợp đồng thuê nhà - Điều 1".as_bytes().to_vec(),
        ),
        ("record-300", Kind::Record, "rec|page", seq(300, 31)),
        (
            "key-dk",
            Kind::Key,
            &keys::context::document("0123456789abcdef0123456789abcdef"),
            seq(32, 5),
        ),
        ("key-mk-pw", Kind::Key, keys::context::MK_PASSWORD, seq(32, 11)),
    ];
    for (name, kind, ctx, pt) in cases {
        let sealed = envelope::seal_with_nonce(&key, kind, ctx.as_bytes(), &pt, &nonce).unwrap();
        envelopes.push(json!({
            "name": name, "kind": kind as u8, "padBlock": kind.block(), "context": ctx,
            "nonceHex": hex(&nonce), "plaintextHex": hex(&pt), "sealedHex": hex(&sealed),
        }));
    }

    let prefix: [u8; stream::PREFIX_LEN] = core::array::from_fn(|i| 0x50 + i as u8);
    let mut streams = Vec::new();
    for (name, len) in [
        ("stream-0", 0usize),
        ("stream-100", 100),
        ("stream-2-chunks", 65_600),
    ] {
        let pt = seq(len, 13);
        let ctx = keys::context::file("fedcba9876543210fedcba9876543210");
        let mut ct = Vec::new();
        stream::encrypt_with_prefix(&key, ctx.as_bytes(), len as u64, &pt[..], &mut ct, &prefix).unwrap();
        streams.push(json!({
            "name": name, "context": ctx, "plaintextLen": len, "plaintextRule": "byte i = (13*i + 7) mod 256",
            "paddedLen": stream::padme(len as u64 + 8), "ciphertextHex": hex(&ct),
        }));
    }

    // Mật khẩu ở dạng NFD (dấu tách rời) để bên kiểm phải chuẩn hóa NFKC trước Argon2id.
    let password = "Mật khẩu thử: Đắk Lắk 2026";
    let password_nfd: String = unicode_normalization::UnicodeNormalization::nfd(password).collect();
    let salt: [u8; 16] = core::array::from_fn(|i| 0x10 + i as u8);
    let params = KdfParams::FLOOR;
    let kek = keys::password_kek(&password_nfd, &salt, params).unwrap();

    let entropy: [u8; 32] = core::array::from_fn(|i| (i * 7 + 3) as u8);
    let phrase = RecoveryPhrase::from_entropy(&entropy).unwrap();
    let rec = phrase.kek().unwrap();

    let out: Value = json!({
        "format": "tsuscan-vault conformance v1 (ADR-0009)",
        "keyHex": hex(key.as_bytes()),
        "envelopes": envelopes,
        "stream": { "prefixHex": hex(&prefix), "chunk": stream::CHUNK, "streams": streams },
        "argon2id": {
            "passwordNfd": password_nfd, "saltHex": hex(&salt),
            "mKib": params.m_kib, "t": params.t, "p": params.p, "keyHex": hex(kek.as_bytes()),
        },
        "recovery": {
            "entropyHex": hex(&entropy), "words": phrase.as_str(),
            "hkdfInfo": String::from_utf8_lossy(keys::RECOVERY_INFO), "keyHex": hex(rec.as_bytes()),
        },
    });
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
