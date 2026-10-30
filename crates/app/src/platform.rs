//! The only impure corner: clocks, entropy and timers.

use merlo_core::Timestamp;
use std::time::Duration;

/// Local wall-clock milliseconds (UTC + local offset), so days end at midnight.
pub fn now() -> Timestamp {
    Timestamp(chrono::Local::now().naive_local().and_utc().timestamp_millis())
}

/// A seed for the drill's note picker.
pub fn seed() -> u64 {
    chrono::Utc::now().timestamp_subsec_nanos() as u64 ^ now().0 as u64
}

pub async fn sleep(ms: u64) {
    futures_timer::Delay::new(Duration::from_millis(ms)).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_is_sane() {
        let a = now();
        assert!(a.0 > 1_700_000_000_000, "after 2023");
        assert!(now() >= a);
        let _ = seed();
    }

    #[test]
    fn sleeping_returns() {
        let start = std::time::Instant::now();
        futures_executor_block_on(sleep(5));
        assert!(start.elapsed() >= Duration::from_millis(5));
    }

    /// A minimal executor, to avoid a dev-dependency for one test.
    fn futures_executor_block_on<F: std::future::Future>(f: F) -> F::Output {
        use std::sync::Arc;
        use std::task::{Context, Poll, Wake};
        struct Thread(std::thread::Thread);
        impl Wake for Thread {
            fn wake(self: Arc<Self>) {
                self.0.unpark();
            }
        }
        let waker = Arc::new(Thread(std::thread::current())).into();
        let mut cx = Context::from_waker(&waker);
        let mut f = std::pin::pin!(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
            std::thread::park();
        }
    }
}
