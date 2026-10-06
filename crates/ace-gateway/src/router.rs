// region: Imports

use ace_core::Vec;

// endregion: Imports

// region: PendingRoute

/// A forwarded request awaiting a CAN response.
///
/// When the gateway forwards UDS bytes onto the CAN bus it records the originating tester address
/// and DoIP source/target addresses. When the ISO-TP node delivers a response, the gateway looks
/// up the pending route to know where to send the DoIP response.
#[derive(Debug, Clone)]
pub struct PendingRoute {
    /// DoIP logical address of the tester that sent the request.
    pub tester_address: u16,

    /// DoIP source address from the original DiagnosticMessage.
    pub doip_source: u16,

    /// DoIP target address from the original DiagnosticMessage.
    pub doip_target: u16,

    /// CAN response ID the gateway expects the ECU to respond on.
    pub response_can_id: u32,
}

// endregion: PendingRoute

// region: PendingRouteTable

/// Tracks in-flight forwarded requests awaiting CAN responses.
///
/// `N` - max concurrent pending routes (matches max tester connections).
#[derive(Debug)]
pub struct PendingRouteTable<const MAX_PENDING: usize> {
    entries: Vec<PendingRoute, MAX_PENDING>,
}

impl<const MAX_PENDING: usize> Default for PendingRouteTable<MAX_PENDING> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const MAX_PENDING: usize> PendingRouteTable<MAX_PENDING> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Total routes this table can hold.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        MAX_PENDING
    }

    /// How many further routes can be recorded.
    ///
    /// Needed to decide a fan-out before performing it: a functionally addressed request
    /// needs one route per responding ECU, and committing some of them and then running
    /// out leaves responses that can never be matched back to a tester.
    #[must_use]
    pub fn remaining(&self) -> usize {
        MAX_PENDING.saturating_sub(self.entries.len())
    }

    /// Records a forwarded request.
    pub fn insert(&mut self, route: PendingRoute) -> bool {
        if self.entries.len() >= MAX_PENDING {
            return false;
        }

        // Silently ignoring this would leave the caller believing it is
        // configured. The entry is absent at runtime instead, so the assertion
        // catches the over-declaration in a debug build.
        debug_assert!(
            !self.entries.is_full(),
            "more entries configured than MAX_ROUTES allows"
        );
        let _ = self.entries.push(route);
        true
    }

    /// Finds and removes a pending route by CAN response ID. Called when a CAN response arrives.
    pub fn take_by_can_response_id(&mut self, can_id: u32) -> Option<PendingRoute> {
        if let Some(pos) = self
            .entries
            .iter()
            .position(|r| r.response_can_id == can_id)
        {
            Some(self.entries.remove(pos))
        } else {
            None
        }
    }

    /// Removes al pending routes for a given tester address. Called when a tester disconnects or
    /// its activation is dropped.
    pub fn remove_tester(&mut self, tester_address: u16) {
        self.entries.retain(|r| r.tester_address != tester_address);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

// endregion: PendingRouteTable
