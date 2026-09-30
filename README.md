# macos-privacy

Query, request and observe the macOS privacy permissions — the rows under **System Settings →
Privacy & Security** — from Rust, with no UI framework dependency.

```toml
[target.'cfg(target_os = "macos")'.dependencies]
macos-privacy = "0.1"
```

| Permission | What macOS reports | Can be requested |
|---|---|---|
| Camera, Microphone | full state: not determined / restricted / denied / authorized | yes, with the user's answer |
| Input Monitoring | granted / denied / not yet asked | yes, no answer |
| Accessibility | authorized or not | yes, no answer |
| Screen & System Audio Recording | authorized or not, possibly stale until restart | yes, no answer |
| Full Disk Access | probe only | no — System Settings only |

## Why not a `bool`

Every other crate returns `bool`. That loses the one distinction that changes what an app must do:
once the user has refused, requesting again does **nothing at all**, and the app has to send them to
System Settings instead. A `bool` cannot tell that apart from "never asked", so apps built on it sit
waiting for a dialog that will never appear.

It would be just as wrong to invent four states everywhere, because four of the six permissions
never report them. So [`Status`] has two shapes — a precise state where macOS provides one, and an
explicit *observed / not observed as authorized* where it does not.

## Requesting

`request` exists only for camera and microphone, because they are the only permissions whose
request has a completion callback. That is enforced by the type system: it takes `Requestable`,
not `Permission`.

```rust,no_run
# async fn example() -> Result<(), macos_privacy::Error> {
use macos_privacy::{AuthorizationStatus, Permission, Requestable};

match macos_privacy::request(Requestable::Microphone).await? {
    AuthorizationStatus::Authorized => { /* start recording */ }
    AuthorizationStatus::NotDetermined => { /* the dialog is still up */ }
    // Refused or restricted: asking again does nothing.
    _ => macos_privacy::open_settings(Permission::Microphone)?
}
# Ok(())
# }
```

For the rest, `prompt` asks macOS to show the dialog and promises nothing more — not that the
dialog appeared, not what the user chose.

### Info.plist

macOS **terminates the process** if camera or microphone access is requested while the app's
`Info.plist` lacks `NSCameraUsageDescription` / `NSMicrophoneUsageDescription`. `request` checks
first and returns `Error::MissingUsageDescription` instead of letting the app die.

## Observing changes

macOS publishes no supported change notification for these permissions. This crate therefore does
not subscribe to anything, does not spawn threads and does not poll on a timer. `Observer` re-reads
when you ask — the natural moment is when your app becomes active again, since granting any of
these requires leaving it — and reports only what it observed.

```rust,no_run
let mut observer = macos_privacy::Observer::all();
// on application activation:
for change in observer.refresh() {
    println!("{} → {:?}", change.permission, change.current);
}
```

## Testing

TCC state belongs to the system, so a consumer's tests would otherwise only ever exercise whichever
branch the developer's machine is in. Enable the `testing` feature in dev-dependencies and pin any
state:

```rust
# #[cfg(feature = "testing")] {
use macos_privacy::{AuthorizationStatus, Permission, Status, testing};

let _guard = testing::pin(Permission::Camera, Status::Known(AuthorizationStatus::Denied));
assert!(!macos_privacy::is_authorized(Permission::Camera));
# }
```

## License

MIT OR Apache-2.0.
