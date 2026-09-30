use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use pretty_assertions::assert_eq;

use super::write_staged_data;

#[test]
fn staging_cancellation_stops_between_writes() {
    struct CancellingWriter<'a> {
        cancelled: &'a AtomicBool,
        bytes_written: usize,
    }

    impl Write for CancellingWriter<'_> {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.bytes_written += bytes.len();
            self.cancelled.store(true, Ordering::Release);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            panic!("cancelled transfer must not finish flushing")
        }
    }

    let cancelled = AtomicBool::new(false);
    let mut writer = CancellingWriter {
        cancelled: &cancelled,
        bytes_written: 0,
    };
    let error = write_staged_data(&mut writer, &vec![7; 1024 * 1024], &cancelled)
        .expect_err("cancelled transfer");
    assert_eq!(
        (error.kind(), writer.bytes_written),
        (std::io::ErrorKind::Interrupted, 64 * 1024)
    );
}
