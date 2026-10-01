//! Reader-order search intent must pass auth/admission before touching controls.
use super::*;

impl MessageProcessor {
    pub(super) async fn prepare_search_admission(
        &self,
        connection_id: ConnectionId,
        session: &Arc<ConnectionSessionState>,
        request: &ClientRequest,
    ) -> Result<SearchAdmission, JSONRPCErrorError> {
        if !matches!(
            request,
            ClientRequest::FuzzyFileSearchSessionStart { .. }
                | ClientRequest::FuzzyFileSearchSessionStop { .. }
        ) {
            return Ok(SearchAdmission::Other);
        }
        let mut admitted = None;
        session
            .rpc_gate
            .run(async {
                admitted = Some(self.search_processor.admit_search_request(
                    connection_id,
                    &session.searches,
                    request,
                ));
            })
            .await;
        admitted.unwrap_or_else(|| Err(invalid_request("search connection RPC gate is closed")))
    }
}
