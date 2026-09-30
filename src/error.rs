use core::fmt;

use crate::Permission;

/// Why a privacy operation could not be carried out.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The permission has no request API; only System Settings can grant it.
    CannotPrompt(Permission),
    /// The app's `Info.plist` is missing the usage description macOS requires before asking.
    ///
    /// Requesting anyway terminates the process, so the request was not made.
    MissingUsageDescription {
        /// The permission that was about to be requested.
        permission: Permission,
        /// The `Info.plist` key macOS requires first.
        key: &'static str,
    },
    /// The System Settings pane could not be opened.
    SettingsUnavailable(Permission),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CannotPrompt(permission) => {
                write!(f, "{permission} cannot be requested programmatically; open System Settings instead")
            }
            Self::MissingUsageDescription { permission, key } => {
                write!(
                    f,
                    "Info.plist is missing {key}; macOS terminates the process if {permission} is requested without it"
                )
            }
            Self::SettingsUnavailable(permission) => {
                write!(f, "could not open the System Settings pane for {permission}")
            }
        }
    }
}

impl core::error::Error for Error {}
