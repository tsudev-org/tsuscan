//! Xác nhận bản vá libsqlite3-sys thật sự biên dịch SQLite3MC (ChaCha20-Poly1305) và FTS5.
#![allow(clippy::unwrap_used)]
use rusqlite::Connection;

#[test]
fn encrypted_db_with_fts5() {
    let dir = std::env::temp_dir().join(format!("tsuscan-mc-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("v.db");
    let _ = std::fs::remove_file(&path);
    {
        let c = Connection::open(&path).unwrap();
        c.pragma_update(
            None,
            "hexkey",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
        )
        .unwrap();
        let cipher: String = c.query_row("PRAGMA cipher", [], |r| r.get(0)).unwrap();
        assert_eq!(cipher, "chacha20");
        c.execute_batch("CREATE VIRTUAL TABLE t USING fts5(x); INSERT INTO t VALUES ('canhvebimatdaxanh');")
            .unwrap();
        let n: i64 = c
            .query_row(
                "SELECT count(*) FROM t WHERE t MATCH 'canhvebimatdaxanh'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }
    let raw = std::fs::read(&path).unwrap();
    assert!(!raw.windows(17).any(|w| w == b"canhvebimatdaxanh"));
    assert!(!raw.starts_with(b"SQLite format 3"));
    let c = Connection::open(&path).unwrap();
    c.pragma_update(
        None,
        "hexkey",
        "ff112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
    )
    .unwrap();
    assert!(
        c.query_row("SELECT count(*) FROM t", [], |r| r.get::<_, i64>(0))
            .is_err()
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
