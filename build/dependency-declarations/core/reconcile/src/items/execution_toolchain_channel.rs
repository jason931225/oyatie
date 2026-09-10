const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const NIGHTLY_PREFIX: &str = "nightly-";
const FLOATING_NIGHTLY: &str = "nightly";

/// Calendar day naming one exact nightly release.
///
/// Ordered by date; the order is meaningful only against another nightly.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct NightlyDate {
    year: u16,
    month: u8,
    day: u8,
}

/// Identity of the execution toolchain channel.
///
/// The two identities are independent: a dated nightly carries no semantic
/// version and therefore no ordering against a stable release. This type
/// deliberately implements neither `Ord` nor `PartialOrd`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExecutionChannel {
    Stable(Version),
    DatedNightly(NightlyDate),
}

impl NightlyDate {
    pub fn year(&self) -> u16 {
        self.year
    }

    pub fn month(&self) -> u8 {
        self.month
    }

    pub fn day(&self) -> u8 {
        self.day
    }
}

impl ExecutionChannel {
    /// The stable version when this channel is a stable release.
    pub fn stable(&self) -> Option<&Version> {
        match self {
            Self::Stable(version) => Some(version),
            Self::DatedNightly(_) => None,
        }
    }

    /// The release date when this channel is an exact dated nightly.
    pub fn dated_nightly(&self) -> Option<NightlyDate> {
        match self {
            Self::Stable(_) => None,
            Self::DatedNightly(date) => Some(*date),
        }
    }

    /// Whether this channel satisfies a declared stable MSRV floor.
    ///
    /// A dated nightly is built from a branch ahead of every released stable,
    /// so the floor is satisfied for every MSRV value; the answer is vacuously
    /// true rather than skipped.
    pub fn meets_msrv(&self, msrv: &Version) -> bool {
        match self {
            Self::Stable(version) => version >= msrv,
            Self::DatedNightly(_) => true,
        }
    }
}

fn parse_execution_channel(
    field: &'static str,
    value: &str,
) -> Result<ExecutionChannel, DeclarationRefusal> {
    if value == FLOATING_NIGHTLY {
        return Err(DeclarationRefusal::FloatingNightlyChannel(
            field,
            value.to_owned(),
        ));
    }
    match value.strip_prefix(NIGHTLY_PREFIX) {
        Some(date) => nightly_date(field, value, date).map(ExecutionChannel::DatedNightly),
        None => stable_version(field, value).map(ExecutionChannel::Stable),
    }
}

fn nightly_date(
    field: &'static str,
    value: &str,
    date: &str,
) -> Result<NightlyDate, DeclarationRefusal> {
    let invalid = || DeclarationRefusal::InvalidNightlyDate(field, value.to_owned());
    let mut parts = date.split('-');
    let year = fixed_width_number(parts.next(), 4).ok_or_else(invalid)?;
    let month = fixed_width_number(parts.next(), 2).ok_or_else(invalid)?;
    let day = fixed_width_number(parts.next(), 2).ok_or_else(invalid)?;
    if parts.next().is_some() {
        return Err(invalid());
    }
    let year = u16::try_from(year).map_err(|_| invalid())?;
    let month = u8::try_from(month).map_err(|_| invalid())?;
    let day = u8::try_from(day).map_err(|_| invalid())?;
    if month == 0 || month > 12 || day == 0 || day > days_in_month(year, month) {
        return Err(invalid());
    }
    Ok(NightlyDate { year, month, day })
}

fn fixed_width_number(part: Option<&str>, width: usize) -> Option<u32> {
    let part = part?;
    if part.len() != width || !part.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
}

fn days_in_month(year: u16, month: u8) -> u8 {
    let index = usize::from(month) - 1;
    if month == 2 && is_leap_year(year) {
        return DAYS_IN_MONTH[index] + 1;
    }
    DAYS_IN_MONTH[index]
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

impl fmt::Display for NightlyDate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { year, month, day } = self;
        write!(formatter, "{year:04}-{month:02}-{day:02}")
    }
}

impl fmt::Display for ExecutionChannel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stable(version) => write!(formatter, "{version}"),
            Self::DatedNightly(date) => write!(formatter, "{NIGHTLY_PREFIX}{date}"),
        }
    }
}
