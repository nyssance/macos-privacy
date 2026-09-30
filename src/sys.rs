//! The macOS frameworks, called through typed bindings. Every unsafe block here is a single
//! framework call whose contract is stated in the comment above it.

use block2::RcBlock;
use objc2::runtime::Bool;
use objc2_application_services::{AXIsProcessTrusted, AXIsProcessTrustedWithOptions, kAXTrustedCheckOptionPrompt};
use objc2_av_foundation::{AVAuthorizationStatus, AVCaptureDevice, AVMediaType, AVMediaTypeAudio, AVMediaTypeVideo};
use objc2_core_foundation::{CFBoolean, CFDictionary, CFString, kCFBooleanTrue};
use objc2_core_graphics::{CGPreflightScreenCaptureAccess, CGRequestScreenCaptureAccess};
use objc2_foundation::{NSBundle, NSString};
use objc2_io_kit::{IOHIDAccessType, IOHIDCheckAccess, IOHIDRequestAccess, IOHIDRequestType};

use crate::{AuthorizationStatus, Requestable};

/// `AXIsProcessTrusted` answers one bit and never says why, so this is all the accessibility
/// state macOS exposes.
pub(crate) fn accessibility_trusted() -> bool {
    // Reads the process's own trust flag; no arguments, no side effects.
    unsafe { AXIsProcessTrusted() }
}

/// Shows the "grant accessibility access" dialog. macOS decides whether the dialog actually
/// appears — it is suppressed once the user has answered — and reports nothing about the answer.
pub(crate) fn accessibility_prompt() {
    // SAFETY: framework constants with static lifetime; `kCFBooleanTrue` is never null.
    let (key, value): (&CFString, &CFBoolean) = unsafe { (kAXTrustedCheckOptionPrompt, kCFBooleanTrue.unwrap()) };
    let options = CFDictionary::from_slices(&[key], &[value]);
    // SAFETY: the options dictionary is well-formed and only read by the call.
    unsafe {
        AXIsProcessTrustedWithOptions(Some(options.as_opaque()));
    }
}

/// `CGPreflightScreenCaptureAccess` answers one bit. A grant made in System Settings is not
/// guaranteed to be visible to this process before it restarts, so a `false` here can be stale.
pub(crate) fn screen_recording_authorized() -> bool {
    CGPreflightScreenCaptureAccess()
}

/// Shows the screen recording dialog once. Later calls return without asking again.
pub(crate) fn screen_recording_prompt() {
    let _ = CGRequestScreenCaptureAccess();
}

/// Input monitoring is the one non-AV permission with three states: `Unknown` means the prompt
/// has not been shown yet, which is the same information `NotDetermined` carries elsewhere.
pub(crate) fn input_monitoring_status() -> AuthorizationStatus {
    match IOHIDCheckAccess(IOHIDRequestType::ListenEvent) {
        IOHIDAccessType::Granted => AuthorizationStatus::Authorized,
        IOHIDAccessType::Denied => AuthorizationStatus::Denied,
        _ => AuthorizationStatus::NotDetermined,
    }
}

/// Shows the input monitoring dialog. The return value reports whether access is granted now,
/// not what the user will choose, so it is deliberately discarded.
pub(crate) fn input_monitoring_prompt() {
    let _ = IOHIDRequestAccess(IOHIDRequestType::ListenEvent);
}

fn media_type(requestable: Requestable) -> &'static AVMediaType {
    // SAFETY: both are framework string constants with static lifetime; AVFoundation always
    // defines them, and requesting capture access without one is not a recoverable state.
    unsafe {
        match requestable {
            Requestable::Camera => AVMediaTypeVideo.expect("AVMediaTypeVideo is defined by AVFoundation"),
            Requestable::Microphone => AVMediaTypeAudio.expect("AVMediaTypeAudio is defined by AVFoundation"),
        }
    }
}

/// Camera and microphone are the only two permissions where macOS reports the full state machine.
pub(crate) fn capture_status(requestable: Requestable) -> AuthorizationStatus {
    // SAFETY: class method on AVCaptureDevice taking a framework media-type constant.
    let status = unsafe { AVCaptureDevice::authorizationStatusForMediaType(media_type(requestable)) };
    match status {
        AVAuthorizationStatus::Authorized => AuthorizationStatus::Authorized,
        AVAuthorizationStatus::Denied => AuthorizationStatus::Denied,
        AVAuthorizationStatus::Restricted => AuthorizationStatus::Restricted,
        _ => AuthorizationStatus::NotDetermined,
    }
}

/// Requests capture access and hands the user's answer to `completion`.
///
/// The completion block is required by the API — passing `nil` violates its contract — and macOS
/// invokes it on an arbitrary dispatch queue, so `completion` must not assume any thread.
pub(crate) fn request_capture_access(requestable: Requestable, completion: impl FnOnce(bool) + Send + 'static) {
    let completion = core::cell::Cell::new(Some(completion));
    let block = RcBlock::new(move |granted: Bool| {
        if let Some(completion) = completion.take() {
            completion(granted.as_bool());
        }
    });
    // SAFETY: the block lives until the framework releases it, and the media type is a constant.
    unsafe {
        AVCaptureDevice::requestAccessForMediaType_completionHandler(media_type(requestable), &block);
    }
}

/// Reads a key from the main bundle's `Info.plist`.
pub(crate) fn info_plist_has(key: &str) -> bool {
    let bundle = NSBundle::mainBundle();
    let key = NSString::from_str(key);
    bundle.objectForInfoDictionaryKey(&key).is_some()
}
