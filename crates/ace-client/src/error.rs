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

    /// Received data exceeded the bounded buffer it was being assembled into.
    FrameTooLong,

    /// The periodic-DID subscription table is full.
    ///
    /// Reported rather than dropped: a caller that subscribes and is silently ignored
    /// waits indefinitely for data the client will never classify as periodic.
    TooManySubscriptions,

    /// The observed-event queue is full, so an event could not be recorded.
    ///
    /// Reported rather than dropped: the caller learns what happened only by draining
    /// this queue, so a discarded event is indistinguishable to them from the exchange
    /// never having occurred.
    EventQueueFull,
}

// endregion: Client Error
