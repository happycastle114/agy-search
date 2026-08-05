use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
    time::Duration,
};

use super::*;
use crate::{error::AgyError, types::NonEmptyText};

#[derive(Debug, Default)]
struct Gate {
    current: AtomicUsize,
    maximum: AtomicUsize,
    released: AtomicBool,
    blocked: Mutex<Vec<Waker>>,
    reached: Mutex<Option<Waker>>,
}

impl Gate {
    fn enter(self: &Arc<Self>) -> Enter {
        Enter {
            gate: Arc::clone(self),
            entered: false,
        }
    }

    fn release(&self) {
        self.released.store(true, Ordering::SeqCst);
        let waiters = std::mem::take(&mut *self.blocked.lock().expect("gate lock"));
        for waiter in waiters {
            waiter.wake();
        }
    }
}

struct Enter {
    gate: Arc<Gate>,
    entered: bool,
}

impl Future for Enter {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.entered {
            let current = self.gate.current.fetch_add(1, Ordering::SeqCst) + 1;
            self.gate.maximum.fetch_max(current, Ordering::SeqCst);
            self.entered = true;
            let reached_waiter = if current == MAX_RECOVERY_CONCURRENCY {
                self.gate.reached.lock().expect("reached lock").take()
            } else {
                None
            };
            if let Some(waiter) = reached_waiter {
                waiter.wake();
            }
        }
        if self.gate.released.load(Ordering::SeqCst) {
            self.gate.current.fetch_sub(1, Ordering::SeqCst);
            Poll::Ready(())
        } else {
            self.gate
                .blocked
                .lock()
                .expect("blocked lock")
                .push(context.waker().clone());
            Poll::Pending
        }
    }
}

struct Reached(Arc<Gate>);

impl Future for Reached {
    type Output = ();

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        if self.0.maximum.load(Ordering::SeqCst) == MAX_RECOVERY_CONCURRENCY {
            Poll::Ready(())
        } else {
            *self.0.reached.lock().expect("reached lock") = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

struct CancellationCounter(Arc<AtomicUsize>);

impl Drop for CancellationCounter {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn recovery_scopes_overlap_without_exceeding_four_in_flight() {
    // Given: eight typed scopes held behind a deterministic poll barrier.
    let scopes = (0..8)
        .map(|index| {
            let text = NonEmptyText::parse(&format!("scope-{index}")).expect("valid scope");
            ScopeLabel::parse(&text).expect("bounded scope")
        })
        .collect::<Vec<_>>();
    let gate = Arc::new(Gate::default());
    let runner_gate = Arc::clone(&gate);
    let execution = tokio::spawn(async move {
        run_bounded(&scopes, move |_scope| {
            let gate = Arc::clone(&runner_gate);
            async move {
                gate.enter().await;
                Ok(())
            }
        })
        .await
    });

    // When: the first recovery wave fills its concurrency window.
    Reached(Arc::clone(&gate)).await;

    // Then: four operations overlap, no fifth starts, and all eight finish after release.
    assert_eq!(gate.current.load(Ordering::SeqCst), 4);
    assert_eq!(gate.maximum.load(Ordering::SeqCst), 4);
    gate.release();
    let completed = execution
        .await
        .expect("runner task must join")
        .expect("all scopes valid");
    assert_eq!(completed.len(), 8);
}

#[tokio::test]
async fn recovery_aborts_pending_scopes_after_the_first_terminal_error() {
    // Given: a terminal first worker and bounded sibling workers that only finish by cancellation.
    const FIRST_WORKER: usize = 0;
    let scopes = (0..MAX_RECOVERY_CONCURRENCY.saturating_mul(2))
        .map(|index| {
            let text = NonEmptyText::parse(&format!("scope-{index}")).expect("valid scope");
            ScopeLabel::parse(&text).expect("bounded scope")
        })
        .collect::<Vec<_>>();
    let scheduled = Arc::new(AtomicUsize::new(0));
    let started = Arc::new(AtomicUsize::new(0));
    let cancelled = Arc::new(AtomicUsize::new(0));

    // When: the first completed worker returns a terminal recovery failure.
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        run_bounded(&scopes, {
            let scheduled = Arc::clone(&scheduled);
            let started = Arc::clone(&started);
            let cancelled = Arc::clone(&cancelled);
            move |_scope| {
                let worker = scheduled.fetch_add(1, Ordering::SeqCst);
                let started = Arc::clone(&started);
                let cancelled = Arc::clone(&cancelled);
                async move {
                    if worker == FIRST_WORKER {
                        tokio::task::yield_now().await;
                        return Err(AgyError::OutputInvalid);
                    }
                    started.fetch_add(1, Ordering::SeqCst);
                    let _cancellation = CancellationCounter(cancelled);
                    std::future::pending::<Result<(), AgyError>>().await
                }
            }
        }),
    )
    .await;

    // Then: no later wave starts, every started sibling is cancelled, and no result escapes.
    assert!(matches!(result, Ok(Err(AgyError::OutputInvalid))));
    assert_eq!(scheduled.load(Ordering::SeqCst), MAX_RECOVERY_CONCURRENCY);
    assert_eq!(started.load(Ordering::SeqCst), MAX_RECOVERY_CONCURRENCY - 1);
    assert_eq!(
        cancelled.load(Ordering::SeqCst),
        MAX_RECOVERY_CONCURRENCY - 1
    );
}
