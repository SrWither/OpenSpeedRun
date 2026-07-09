use chrono::Duration;

/// Parses a .NET `TimeSpan`-formatted string as LiveSplit writes them:
/// `[-]H:MM:SS[.fffffff]` (fractional seconds are optional and can have any
/// number of digits — LiveSplit itself writes both `00:01:18` and
/// `00:01:16.3560000` in the same file). Only millisecond precision is kept,
/// which is all `Duration`/our own format already carry.
///
/// Doesn't handle the `d.hh:mm:ss` day-prefixed form .NET uses for spans
/// over 24h — not a realistic duration for a single split/attempt.
pub fn parse_dotnet_timespan(raw: &str) -> Option<Duration> {
    let raw = raw.trim();
    let (negative, raw) = match raw.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, raw),
    };

    let (hms, millis) = match raw.split_once('.') {
        Some((hms, frac)) => {
            let frac3: String = frac.chars().chain(std::iter::repeat('0')).take(3).collect();
            (hms, frac3.parse::<i64>().ok()?)
        }
        None => (raw, 0),
    };

    let mut parts = hms.split(':');
    let hours: i64 = parts.next()?.parse().ok()?;
    let minutes: i64 = parts.next()?.parse().ok()?;
    let seconds: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }

    let total_millis = ((hours * 60 + minutes) * 60 + seconds) * 1000 + millis;
    let total_millis = if negative { -total_millis } else { total_millis };
    Some(Duration::milliseconds(total_millis))
}

/// Formats a `Duration` as a .NET `TimeSpan` string LiveSplit can parse:
/// `[-]HH:MM:SS.fff`.
pub fn format_dotnet_timespan(duration: Duration) -> String {
    let sign = if duration < Duration::zero() { "-" } else { "" };
    let abs = duration.abs();

    let hours = abs.num_hours();
    let minutes = abs.num_minutes() % 60;
    let seconds = abs.num_seconds() % 60;
    let millis = abs.num_milliseconds() % 1000;

    format!("{sign}{hours:02}:{minutes:02}:{seconds:02}.{millis:03}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_with_and_without_fraction() {
        assert_eq!(
            parse_dotnet_timespan("00:01:16.3560000"),
            Some(Duration::milliseconds(76356))
        );
        assert_eq!(
            parse_dotnet_timespan("00:01:18"),
            Some(Duration::milliseconds(78000))
        );
        assert_eq!(
            parse_dotnet_timespan("-0:00:10"),
            Some(Duration::milliseconds(-10000))
        );
    }

    #[test]
    fn round_trips_through_format() {
        let d = Duration::milliseconds(76356);
        assert_eq!(parse_dotnet_timespan(&format_dotnet_timespan(d)), Some(d));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(parse_dotnet_timespan("not a time"), None);
        assert_eq!(parse_dotnet_timespan("1:2:3:4"), None);
    }
}
