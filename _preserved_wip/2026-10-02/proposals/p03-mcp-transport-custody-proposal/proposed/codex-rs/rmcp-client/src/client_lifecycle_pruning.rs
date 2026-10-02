//! Opportunistic exact observations bound successful generation accumulation.

use super::*;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

const SWEEP_LIMIT: usize = 32;
static CLIENT_SWEEP_CURSOR: AtomicUsize = AtomicUsize::new(0);

impl ClientLifecycle {
    /// An idle client may have been dropped without an observer. Only poll its
    /// already-published completion; never initiate shutdown of an open client.
    pub(crate) async fn observe_completed_clients() {
        let clients = {
            let clients = CLIENTS.get_or_init(Default::default).lock()
                .unwrap_or_else(PoisonError::into_inner);
            snapshot(&clients, CLIENT_SWEEP_CURSOR.fetch_add(SWEEP_LIMIT, Ordering::Relaxed))
        };
        for client in clients {
            let completion = client.state.lock().unwrap_or_else(PoisonError::into_inner)
                .completion.clone();
            if let Some(completion) = completion
                && let Ok(Ok(outcome)) = tokio::time::timeout(Duration::ZERO, completion.wait()).await
                && *outcome == Ok(McpShutdownConfirmation::Confirmed)
            {
                client.retire();
            }
        }
    }

    /// Poll at most 32 exact owners per roster. Pending, failed and unconfirmed generations
    /// remain in the roster. Snapshots and retired resources drop outside locks.
    pub(crate) async fn prune_clean(&self) {
        let (services, processes, transports, jobs, pending) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let cursor = state.prune_cursor;
            state.prune_cursor = cursor.wrapping_add(SWEEP_LIMIT);
            (snapshot(&state.services, cursor), snapshot(&state.processes, cursor),
                snapshot(&state.transports, cursor), snapshot(&state.jobs, cursor),
                snapshot(&state.pending, cursor))
        };
        let mut clean_services = Vec::new();
        for service in services {
            if matches!(tokio::time::timeout(Duration::ZERO, service.wait_closed()).await, Ok(Ok(()))) {
                clean_services.push(service);
            }
        }
        let mut clean_transports = Vec::new();
        for transport in transports {
            if matches!(tokio::time::timeout(Duration::ZERO, transport.wait()).await,
                Ok(Ok(TransportCloseConfirmation::Confirmed)))
            {
                clean_transports.push(transport);
            }
        }
        let mut clean_processes = Vec::new();
        for process in processes {
            if process.is_shutdown_started()
                && matches!(tokio::time::timeout(Duration::ZERO, process.wait_closed()).await, Ok(Ok(())))
            {
                clean_processes.push(process);
            }
        }
        let mut clean_jobs = Vec::new();
        for job in jobs {
            if matches!(tokio::time::timeout(Duration::ZERO, job.wait()).await, Ok(Ok(()))) {
                clean_jobs.push(job);
            }
        }
        let empty_pending: Vec<_> = pending.into_iter().filter(|pending| pending.is_empty()).collect();
        let retired = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            (
                take_matching(&mut state.services, &clean_services),
                take_matching(&mut state.processes, &clean_processes),
                take_matching(&mut state.transports, &clean_transports),
                take_matching(&mut state.jobs, &clean_jobs),
                take_matching(&mut state.pending, &empty_pending),
            )
        };
        drop(retired);
    }

    fn retire(&self) {
        let retired = {
            let mut clients = CLIENTS.get_or_init(Default::default).lock()
                .unwrap_or_else(PoisonError::into_inner);
            clients.iter().position(|client| std::ptr::eq(Arc::as_ptr(client), self))
                .map(|index| clients.swap_remove(index))
        };
        drop(retired);
    }
}

fn take_matching<T: ?Sized>(owners: &mut Vec<Arc<T>>, observed: &[Arc<T>]) -> Vec<Arc<T>> {
    let mut retired = Vec::new();
    let mut index = 0;
    while index < owners.len() {
        if observed.iter().any(|owner| Arc::ptr_eq(owner, &owners[index])) {
            retired.push(owners.swap_remove(index));
        } else {
            index += 1;
        }
    }
    retired
}

fn snapshot<T: ?Sized>(owners: &[Arc<T>], cursor: usize) -> Vec<Arc<T>> {
    if owners.is_empty() {
        return Vec::new();
    }
    owners.iter().cycle().skip(cursor % owners.len()).take(owners.len().min(SWEEP_LIMIT))
        .map(Arc::clone).collect()
}
