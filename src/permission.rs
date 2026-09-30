use core::fmt;

/// A macOS privacy permission, as named in System Settings → Privacy & Security.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Permission {
    /// Accessibility — control the computer, synthesize input, inspect other apps' UI.
    Accessibility,
    /// Screen & System Audio Recording — read the contents of other windows.
    ScreenRecording,
    /// Input Monitoring — observe keyboard and pointer events while not frontmost.
    InputMonitoring,
    /// Full Disk Access — read the files macOS protects by default.
    FullDiskAccess,
    /// Camera.
    Camera,
    /// Microphone.
    Microphone,
}

/// A permission whose request has a completion callback, so the result can be awaited.
///
/// This is a separate type on purpose: for the other four permissions macOS has no callback, so
/// there is nothing to await, and a uniform `request` would have to lie about that.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Requestable {
    /// Camera.
    Camera,
    /// Microphone.
    Microphone,
}

impl Permission {
    /// Every permission this crate covers.
    pub const ALL: [Self; 6] = [
        Self::Accessibility,
        Self::ScreenRecording,
        Self::InputMonitoring,
        Self::FullDiskAccess,
        Self::Camera,
        Self::Microphone,
    ];

    /// Whether [`crate::prompt`] can surface a system dialog for this permission.
    ///
    /// Full Disk Access has no request API at all: the user has to add the app in System Settings.
    pub const fn can_prompt(self) -> bool {
        !matches!(self, Self::FullDiskAccess)
    }

    /// The awaitable form, when this permission has one.
    pub const fn requestable(self) -> Option<Requestable> {
        match self {
            Self::Camera => Some(Requestable::Camera),
            Self::Microphone => Some(Requestable::Microphone),
            _ => None,
        }
    }

    /// The `Info.plist` key macOS requires before this permission may be requested.
    ///
    /// Requesting camera or microphone access without it does not fail — the system terminates
    /// the process. [`crate::request`] checks this first and returns an error instead.
    pub const fn usage_description_key(self) -> Option<&'static str> {
        match self {
            Self::Camera => Some("NSCameraUsageDescription"),
            Self::Microphone => Some("NSMicrophoneUsageDescription"),
            _ => None,
        }
    }

    /// The System Settings pane that governs this permission.
    pub(crate) const fn settings_url(self) -> &'static str {
        match self {
            Self::Accessibility => "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility",
            Self::ScreenRecording => "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            Self::InputMonitoring => "x-apple.systempreferences:com.apple.preference.security?Privacy_ListenEvent",
            Self::FullDiskAccess => "x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles",
            Self::Camera => "x-apple.systempreferences:com.apple.preference.security?Privacy_Camera",
            Self::Microphone => "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone",
        }
    }
}

impl From<Requestable> for Permission {
    fn from(value: Requestable) -> Self {
        match value {
            Requestable::Camera => Self::Camera,
            Requestable::Microphone => Self::Microphone,
        }
    }
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Accessibility => "Accessibility",
            Self::ScreenRecording => "Screen & System Audio Recording",
            Self::InputMonitoring => "Input Monitoring",
            Self::FullDiskAccess => "Full Disk Access",
            Self::Camera => "Camera",
            Self::Microphone => "Microphone",
        })
    }
}
