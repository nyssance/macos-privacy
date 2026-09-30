//! Best-effort observation of permission changes.
//!
//! macOS publishes no supported notification for any of these permissions, so nothing here
//! subscribes to one and nothing runs on a timer. An `Observer` re-reads the permissions when the
//! caller asks — the natural moment is when the app becomes active again, since granting one of
//! these requires leaving the app — and reports only the changes it observed.

use std::collections::BTreeMap;

use crate::{Permission, Status, status};

/// Remembers what was last observed, so a re-check can report what changed.
#[derive(Clone, Debug)]
pub struct Observer {
    last: BTreeMap<Permission, Status>,
}

/// One observed change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Change {
    /// The permission that changed.
    pub permission: Permission,
    /// What it was when last observed.
    pub previous: Status,
    /// What it is now.
    pub current: Status,
}

impl Observer {
    /// Starts observing `permissions`, recording their current status as the baseline.
    pub fn new(permissions: impl IntoIterator<Item = Permission>) -> Self {
        Self { last: permissions.into_iter().map(|permission| (permission, status(permission))).collect() }
    }

    /// Observes every permission this crate covers.
    pub fn all() -> Self {
        Self::new(Permission::ALL)
    }

    /// What was last observed for `permission`, if it is being observed.
    pub fn last(&self, permission: Permission) -> Option<Status> {
        self.last.get(&permission).copied()
    }

    /// Re-reads the observed permissions and returns those whose status changed.
    ///
    /// Call this when your app becomes active, or whenever the user asks you to check again.
    /// Screen recording is the one permission whose grant may stay invisible to a running process,
    /// so a change there can arrive late or, until the app restarts, not at all.
    pub fn refresh(&mut self) -> Vec<Change> {
        let mut changes = Vec::new();
        for (permission, previous) in &mut self.last {
            let current = status(*permission);
            if current != *previous {
                changes.push(Change { permission: *permission, previous: *previous, current });
                *previous = current;
            }
        }
        changes
    }
}
