use crossbeam_channel::{unbounded, Receiver, Sender};
use tokio::sync::broadcast;

use crate::event::Event;

// Type alias for the sync side sender
pub type SyncSender = Sender<Event>;

// Event broadcaster that bridges sync collection to async WebSocket clients
pub struct EventBroadcaster {
    receiver: Receiver<Event>,
    tokio_broadcast: broadcast::Sender<Event>,
}

impl EventBroadcaster {
    // Create a new broadcaster with sync and async channels
    pub fn new() -> (SyncSender, Self) {
        let (sync_tx, sync_rx) = unbounded();
        let (tokio_tx, _rx) = broadcast::channel(1000);

        (
            sync_tx,
            Self {
                receiver: sync_rx,
                tokio_broadcast: tokio_tx,
            },
        )
    }

    // Run the broadcaster loop (bridges crossbeam → tokio broadcast)
    // This should be spawned in an async task
    pub async fn run(self) {
        loop {
            match self.receiver.recv() {
                Ok(event) => {
                    // Broadcast to all WebSocket subscribers
                    // Ignore send errors (happens when no subscribers)
                    let _ = self.tokio_broadcast.send(event);
                }
                Err(_) => {
                    // Channel closed, exit loop
                    break;
                }
            }
        }
    }

    // Subscribe to events (for WebSocket clients)
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.tokio_broadcast.subscribe()
    }

    // Get a clone of the broadcast sender (useful for sharing)
    pub fn get_sender(&self) -> broadcast::Sender<Event> {
        self.tokio_broadcast.clone()
    }
}

impl Clone for EventBroadcaster {
    fn clone(&self) -> Self {
        Self {
            receiver: self.receiver.clone(),
            tokio_broadcast: self.tokio_broadcast.clone(),
        }
    }
}
