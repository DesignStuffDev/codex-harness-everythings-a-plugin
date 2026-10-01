use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::pattern::{CaseMatching, Normalization};
use crate::{Config, Nucleo};

#[test]
fn supplied_scoped_pool_runs_native_matching_and_terminates() {
    thread::scope(|scope| {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(2)
            .spawn_handler(|worker| {
                thread::Builder::new().spawn_scoped(scope, move || worker.run())?;
                Ok(())
            })
            .build()
            .unwrap();
        let mut nucleo = Nucleo::new_with_thread_pool(
            Config::DEFAULT,
            Arc::new(|| {}),
            pool,
            /*columns*/ 1,
        );
        let injector = nucleo.injector();
        let mut expected: Vec<_> = (0..1_024).map(|index| format!("file-{index}.rs")).collect();
        for name in &expected {
            injector.push(name.clone(), |name, columns| columns[0] = name.as_str().into());
        }
        injector.push("other.txt".to_owned(), |name, columns| {
            columns[0] = name.as_str().into();
        });
        drop(injector);
        nucleo.pattern.reparse(
            /*column*/ 0,
            "file",
            CaseMatching::Respect,
            Normalization::Never,
            /*append*/ false,
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        while nucleo.tick(/*timeout*/ 10).running {
            assert!(Instant::now() < deadline, "native matching did not finish");
        }
        let mut actual: Vec<_> = nucleo
            .snapshot()
            .matched_items(..)
            .map(|item| item.data.clone())
            .collect();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected);
        nucleo.shutdown();
        // The scope joins the OS workers after shutdown drops their pool.
    });
}

#[test]
fn shutdown_waits_for_a_running_matcher_notification() {
    thread::scope(|scope| {
        let (start_worker, worker_ready) = mpsc::channel();
        let worker_ready = Mutex::new(worker_ready);
        let (notification_started, started) = mpsc::channel();
        let (finish_notification, finish) = mpsc::channel();
        let finish = Mutex::new(finish);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .start_handler(move |_| {
                let _ = worker_ready.lock().recv();
            })
            .spawn_handler(|worker| {
                thread::Builder::new().spawn_scoped(scope, move || worker.run())?;
                Ok(())
            })
            .build()
            .unwrap();
        let mut nucleo = Nucleo::new_with_thread_pool(
            Config::DEFAULT,
            Arc::new(move || {
                if rayon::current_thread_index().is_some() {
                    let _ = notification_started.send(());
                    let _ = finish.lock().recv();
                }
            }),
            pool,
            /*columns*/ 1,
        );
        let injector = nucleo.injector();
        injector.push((), |_, columns| columns[0] = "item".into());
        drop(injector);
        // The worker is still gated, so tick must request its notification.
        assert!(nucleo.tick(/*timeout*/ 0).running);
        start_worker.send(()).unwrap();
        started.recv_timeout(Duration::from_secs(5)).unwrap();

        let (closing, close_started) = mpsc::channel();
        let (closed, close_finished) = mpsc::channel();
        let owner = scope.spawn(move || {
            closing.send(()).unwrap();
            nucleo.shutdown();
            closed.send(()).unwrap();
        });
        close_started.recv_timeout(Duration::from_secs(5)).unwrap();
        let early_result = close_finished.recv_timeout(Duration::from_millis(50));
        // Release the callback before assertions so a failure cannot strand it.
        finish_notification.send(()).unwrap();
        owner.join().unwrap();
        assert_eq!(early_result, Err(mpsc::RecvTimeoutError::Timeout));
        assert_eq!(close_finished.recv_timeout(Duration::from_secs(5)), Ok(()));
    });
}
