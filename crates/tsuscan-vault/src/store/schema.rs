//! Lược đồ CSDL và migration. Migration đã phát hành là BẤT BIẾN: chỉ thêm bước mới, tăng [`VERSION`].

use rusqlite::Connection;

use crate::error::{Result, VaultError};

/// Phiên bản lược đồ hiện tại (`PRAGMA user_version`).
pub const VERSION: i64 = 1;

const V1: &str = r"
CREATE TABLE meta (
    k TEXT PRIMARY KEY NOT NULL,
    v BLOB NOT NULL
) STRICT;

CREATE TABLE folders (
    id TEXT PRIMARY KEY NOT NULL,
    parent_id TEXT REFERENCES folders(id),
    name TEXT NOT NULL,
    hlc TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE documents (
    id TEXT PRIMARY KEY NOT NULL,
    folder_id TEXT REFERENCES folders(id),
    title TEXT NOT NULL,
    created_ms INTEGER NOT NULL,
    updated_ms INTEGER NOT NULL,
    wrapped_dk BLOB NOT NULL,
    hlc TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX documents_folder ON documents(folder_id, deleted, updated_ms);

CREATE TABLE files (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL REFERENCES documents(id),
    wrapped_fk BLOB NOT NULL,
    plain_len INTEGER NOT NULL,
    cipher_len INTEGER NOT NULL
) STRICT;
CREATE INDEX files_document ON files(document_id);

CREATE TABLE pages (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL REFERENCES documents(id),
    position INTEGER NOT NULL,
    file_id TEXT REFERENCES files(id),
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    dpi INTEGER NOT NULL,
    ocr_text TEXT NOT NULL DEFAULT '',
    ocr_blob BLOB,
    hlc TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
) STRICT;
CREATE INDEX pages_document ON pages(document_id, deleted, position);

CREATE TABLE tags (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hlc TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE document_tags (
    document_id TEXT NOT NULL REFERENCES documents(id),
    tag_id TEXT NOT NULL REFERENCES tags(id),
    hlc TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (document_id, tag_id)
) STRICT;
CREATE INDEX document_tags_tag ON document_tags(tag_id, deleted);

-- Chỉ mục tìm kiếm, một dòng mỗi tài liệu (rowid = documents.rowid). Cột *_norm: không dấu (để khớp), cột *_acc:
-- có dấu chữ thường (để xếp hạng khớp đúng dấu cao hơn). Chuẩn hóa sẵn bằng crate::normalize lúc ghi.
CREATE VIRTUAL TABLE search USING fts5(
    title_norm, body_norm, title_acc, body_acc,
    tokenize = 'unicode61 remove_diacritics 0'
);
";

/// Đưa lược đồ lên [`VERSION`].
///
/// # Errors
/// [`VaultError::Format`] khi kho do bản app mới hơn tạo; lỗi SQLite.
pub fn migrate(conn: &mut Connection) -> Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current > VERSION {
        return Err(VaultError::Format("kho do phiên bản app mới hơn tạo"));
    }
    if current < 1 {
        let tx = conn.transaction()?;
        tx.execute_batch(V1)?;
        tx.pragma_update(None, "user_version", 1)?;
        tx.commit()?;
    }
    Ok(())
}
