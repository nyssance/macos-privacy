//! Test seam. TCC state belongs to the system and cannot be simulated, so a consumer's tests
//! could otherwise only exercise whichever branch the developer's own machine happens to be in.
//!
//! Enable the `testing` feature in dev-dependencies and pin whatever state the test needs:
//!
//! ```
//! # #[cfg(feature = "testing")] {
//! use macos_privacy::{AuthorizationStatus, Permission, Status, testing};
//!
//! let _guard = testing::pin(Permission::Camera, Status::Known(AuthorizationStatus::Denied));
//! assert!(!macos_privacy::is_authorized(Permission::Camera));
//! # }
//! ```
//!
//! Overrides are per-thread, so tests running in parallel do not disturb each other, and the
//! guard restores the previous value when dropped.

use std::cell::RefCell;
use std::collections::BTreeMap;

use crate::{Permission, Status};

thread_local! {
    static OVERRIDES: RefCell<BTreeMap<Permission, Status>> = const { RefCell::new(BTreeMap::new()) };
}

/// Restores the previous override when dropped.
#[must_use = "the override is removed as soon as the guard is dropped"]
pub struct Pin {
    permission: Permission,
    previous: Option<Status>,
}

/// Makes [`crate::status`] report `status` for `permission` on this thread.
pub fn pin(permission: Permission, status: Status) -> Pin {
    let previous = OVERRIDES.with(|overrides| overrides.borrow_mut().insert(permission, status));
    Pin { permission, previous }
}

impl Drop for Pin {
    fn drop(&mut self) {
        OVERRIDES.with(|overrides| {
            let mut overrides = overrides.borrow_mut();
            match self.previous {
                Some(previous) => overrides.insert(self.permission, previous),
                None => overrides.remove(&self.permission),
            }
        });
    }
}

pub(crate) fn override_for(permission: Permission) -> Option<Status> {
    OVERRIDES.with(|overrides| overrides.borrow().get(&permission).copied())
}
