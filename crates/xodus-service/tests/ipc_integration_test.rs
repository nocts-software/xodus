use std::path::PathBuf;
use prost::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use xodus::proto::xodus::{
    XGameSaveWriteBlobRequest, XGameSaveWriteBlobResponse, XodusMessage, XodusMessageType,
};
use xodus_service::simple_context::SimpleContext;


#[tokio::test]
async fn test_ipc_proto_write_blob_flow() {
    let socket_path = format!("/tmp/xodus_ipc_test_{}.sock", std::process::id());
    let _ = tokio::fs::remove_file(&socket_path).await;

    let listener = UnixListener::bind(&socket_path).expect("Failed to bind test socket");
    let sock_clone = socket_path.clone();

    // Spawn test server task
    tokio::spawn(async move {
        if let Ok((mut socket, _)) = listener.accept().await {
            let mut ctx = SimpleContext::mock();
            let _ = xodus_service::connection::proto::handle(&mut socket, &mut ctx).await;
        }
    });


    // Small delay for listener to initialize
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Connect client
    let mut client = UnixStream::connect(&socket_path)
        .await
        .expect("Failed to connect to test socket");

    // Build write blob payload
    let req_payload = XGameSaveWriteBlobRequest {
        user_id: 1,
        scid: "00000000-0000-0000-0000-000000000000".to_string(),
        container_name: "test_container".to_string(),
        blob_name: "save.bin".to_string(),
        data: b"TEST_SAVE_DATA_BLOB".to_vec(),
    };
    let payload_bytes = req_payload.encode_to_vec();

    let msg = XodusMessage {
        msg_type: XodusMessageType::XgamesaveWriteBlobRequest as i32,
        request_id: 1,
        payload: payload_bytes,
    };
    let msg_bytes = msg.encode_to_vec();

    // Send 4-byte len + pb payload
    let len_bytes = (msg_bytes.len() as u32).to_le_bytes();

    client.write_all(&len_bytes).await.unwrap();
    client.write_all(&msg_bytes).await.unwrap();
    client.flush().await.unwrap();


    // Read 4-byte response length
    let mut resp_len_buf = [0u8; 4];
    client.read_exact(&mut resp_len_buf).await.unwrap();
    let resp_len = u32::from_le_bytes(resp_len_buf) as usize;

    let mut resp_buf = vec![0u8; resp_len];
    client.read_exact(&mut resp_buf).await.unwrap();

    let resp_msg = XodusMessage::decode(&resp_buf[..]).expect("Failed to decode response XodusMessage");
    assert_eq!(resp_msg.msg_type, XodusMessageType::XgamesaveWriteBlobResponse as i32);

    let blob_resp = XGameSaveWriteBlobResponse::decode(&resp_msg.payload[..])
        .expect("Failed to decode XGameSaveWriteBlobResponse");
    assert_eq!(blob_resp.status, 0);

    // Verify blob file written to ~/.local/share/xodus/saves/
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let blob_path = PathBuf::from(home)
        .join(".local/share/xodus/saves/00000000-0000-0000-0000-000000000000/1/test_container/save.bin");

    assert!(blob_path.exists(), "Blob file should exist on disk at {:?}", blob_path);
    let read_back = tokio::fs::read(&blob_path).await.unwrap();
    assert_eq!(read_back, b"TEST_SAVE_DATA_BLOB");

    let _ = tokio::fs::remove_file(&sock_clone).await;
}
