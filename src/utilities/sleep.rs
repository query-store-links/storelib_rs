//! Cross-target async sleep helper.
//!
//! On native targets this defers to `tokio::time::sleep`. On wasm32 it uses
//! `gloo_timers::future::TimeoutFuture`, which is backed by `setTimeout`.

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
pub async fn sleep(dur: Duration) {
    tokio::time::sleep(dur).await;
}

#[cfg(target_arch = "wasm32")]
pub async fn sleep(dur: Duration) {
    let ms = dur.as_millis().min(u32::MAX as u128) as u32;
    gloo_timers::future::TimeoutFuture::new(ms).await;
}
