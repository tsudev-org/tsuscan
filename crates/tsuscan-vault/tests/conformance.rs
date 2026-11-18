//! Vector đối chiếu `conformance/vault-v1.json`: giải mã đúng VÀ mã hóa lại ra đúng từng byte (phát hiện lệch định
//! dạng). Sinh lại: `cargo run -p tsuscan-vault --example gen_conformance > conformance/vault-v1.json`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::Value;
use tsuscan_vault::envelope::{self, Kind};
use tsuscan_vault::keys::{self, KdfParams, RecoveryPhrase};
use tsuscan_vault::{Key32, stream};

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn load() -> Value {
    let p = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/vault-v1.json");
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v[k].as_str().unwrap()
}

#[test]
fn envelopes_open_and_reseal_byte_exact() {
    let v = load();
    let key = Key32::from_slice(&unhex(s(&v, "keyHex"))).unwrap();
    let envs = v["envelopes"].as_array().unwrap();
    assert_eq!(envs.len(), 5);
    for e in envs {
        let kind = if e["kind"] == 1 { Kind::Record } else { Kind::Key };
        let sealed = unhex(s(e, "sealedHex"));
        let pt = unhex(s(e, "plaintextHex"));
        let ctx = s(e, "context").as_bytes();
        assert_eq!(
            *envelope::open(&key, kind, ctx, &sealed).unwrap(),
            pt,
            "{}",
            e["name"]
        );
        let nonce: [u8; 24] = unhex(s(e, "nonceHex")).try_into().unwrap();
        assert_eq!(
            envelope::seal_with_nonce(&key, kind, ctx, &pt, &nonce).unwrap(),
            sealed,
            "{}",
            e["name"]
        );
    }
}

#[test]
fn streams_decrypt_and_reencrypt_byte_exact() {
    let v = load();
    let key = Key32::from_slice(&unhex(s(&v, "keyHex"))).unwrap();
    let prefix: [u8; stream::PREFIX_LEN] = unhex(s(&v["stream"], "prefixHex")).try_into().unwrap();
    for st in v["stream"]["streams"].as_array().unwrap() {
        let n = usize::try_from(st["plaintextLen"].as_u64().unwrap()).unwrap();
        let pt: Vec<u8> = (0..n).map(|i| ((i * 13 + 7) % 256) as u8).collect();
        let ct = unhex(s(st, "ciphertextHex"));
        let ctx = s(st, "context").as_bytes();
        let mut out = Vec::new();
        assert_eq!(stream::decrypt(&key, ctx, &ct[..], &mut out).unwrap(), n as u64);
        assert_eq!(out, pt);
        let mut again = Vec::new();
        stream::encrypt_with_prefix(&key, ctx, n as u64, &pt[..], &mut again, &prefix).unwrap();
        assert_eq!(again, ct, "{}", st["name"]);
    }
}

#[test]
fn argon2id_and_recovery_kdf() {
    let v = load();
    let a = &v["argon2id"];
    let params = KdfParams {
        m_kib: u32::try_from(a["mKib"].as_u64().unwrap()).unwrap(),
        t: u32::try_from(a["t"].as_u64().unwrap()).unwrap(),
        p: u32::try_from(a["p"].as_u64().unwrap()).unwrap(),
    };
    let salt: [u8; 16] = unhex(s(a, "saltHex")).try_into().unwrap();
    let kek = keys::password_kek(s(a, "passwordNfd"), &salt, params).unwrap();
    assert_eq!(kek.as_bytes().to_vec(), unhex(s(a, "keyHex")));

    let r = &v["recovery"];
    let phrase = RecoveryPhrase::parse(s(r, "words")).unwrap();
    assert_eq!(phrase.kek().unwrap().as_bytes().to_vec(), unhex(s(r, "keyHex")));
    let entropy: [u8; 32] = unhex(s(r, "entropyHex")).try_into().unwrap();
    assert_eq!(
        RecoveryPhrase::from_entropy(&entropy).unwrap().as_str(),
        s(r, "words")
    );
}
