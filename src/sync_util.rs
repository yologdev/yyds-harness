//! Shared synchronisation helpers — lock-recovery for poisoned mutexes.
//!
//! When a thread panics while holding a `Mutex` the lock becomes "poisoned".
//! Rather than cascading the panic to every subsequent caller we recover the
//! inner data — the data itself is still valid, only the invariant *might* be
//! broken, and for our use-cases (counters, output buffers, session state) that
//! is acceptable.
//!
//! Extracted on Day 58 to deduplicate identical 1-line helpers that lived in
//! `commands_bg`, `commands_spawn`, and `session`.

use std::sync::{Mutex, MutexGuard};

/// Acquire a [`Mutex`] guard, recovering from a poisoned mutex instead of
/// panicking.
///
/// # Examples
///
/// ```ignore
/// let mutex = std::sync::Mutex::new(42);
/// let guard = lock_or_recover(&mutex);
/// assert_eq!(*guard, 42);
/// ```
pub fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_lock_or_recover_normal() {
        let mutex = Mutex::new(42);
        let guard = lock_or_recover(&mutex);
        assert_eq!(*guard, 42);
    }

    #[test]
    fn test_lock_or_recover_poisoned() {
        let mutex = Arc::new(Mutex::new(vec![1, 2, 3]));
        let m2 = Arc::clone(&mutex);

        // Poison the mutex by panicking while holding the lock
        let _ = std::thread::spawn(move || {
            let _guard = m2.lock().unwrap();
            panic!("intentional panic to poison mutex");
        })
        .join();

        // The mutex is now poisoned — .lock().unwrap() would panic here
        assert!(mutex.lock().is_err(), "mutex should be poisoned");

        // lock_or_recover should still give us the data
        let guard = lock_or_recover(&mutex);
        assert_eq!(*guard, vec![1, 2, 3]);
    }
}
