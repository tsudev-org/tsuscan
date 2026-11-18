# libsqlite3-sys 0.40.0 - bản vá của Tsu Scan

Bản sao `libsqlite3-sys` 0.40.0 (MIT, rusqlite) nạp qua `[patch.crates-io]` ở `Cargo.toml` gốc. Lý do: ADR-0009.

Khác bản gốc (chỉ 3 điểm, đánh dấu `tsuscan` trong mã):

1. `Cargo.toml`: thêm build-dependency `sqlite3mc-src = "=206.1.0"` (SQLite3MultipleCiphers 2.6.1, SQLite 3.54.0).
2. `build.rs` nhánh `bundled`: biên dịch `sqlite3mc_amalgamation.c` thay cho `sqlite3/sqlite3.c` (đã xóa khỏi bản
   sao); chỉ bật ChaCha20-Poly1305, `SQLITE_TEMP_STORE=3`, `SQLITE_SECURE_DELETE=1`, `SQLITE_DQS=0`, tắt nạp extension.
3. Xóa `sqlite3/sqlite3.c`, `sqlcipher/`, `Cargo.lock`, `Cargo.toml.orig`.

Nâng rusqlite (kéo libsqlite3-sys mới): chép lại thư mục crate từ `~/.cargo/registry/src/*/libsqlite3-sys-<bản>`,
áp lại 3 điểm trên, chạy `cargo test -p tsuscan-vault --test sqlite3mc_probe` (khẳng định `PRAGMA cipher` =
`chacha20` và tệp không lộ bản rõ). Bindings dựng sẵn khớp SQLite 3.54.x; nâng sqlite3mc-src lệch bản SQLite lớn ⇒
so `SQLITE_VERSION` của hai bên.
