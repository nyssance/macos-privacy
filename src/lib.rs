//! Query, request and observe the macOS privacy permissions — the rows under
//! System Settings → Privacy & Security — without binding to any UI framework.
//!
//! # Why the API looks like this
//!
//! macOS does not expose these six permissions uniformly, and most libraries paper over that:
//!
//! * Camera and microphone report the full TCC state machine and their request has a completion
//!   callback, so the user's answer can be awaited.
//! * Input monitoring reports three states — granted, denied, and *not yet asked*.
//! * Accessibility and screen recording report a single bit. Nothing distinguishes "never asked"
//!   from "refused", and there is no callback.
//! * Full disk access has no query and no request API at all; only System Settings can grant it.
//!
//! Flattening all six into `bool` loses the only distinction that changes what an app should do:
//! when the user has already refused, asking again does nothing at all, and the app must send them
//! to System Settings instead. Inventing four states everywhere would be the opposite error. So
//! [`Status`] keeps the difference visible, [`request`] exists only where an answer can actually be
//! awaited, and [`prompt`] promises no more than "the dialog was asked for".
//!
//! ```no_run
//! use macos_privacy::{Permission, Requestable, Status};
//!
//! # async fn example() -> Result<(), macos_privacy::Error> {
//! if !macos_privacy::status(Permission::ScreenRecording).is_authorized() {
//!     macos_privacy::prompt(Permission::ScreenRecording)?;
//! }
//!
//! // Camera and microphone hand back the user's actual answer.
//! let granted = macos_privacy::request(Requestable::Camera).await?;
//! if !granted.may_prompt() && !granted.is_authorized() {
//!     // Refused earlier: asking again is a no-op, so open the pane instead.
//!     macos_privacy::open_settings(Permission::Camera)?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # What this crate will not do
//!
//! * It will not claim a permission is "denied" when macOS only said "not authorized".
//! * It will not poll on a timer. macOS publishes no supported change notification for these
//!   permissions, so [`Observer`] re-checks when you tell it to — typically when your app becomes
//!   active — and reports only what it observed.
//! * It will not request camera or microphone access when the app's `Info.plist` lacks the usage
//!   description: macOS terminates the process for that, so [`request`] returns
//!   [`Error::MissingUsageDescription`] instead.

#![cfg(target_os = "macos")]
#![deny(missing_docs, unsafe_op_in_unsafe_fn)]

mod error;
mod permission;
mod status;
mod sys;

pub use error::Error;
pub use permission::{Permission, Requestable};
pub use status::{AuthorizationStatus, Status};

use objc2_app_kit::NSWorkspace;
use objc2_foundation::NSURL;

/// What the system reports about this process's access right now.
///
/// Screen recording is the one reading that can be stale: a grant made in System Settings is not
/// guaranteed to be visible to a running process, so treat a negative answer as "not observed as
/// authorized", never as "the user refused".
pub fn status(permission: Permission) -> Status {
    #[cfg(feature = "testing")]
    if let Some(status) = testing::override_for(permission) {
        return status;
    }
    match permission {
        Permission::Accessibility => Status::Observed { authorized: sys::accessibility_trusted() },
        Permission::ScreenRecording => Status::Observed { authorized: sys::screen_recording_authorized() },
        Permission::InputMonitoring => Status::Known(sys::input_monitoring_status()),
        Permission::FullDiskAccess => Status::Observed { authorized: full_disk::authorized() },
        Permission::Camera => Status::Known(sys::capture_status(Requestable::Camera)),
        Permission::Microphone => Status::Known(sys::capture_status(Requestable::Microphone)),
    }
}

/// Whether this process is authorized for `permission` right now.
///
/// The one question every permission can answer.
pub fn is_authorized(permission: Permission) -> bool {
    status(permission).is_authorized()
}

/// Asks macOS to show the permission dialog.
///
/// This returns as soon as the request has been made. It reports nothing about whether the dialog
/// appeared — macOS suppresses it once the user has answered — and nothing about their choice.
/// Read [`status`] again later, or use an [`Observer`], to learn the outcome.
///
/// For camera and microphone prefer [`request`], which does receive the answer.
pub fn prompt(permission: Permission) -> Result<(), Error> {
    if let Some(key) = permission.usage_description_key()
        && !sys::info_plist_has(key)
    {
        return Err(Error::MissingUsageDescription { permission, key });
    }
    match permission {
        Permission::Accessibility => sys::accessibility_prompt(),
        Permission::ScreenRecording => sys::screen_recording_prompt(),
        Permission::InputMonitoring => sys::input_monitoring_prompt(),
        Permission::Camera => sys::request_capture_access(Requestable::Camera, |_| {}),
        Permission::Microphone => sys::request_capture_access(Requestable::Microphone, |_| {}),
        Permission::FullDiskAccess => return Err(Error::CannotPrompt(permission)),
    }
    Ok(())
}

/// Requests access and resolves with the user's answer.
///
/// Only camera and microphone have a completion callback, which is why [`Requestable`] is a
/// separate type: for the other four permissions there is nothing to await and this function
/// cannot be called by mistake.
///
/// The callback arrives on an arbitrary dispatch queue; the returned future carries the answer
/// back to whatever task awaits it.
#[cfg(feature = "async")]
pub async fn request(requestable: Requestable) -> Result<AuthorizationStatus, Error> {
    let (sender, receiver) = futures_channel::oneshot::channel();
    request_with_callback(requestable, move |status| {
        let _ = sender.send(status);
    })?;
    // The framework always invokes the block, so the sender cannot be dropped unsent; if macOS
    // ever failed to, awaiting forever would be worse than reporting what we can still observe.
    Ok(receiver.await.unwrap_or_else(|_| sys::capture_status(requestable)))
}

/// The callback form of [`request`], for callers without an async runtime.
///
/// `completion` runs on an arbitrary dispatch queue.
pub fn request_with_callback(
    requestable: Requestable, completion: impl FnOnce(AuthorizationStatus) + Send + 'static,
) -> Result<(), Error> {
    let permission = Permission::from(requestable);
    if let Some(key) = permission.usage_description_key()
        && !sys::info_plist_has(key)
    {
        return Err(Error::MissingUsageDescription { permission, key });
    }
    sys::request_capture_access(requestable, move |granted| {
        completion(if granted { AuthorizationStatus::Authorized } else { sys::capture_status(requestable) });
    });
    Ok(())
}

/// Opens the System Settings pane that governs `permission`.
///
/// This is the only way a user can change a permission they have already refused.
pub fn open_settings(permission: Permission) -> Result<(), Error> {
    let url = NSString::from_str(permission.settings_url());
    let url = NSURL::URLWithString(&url).ok_or(Error::SettingsUnavailable(permission))?;
    let opened = NSWorkspace::sharedWorkspace().openURL(&url);
    if opened { Ok(()) } else { Err(Error::SettingsUnavailable(permission)) }
}

use objc2_foundation::NSString;

mod full_disk;
mod observer;
pub use observer::Observer;

#[cfg(feature = "testing")]
pub mod testing;
