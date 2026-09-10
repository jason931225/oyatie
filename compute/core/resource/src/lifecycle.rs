#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum ResourceState {
    Pending,
    Running,
    Stopped,
    Terminated,
    Error,
}

impl ResourceState {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminated)
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Terminated => "terminated",
            Self::Error => "error",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "stopped" => Some(Self::Stopped),
            "terminated" => Some(Self::Terminated),
            "error" => Some(Self::Error),
            _ => None,
        }
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::Running)
    }

    pub const fn is_quiescent(self) -> bool {
        matches!(self, Self::Stopped)
    }

    pub const fn allowed_next(self) -> &'static [Self] {
        match self {
            Self::Pending => &[Self::Pending, Self::Running, Self::Error, Self::Terminated],
            Self::Running => &[Self::Running, Self::Stopped, Self::Error, Self::Terminated],
            Self::Stopped => &[Self::Stopped, Self::Running, Self::Error, Self::Terminated],
            Self::Error => &[Self::Error, Self::Terminated],
            Self::Terminated => &[Self::Terminated],
        }
    }

    pub fn can_transition_to(self, next: Self) -> bool {
        state_transition_allowed(self, next)
    }
}

pub(crate) fn state_transition_allowed(current: ResourceState, next: ResourceState) -> bool {
    current.allowed_next().contains(&next)
}
