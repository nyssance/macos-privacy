/// What the system reports about this process's access to a permission.
///
/// macOS is not uniform here: two of the six permissions expose the full TCC state machine,
/// the rest answer a single question — "is this process authorized right now". This type keeps
/// that difference visible instead of flattening it into a `bool` (which loses the distinction
/// between "never asked" and "denied") or into a four-state enum (which would invent states the
/// system never reports).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// The system reports a precise authorization state.
    Known(AuthorizationStatus),
    /// The system reports only whether this process is authorized at this moment.
    ///
    /// `authorized: false` means *not observed as authorized*. It does **not** tell you whether
    /// the user was ever asked, whether they refused, or whether asking again would show a
    /// dialog. Do not derive any of those from it.
    Observed {
        /// Whether this process was observed to be authorized just now.
        authorized: bool,
    },
}

impl Status {
    /// The one question every variant can answer.
    pub const fn is_authorized(self) -> bool {
        match self {
            Self::Known(status) => status.is_authorized(),
            Self::Observed { authorized } => authorized,
        }
    }

    /// The precise state, when the system reports one.
    pub const fn known(self) -> Option<AuthorizationStatus> {
        match self {
            Self::Known(status) => Some(status),
            Self::Observed { .. } => None,
        }
    }
}

/// The TCC authorization state, as reported by the frameworks that expose it.
///
/// The discriminants match Apple's `AVAuthorizationStatus`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(isize)]
pub enum AuthorizationStatus {
    /// The user has not been asked yet. A request can still show the system dialog.
    NotDetermined = 0,
    /// Access is denied by policy (parental controls, MDM) and the user cannot change it.
    Restricted = 1,
    /// The user refused. Asking again does nothing; only System Settings can change it.
    Denied = 2,
    /// Access is granted.
    Authorized = 3,
}

impl AuthorizationStatus {
    /// Whether access is granted.
    pub const fn is_authorized(self) -> bool {
        matches!(self, Self::Authorized)
    }

    /// Whether a request could still surface the system dialog.
    ///
    /// Only `NotDetermined` can; for every other state the request call returns without asking
    /// the user, which is why callers must send them to System Settings instead.
    pub const fn may_prompt(self) -> bool {
        matches!(self, Self::NotDetermined)
    }
}
