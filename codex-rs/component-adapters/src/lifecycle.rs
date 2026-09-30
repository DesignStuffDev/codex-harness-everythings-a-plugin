use codex_component_host::ComponentBinding;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ThreadIdleCause;
use codex_extension_api::ThreadIdleInput;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadReadyInput;
use codex_extension_api::ThreadResumeInput;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ThreadStopInput;
use codex_extension_api::TurnAbortInput;
use codex_extension_api::TurnErrorInput;
use codex_extension_api::TurnLifecycleContributor;
use codex_extension_api::TurnStartInput;
use codex_extension_api::TurnStartPhase;
use codex_extension_api::TurnStopInput;
use serde_json::Value;
use serde_json::json;

pub(crate) struct ComponentLifecycle(pub(crate) ComponentBinding);

impl ComponentLifecycle {
    fn event<'a>(
        &'a self,
        event: &'static str,
        session: &ExtensionData,
        thread: &ExtensionData,
        turn_id: Option<&str>,
        details: Value,
    ) -> ExtensionFuture<'a, ()> {
        let params = json!({
            "event": event,
            "session_id": session.level_id(),
            "thread_id": thread.level_id(),
            "turn_id": turn_id,
            "details": details,
        });
        Box::pin(async move {
            if let Err(error) = self.0.call("event", params).await {
                // Observers do not own the host state transition. A failed observer must not
                // prevent the host from flushing its thread or reporting a completed turn.
                tracing::warn!(plugin_id = %self.0.plugin_id, event, %error, "component lifecycle callback failed");
            }
        })
    }
}

impl<C: Sync> ThreadLifecycleContributor<C> for ComponentLifecycle {
    fn on_thread_start<'a>(&'a self, input: ThreadStartInput<'a, C>) -> ExtensionFuture<'a, ()> {
        self.event(
            "thread/start",
            input.session_store,
            input.thread_store,
            /*turn_id*/ None,
            json!({"persistent_state_available": input.persistent_thread_state_available}),
        )
    }

    fn on_thread_ready<'a>(&'a self, input: ThreadReadyInput<'a, C>) -> ExtensionFuture<'a, ()> {
        self.event(
            "thread/ready",
            input.session_store,
            input.thread_store,
            /*turn_id*/ None,
            json!({}),
        )
    }

    fn on_thread_resume<'a>(&'a self, input: ThreadResumeInput<'a>) -> ExtensionFuture<'a, ()> {
        self.event(
            "thread/resume",
            input.session_store,
            input.thread_store,
            /*turn_id*/ None,
            json!({}),
        )
    }

    fn on_thread_idle<'a>(&'a self, input: ThreadIdleInput<'a>) -> ExtensionFuture<'a, ()> {
        let cause = match input.cause {
            ThreadIdleCause::Completed => "completed",
            ThreadIdleCause::Interrupted => "interrupted",
            ThreadIdleCause::Failed => "failed",
        };
        self.event(
            "thread/idle",
            input.session_store,
            input.thread_store,
            /*turn_id*/ None,
            json!({"cause": cause}),
        )
    }

    fn on_thread_stop<'a>(&'a self, input: ThreadStopInput<'a>) -> ExtensionFuture<'a, ()> {
        self.event(
            "thread/stop",
            input.session_store,
            input.thread_store,
            /*turn_id*/ None,
            json!({}),
        )
    }
}

impl TurnLifecycleContributor for ComponentLifecycle {
    fn turn_start_phase(&self, _thread_store: &ExtensionData) -> TurnStartPhase {
        // An external process can block: run inside the cancellable task, after registration.
        TurnStartPhase::RegularTaskStart
    }

    fn on_turn_start<'a>(&'a self, input: TurnStartInput<'a>) -> ExtensionFuture<'a, ()> {
        self.event(
            "turn/start",
            input.session_store,
            input.thread_store,
            Some(input.turn_id),
            json!({}),
        )
    }

    fn on_turn_stop<'a>(&'a self, input: TurnStopInput<'a>) -> ExtensionFuture<'a, ()> {
        self.event(
            "turn/stop",
            input.session_store,
            input.thread_store,
            Some(input.turn_store.level_id()),
            json!({}),
        )
    }

    fn on_turn_abort<'a>(&'a self, input: TurnAbortInput<'a>) -> ExtensionFuture<'a, ()> {
        self.event(
            "turn/abort",
            input.session_store,
            input.thread_store,
            Some(input.turn_store.level_id()),
            json!({"reason": input.reason}),
        )
    }

    fn on_turn_error<'a>(&'a self, input: TurnErrorInput<'a>) -> ExtensionFuture<'a, ()> {
        // Deliberately omit error_details, which can contain backend metadata and credentials.
        self.event(
            "turn/error",
            input.session_store,
            input.thread_store,
            Some(input.turn_id),
            json!({"category": input.error}),
        )
    }
}
