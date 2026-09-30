//! The parts of the model that hold regardless of what this machine has granted.

use macos_privacy::{AuthorizationStatus, Permission, Requestable, Status};

#[test]
fn only_not_determined_can_still_prompt() {
    assert!(AuthorizationStatus::NotDetermined.may_prompt());
    for status in [AuthorizationStatus::Restricted, AuthorizationStatus::Denied, AuthorizationStatus::Authorized] {
        assert!(!status.may_prompt(), "{status:?} cannot surface the dialog again");
    }
}

#[test]
fn observed_status_reports_no_precise_state() {
    assert_eq!(Status::Observed { authorized: false }.known(), None);
    assert!(!Status::Observed { authorized: false }.is_authorized());
    assert!(Status::Observed { authorized: true }.is_authorized());
    assert_eq!(Status::Known(AuthorizationStatus::Denied).known(), Some(AuthorizationStatus::Denied));
}

#[test]
fn only_capture_permissions_can_be_awaited() {
    let requestable: Vec<_> = Permission::ALL.into_iter().filter(|p| p.requestable().is_some()).collect();
    assert_eq!(requestable, [Permission::Camera, Permission::Microphone]);
    assert_eq!(Permission::from(Requestable::Camera), Permission::Camera);
}

#[test]
fn full_disk_access_is_the_one_permission_that_cannot_be_requested() {
    let cannot: Vec<_> = Permission::ALL.into_iter().filter(|p| !p.can_prompt()).collect();
    assert_eq!(cannot, [Permission::FullDiskAccess]);
}

#[test]
fn capture_permissions_declare_their_info_plist_key() {
    assert_eq!(Permission::Camera.usage_description_key(), Some("NSCameraUsageDescription"));
    assert_eq!(Permission::Microphone.usage_description_key(), Some("NSMicrophoneUsageDescription"));
    assert_eq!(Permission::Accessibility.usage_description_key(), None);
}

#[test]
fn every_permission_has_a_settings_pane() {
    for permission in Permission::ALL {
        assert!(!format!("{permission}").is_empty());
    }
}
