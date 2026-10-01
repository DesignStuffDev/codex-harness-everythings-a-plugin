use super::connection_handling_websocket::DEFAULT_READ_TIMEOUT;
use super::connection_handling_websocket::WsClient;
use super::connection_handling_websocket::connect_websocket;
use super::connection_handling_websocket::create_config_toml;
use super::connection_handling_websocket::read_jsonrpc_message;
use super::connection_handling_websocket::read_response_for_id;
use super::connection_handling_websocket::send_request;
use super::connection_handling_websocket::spawn_websocket_server;
use anyhow::Result;
use anyhow::bail;
use app_test_support::create_mock_responses_server_sequence_unchecked;
use codex_app_server_protocol::FuzzyFileSearchSessionUpdatedNotification;
use codex_app_server_protocol::JSONRPCMessage;
use codex_app_server_protocol::RequestId;
use pretty_assertions::assert_eq;
use serde_json::json;
use tempfile::TempDir;
use tokio::time::Duration;
use tokio::time::timeout;

const SESSION_ID: &str = "same-session-on-both-connections";

#[tokio::test]
async fn websocket_search_sessions_are_private_and_survive_another_connection_closing() -> Result<()>
{
    let provider = create_mock_responses_server_sequence_unchecked(Vec::new()).await;
    let home = TempDir::new()?;
    create_config_toml(home.path(), &provider.uri(), "never")?;
    let first_root = TempDir::new()?;
    let second_root = TempDir::new()?;
    std::fs::write(first_root.path().join("alpha-first.txt"), "first")?;
    std::fs::write(second_root.path().join("alpha-second.txt"), "second")?;
    let (mut process, address) = spawn_websocket_server(home.path()).await?;
    let mut first = connect_websocket(address).await?;
    let mut second = connect_websocket(address).await?;
    for (client, root) in [
        (&mut first, first_root.path()),
        (&mut second, second_root.path()),
    ] {
        send_request(
            client,
            "initialize",
            1,
            Some(json!({
                "clientInfo": {"name": "search-connection-test", "version": "0.1.0"},
                "capabilities": {"experimentalApi": true}
            })),
        )
        .await?;
        read_response_for_id(client, 1).await?;
        send_request(
            client,
            "fuzzyFileSearch/sessionStart",
            2,
            Some(json!({
                "sessionId": SESSION_ID, "roots": [root]
            })),
        )
        .await?;
        // Session startup has its own empty-query snapshot/completion cycle.
        // Consume it with the response so it cannot be mistaken for the first
        // typed query or for a notification leaked from the other connection.
        read_search_cycle(client, 2, "", &[]).await?;
    }

    query_and_wait(&mut first, 3, "alpha", "alpha-first.txt").await?;
    assert_no_search_notifications(&mut second).await?;
    query_and_wait(&mut second, 3, "alpha", "alpha-second.txt").await?;
    assert_no_search_notifications(&mut first).await?;

    first.close(None).await?;
    query_and_wait(&mut second, 4, "second", "alpha-second.txt").await?;
    send_request(
        &mut second,
        "fuzzyFileSearch/sessionStop",
        5,
        Some(json!({
            "sessionId": SESSION_ID
        })),
    )
    .await?;
    read_response_for_id(&mut second, 5).await?;
    // Every earlier frame has been consumed up through the stop acknowledgement.
    // There is no grace period in which a post-stop search update is accepted.
    assert_no_search_notifications(&mut second).await?;
    second.close(None).await?;
    // This fixture checks connection shutdown and joined session stop. Whole
    // server graceful shutdown is covered by the separate lifecycle fixtures.
    process.kill().await?;
    process.wait().await?;
    Ok(())
}

async fn query_and_wait(
    client: &mut WsClient,
    id: i64,
    query: &str,
    expected_file: &str,
) -> Result<()> {
    send_request(
        client,
        "fuzzyFileSearch/sessionUpdate",
        id,
        Some(json!({
            "sessionId": SESSION_ID, "query": query
        })),
    )
    .await?;
    read_search_cycle(client, id, query, &[expected_file]).await
}

async fn read_search_cycle(
    client: &mut WsClient,
    id: i64,
    query: &str,
    expected_files: &[&str],
) -> Result<()> {
    timeout(DEFAULT_READ_TIMEOUT, async {
        let mut response = false;
        let mut completed = false;
        let mut snapshot_received = false;
        let mut files = Vec::new();
        while !response || !completed {
            match read_jsonrpc_message(client).await? {
                JSONRPCMessage::Response(value) if value.id == RequestId::Integer(id) => {
                    response = true
                }
                JSONRPCMessage::Error(error) => bail!("unexpected RPC error: {error:?}"),
                JSONRPCMessage::Notification(value)
                    if value.method == "fuzzyFileSearch/sessionUpdated" =>
                {
                    let update: FuzzyFileSearchSessionUpdatedNotification =
                        serde_json::from_value(value.params.unwrap_or_default())?;
                    assert_eq!(
                        (update.session_id.as_str(), update.query.as_str()),
                        (SESSION_ID, query)
                    );
                    files = update.files.into_iter().map(|file| file.path).collect();
                    snapshot_received = true;
                }
                JSONRPCMessage::Notification(value)
                    if value.method == "fuzzyFileSearch/sessionCompleted" =>
                {
                    assert_eq!(value.params, Some(json!({"sessionId": SESSION_ID})));
                    assert!(
                        snapshot_received,
                        "completion must follow its query snapshot"
                    );
                    completed = true;
                }
                _ => {}
            }
        }
        assert_eq!(
            files.iter().map(String::as_str).collect::<Vec<_>>(),
            expected_files
        );
        Ok(())
    })
    .await?
}

async fn assert_no_search_notifications(client: &mut WsClient) -> Result<()> {
    let result = timeout(Duration::from_millis(250), async {
        loop {
            match read_jsonrpc_message(client).await? {
                JSONRPCMessage::Notification(value)
                    if value.method.starts_with("fuzzyFileSearch/") =>
                {
                    bail!("unexpected search notification: {value:?}");
                }
                JSONRPCMessage::Error(error) => bail!("unexpected RPC error: {error:?}"),
                _ => {}
            }
        }
    })
    .await;
    match result {
        Err(_) => Ok(()),
        Ok(result) => result,
    }
}
