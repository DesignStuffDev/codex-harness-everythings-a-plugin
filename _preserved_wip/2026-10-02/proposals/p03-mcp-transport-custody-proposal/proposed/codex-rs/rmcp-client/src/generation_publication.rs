//! A cancelled admission observer closes its generation, including late publication.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::PoisonError;
use tokio_util::sync::CancellationToken;

use crate::client_lifecycle::PendingShutdown;

#[derive(Default)]
struct PublicationState {
    abandoned: bool,
    generation: Option<Arc<dyn PendingShutdown>>,
}

/// One publication per construction or handshake attempt. The retained task
/// publishes its generation here after attaching it to the client roster.
#[derive(Default)]
pub(crate) struct GenerationPublication {
    state: Mutex<PublicationState>,
    cancelled: CancellationToken,
}

impl GenerationPublication {
    pub(crate) fn new() -> (Arc<Self>, GenerationAdmissionGuard) {
        let publication = Arc::new(Self::default());
        let guard = GenerationAdmissionGuard(Some(Arc::clone(&publication)));
        (publication, guard)
    }

    pub(crate) fn publish(&self, generation: Arc<dyn PendingShutdown>) {
        let (abandoned, previous) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let previous = state.generation.replace(Arc::clone(&generation));
            (state.abandoned, previous)
        };
        // Neither a replaced generation's destructor nor its shutdown callback
        // may run while the admission mutex is held.
        drop(previous);
        if abandoned {
            generation.begin_shutdown();
        }
    }

    pub(crate) fn cancellation_token(&self) -> CancellationToken {
        self.cancelled.clone()
    }
}

pub(crate) struct GenerationAdmissionGuard(Option<Arc<GenerationPublication>>);

impl GenerationAdmissionGuard {
    pub(crate) fn for_generation(generation: Arc<dyn PendingShutdown>) -> Self {
        let (publication, guard) = GenerationPublication::new();
        publication.publish(generation);
        guard
    }

    pub(crate) fn disarm(mut self) {
        let _ = self.0.take();
    }
}

impl Drop for GenerationAdmissionGuard {
    fn drop(&mut self) {
        if let Some(publication) = self.0.take() {
            let generation = {
                let mut state = publication.state.lock().unwrap_or_else(PoisonError::into_inner);
                state.abandoned = true;
                state.generation.take()
            };
            // Cancellation may wake arbitrary observers; both signals run only
            // after the abandoned state is visible and the mutex is released.
            publication.cancelled.cancel();
            if let Some(generation) = generation {
                generation.begin_shutdown();
            }
        }
    }
}

#[cfg(test)]
#[path = "generation_publication_tests.rs"]
mod tests;
