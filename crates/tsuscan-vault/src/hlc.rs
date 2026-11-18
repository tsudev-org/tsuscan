//! Đồng hồ logic lai (HLC, Kulkarni và cộng sự 2014) cho hợp nhất theo trường, ghi sau thắng (ADR-0009 mục 3.5).

use std::cmp::Ordering;

use crate::error::{Result, VaultError};

/// Lệch tương lai tối đa chấp nhận từ thiết bị khác (5 phút).
pub const MAX_DRIFT_MS: u64 = 5 * 60 * 1000;
const MAX_MILLIS: u64 = (1 << 48) - 1;

/// Dấu thời gian HLC. Thứ tự: mili giây, bộ đếm, rồi id thiết bị (phá hòa xác định).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hlc {
    /// Mili giây Unix (48 bit).
    pub millis: u64,
    /// Bộ đếm trong cùng mili giây.
    pub counter: u16,
    /// Id thiết bị.
    pub node: u64,
}

impl Ord for Hlc {
    fn cmp(&self, o: &Self) -> Ordering {
        (self.millis, self.counter, self.node).cmp(&(o.millis, o.counter, o.node))
    }
}

impl PartialOrd for Hlc {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

impl Hlc {
    /// Dạng chuỗi 32 ký tự hex, so sánh chuỗi cùng thứ tự với so sánh giá trị (dùng làm cột SQLite).
    #[must_use]
    pub fn encode(&self) -> String {
        format!("{:012x}{:04x}{:016x}", self.millis, self.counter, self.node)
    }

    /// Đọc dạng chuỗi của [`Hlc::encode`].
    ///
    /// # Errors
    /// [`VaultError::Format`] khi sai độ dài hoặc không phải hex thường.
    pub fn decode(s: &str) -> Result<Self> {
        let ok = s.len() == 32
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
        if !ok {
            return Err(VaultError::Format("HLC sai định dạng"));
        }
        let p =
            |r: std::ops::Range<usize>| u64::from_str_radix(&s[r], 16).map_err(|_| VaultError::Format("HLC"));
        Ok(Self {
            millis: p(0..12)?,
            counter: p(12..16)? as u16,
            node: p(16..32)?,
        })
    }
}

/// Đồng hồ HLC của một thiết bị.
#[derive(Debug, Clone)]
pub struct Clock {
    node: u64,
    millis: u64,
    counter: u16,
}

impl Clock {
    /// Đồng hồ mới cho thiết bị `node`, tiếp nối dấu lớn nhất đã lưu (nếu có) để không lùi sau khi mở lại app.
    #[must_use]
    pub fn new(node: u64, last: Option<Hlc>) -> Self {
        let (millis, counter) = last.map_or((0, 0), |h| (h.millis, h.counter));
        Self {
            node,
            millis,
            counter,
        }
    }

    /// Dấu cho một lần ghi cục bộ tại giờ hệ thống `wall_ms`.
    ///
    /// # Errors
    /// [`VaultError::ClockDrift`] khi bộ đếm tràn trong cùng mili giây ở giới hạn 48 bit (không xảy ra thực tế).
    pub fn tick(&mut self, wall_ms: u64) -> Result<Hlc> {
        let wall = wall_ms.min(MAX_MILLIS);
        if wall > self.millis {
            self.millis = wall;
            self.counter = 0;
        } else {
            self.bump()?;
        }
        Ok(self.current())
    }

    /// Nhận dấu `remote` từ thiết bị khác (khi kéo bản ghi về) để các lần ghi sau luôn lớn hơn.
    ///
    /// # Errors
    /// [`VaultError::ClockDrift`] khi `remote` vượt giờ hệ thống quá [`MAX_DRIFT_MS`]: không kéo đồng hồ theo.
    pub fn observe(&mut self, remote: Hlc, wall_ms: u64) -> Result<()> {
        if remote.millis > wall_ms.saturating_add(MAX_DRIFT_MS) {
            return Err(VaultError::ClockDrift);
        }
        let wall = wall_ms.min(MAX_MILLIS);
        let m = self.millis.max(remote.millis).max(wall);
        let c = match (m == self.millis, m == remote.millis) {
            (true, true) => self.counter.max(remote.counter),
            (true, false) => self.counter,
            (false, true) => remote.counter,
            (false, false) => {
                self.millis = m;
                self.counter = 0;
                return Ok(());
            }
        };
        self.millis = m;
        self.counter = c;
        Ok(())
    }

    fn bump(&mut self) -> Result<()> {
        if let Some(c) = self.counter.checked_add(1) {
            self.counter = c;
        } else if self.millis < MAX_MILLIS {
            self.millis += 1;
            self.counter = 0;
        } else {
            return Err(VaultError::ClockDrift);
        }
        Ok(())
    }

    fn current(&self) -> Hlc {
        Hlc {
            millis: self.millis,
            counter: self.counter,
            node: self.node,
        }
    }
}

/// Giờ hệ thống tính bằng mili giây Unix (0 nếu đồng hồ trước 1970).
#[must_use]
pub fn wall_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_even_when_wall_clock_goes_back() {
        let mut c = Clock::new(1, None);
        let a = c.tick(1000).unwrap_or_else(|_| unreachable!());
        let b = c.tick(1000).unwrap_or_else(|_| unreachable!());
        let d = c.tick(500).unwrap_or_else(|_| unreachable!());
        let e = c.tick(2000).unwrap_or_else(|_| unreachable!());
        assert!(a < b && b < d && d < e);
        assert_eq!((d.millis, d.counter), (1000, 2));
        assert_eq!((e.millis, e.counter), (2000, 0));
    }

    #[test]
    fn observe_moves_past_remote_and_rejects_far_future() {
        let mut c = Clock::new(1, None);
        let remote = Hlc {
            millis: 5000,
            counter: 7,
            node: 2,
        };
        assert!(c.observe(remote, 4000).is_ok());
        let next = c.tick(4000).unwrap_or_else(|_| unreachable!());
        assert!(next > remote);
        let far = Hlc {
            millis: 4000 + MAX_DRIFT_MS + 1,
            counter: 0,
            node: 2,
        };
        assert!(matches!(c.observe(far, 4000), Err(VaultError::ClockDrift)));
        assert!(c.tick(4000).unwrap_or_else(|_| unreachable!()) < far);
    }

    #[test]
    fn counter_overflow_rolls_millis() {
        let mut c = Clock::new(
            1,
            Some(Hlc {
                millis: 10,
                counter: u16::MAX,
                node: 1,
            }),
        );
        let h = c.tick(10).unwrap_or_else(|_| unreachable!());
        assert_eq!((h.millis, h.counter), (11, 0));
    }

    #[test]
    fn encode_sorts_like_values_and_roundtrips() {
        let hs = [
            Hlc {
                millis: 1,
                counter: 0,
                node: 9,
            },
            Hlc {
                millis: 1,
                counter: 1,
                node: 0,
            },
            Hlc {
                millis: 2,
                counter: 0,
                node: 0,
            },
            Hlc {
                millis: 2,
                counter: 0,
                node: 1,
            },
        ];
        for w in hs.windows(2) {
            assert!(w[0] < w[1] && w[0].encode() < w[1].encode());
        }
        for h in hs {
            assert_eq!(Hlc::decode(&h.encode()).ok(), Some(h));
        }
        assert!(Hlc::decode("ABC").is_err());
        assert!(Hlc::decode(&"G".repeat(32)).is_err());
    }
}
