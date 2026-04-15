// region: Imports

// endregion: Imports

// region: Client Error

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// Request queue is full - cannot enqueue another request. With N=1 this means a request is
    /// already pending.
    QueueFull,

    /// The provided frame data is empty.
    EmptyRequest,

    /// Outbox is full - cannot enqueue outbound frame.
    OutboxFull,
}

// endregion: Client Error
