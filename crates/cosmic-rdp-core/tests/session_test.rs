use cosmic_rdp_core::{
    events::{SessionEvent, SessionState},
    mock::MockRdpBackend,
    session::{RdpBackend, RdpSessionHandle},
};
use cosmic_rdp_models::ConnectionProfile;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

#[tokio::test]
async fn test_mock_backend_connection_lifecycle() {
    let profile = ConnectionProfile::new("Test Machine", "127.0.0.1");
    let (cmd_tx, cmd_rx) = mpsc::channel(16);
    let (event_tx, mut event_rx) = mpsc::channel(16);
    let handle = RdpSessionHandle::new(cmd_tx);

    let runner = tokio::spawn(async move {
        let mut backend = MockRdpBackend::new();
        backend.start(profile, None, event_tx, cmd_rx).await
    });

    // 1. Should receive Connecting state
    let first_event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .expect("event received");
    match first_event {
        SessionEvent::StateChanged(SessionState::Connecting { message }) => {
            assert!(message.contains("127.0.0.1"));
        }
        other => panic!("Expected Connecting event, got {:?}", other),
    }

    // 2. Should receive Connected state
    let second_event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .expect("event received");
    assert_eq!(second_event, SessionEvent::StateChanged(SessionState::Connected));

    // 3. Should receive Resolution Changed event
    let third_event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .expect("event received");
    match third_event {
        SessionEvent::ServerResolutionChanged { width, height } => {
            assert!(width > 0);
            assert!(height > 0);
        }
        other => panic!("Expected ServerResolutionChanged, got {:?}", other),
    }

    // 4. Should receive initial FrameUpdate
    let frame_event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .expect("event received");
    match frame_event {
        SessionEvent::FrameUpdate(frame) => {
            assert!(frame.width > 0);
            assert!(frame.height > 0);
            assert_eq!(frame.data.len(), (frame.width * frame.height * 4) as usize);
        }
        other => panic!("Expected FrameUpdate, got {:?}", other),
    }

    // 5. Test Disconnect command
    handle.disconnect().await.unwrap();

    let disconnect_event = timeout(Duration::from_secs(2), event_rx.recv())
        .await
        .unwrap()
        .expect("event received");
    assert_eq!(disconnect_event, SessionEvent::StateChanged(SessionState::Disconnected));

    let _ = runner.await;
}
