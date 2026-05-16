//! Cooperative cancellation for async operations.
//!
//! [`CancellationToken`] mirrors the JavaScript `AbortSignal` pattern in Rust:
//! cheap to clone, callable from any thread, and works on both native and
//! wasm32 targets. Cancellation signals propagate to every clone; awaiting
//! [`CancellationToken::cancelled`] on any clone resolves as soon as `cancel`
//! is called on any other clone.
//!
//! Designed to interop with the WASM [`AbortSignal`] bridge in [`crate::wasm`]
//! — the WASM wrapper adapts an incoming `AbortSignal` into a
//! `CancellationToken` and forwards it to the same service-level call paths.

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};

use event_listener::{Event, EventListener};

/// A cooperative cancellation primitive.
///
/// Cheap to clone (`Arc`-backed). Calling [`cancel`](Self::cancel) on any
/// clone wakes every future returned by [`cancelled`](Self::cancelled) on
/// every clone. Subsequent calls to `cancelled()` after `cancel()` resolve
/// immediately on first poll.
#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<Inner>,
}

struct Inner {
    cancelled: AtomicBool,
    event: Event,
}

impl CancellationToken {
    /// Create a new uncancelled token.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                cancelled: AtomicBool::new(false),
                event: Event::new(),
            }),
        }
    }

    /// Signal cancellation. Idempotent — subsequent calls are no-ops.
    pub fn cancel(&self) {
        // Order: flip the flag before notifying so anyone we wake observes it.
        self.inner.cancelled.store(true, Ordering::SeqCst);
        self.inner.event.notify(usize::MAX);
    }

    /// Returns whether [`cancel`](Self::cancel) has been called on this token
    /// (or any clone).
    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::SeqCst)
    }

    /// Future that resolves when this token is cancelled. Returns immediately
    /// (on the first poll) if the token is already cancelled.
    pub fn cancelled(&self) -> Cancelled<'_> {
        Cancelled {
            token: self,
            listener: None,
        }
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for CancellationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CancellationToken")
            .field("cancelled", &self.is_cancelled())
            .finish()
    }
}

/// Future returned by [`CancellationToken::cancelled`]. Resolves with `()`
/// the moment the underlying token is cancelled.
pub struct Cancelled<'a> {
    token: &'a CancellationToken,
    listener: Option<Pin<Box<EventListener>>>,
}

impl Future for Cancelled<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        // Standard event-listener wait loop:
        //   1. Check the condition. If set, return Ready.
        //   2. Register a listener.
        //   3. Re-check the condition (cover the race between (1) and (2)).
        //   4. Poll the listener. On notify, drop it and repeat from (1).
        loop {
            if self.token.is_cancelled() {
                return Poll::Ready(());
            }

            if self.listener.is_none() {
                self.listener = Some(Box::pin(self.token.inner.event.listen()));
                if self.token.is_cancelled() {
                    return Poll::Ready(());
                }
            }

            let listener = self.listener.as_mut().expect("listener was just installed");
            match listener.as_mut().poll(cx) {
                Poll::Ready(()) => {
                    // Notification arrived. Drop the listener and re-check
                    // cancellation on the next loop iteration.
                    self.listener = None;
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn new_is_uncancelled() {
        let t = CancellationToken::new();
        assert!(!t.is_cancelled());
    }

    #[test]
    fn cancel_sets_flag() {
        let t = CancellationToken::new();
        t.cancel();
        assert!(t.is_cancelled());
    }

    #[test]
    fn cancel_is_idempotent() {
        let t = CancellationToken::new();
        t.cancel();
        t.cancel();
        t.cancel();
        assert!(t.is_cancelled());
    }

    #[test]
    fn clone_shares_state() {
        let a = CancellationToken::new();
        let b = a.clone();
        assert!(!a.is_cancelled() && !b.is_cancelled());
        b.cancel();
        assert!(a.is_cancelled() && b.is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_resolves_after_cancel() {
        let t = CancellationToken::new();
        let t_for_task = t.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            t_for_task.cancel();
        });
        // Resolves once the spawned task fires cancel().
        t.cancelled().await;
        assert!(t.is_cancelled());
    }

    #[tokio::test]
    async fn cancelled_resolves_immediately_when_already_cancelled() {
        let t = CancellationToken::new();
        t.cancel();
        // Should not block.
        tokio::time::timeout(Duration::from_millis(50), t.cancelled())
            .await
            .expect("already-cancelled token should resolve immediately");
    }

    #[tokio::test]
    async fn cancelled_pending_until_cancel() {
        let t = CancellationToken::new();
        // Without cancel, the future should NOT resolve.
        let res = tokio::time::timeout(Duration::from_millis(30), t.cancelled()).await;
        assert!(res.is_err(), "cancelled() resolved without cancel()");
    }

    #[tokio::test]
    async fn cancel_wakes_multiple_clones() {
        let t = CancellationToken::new();
        let a = t.clone();
        let b = t.clone();
        let c = t.clone();

        let t_canceller = t.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            t_canceller.cancel();
        });

        tokio::join!(a.cancelled(), b.cancelled(), c.cancelled());
        assert!(t.is_cancelled());
    }
}
