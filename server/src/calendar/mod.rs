//! Calendar: a small RFC-5545 **subset** recurrence engine + the
//! occurrence expansion the agenda query uses.
//!
//! Supported: `FREQ` (DAILY/WEEKLY/MONTHLY/YEARLY), `INTERVAL`,
//! `COUNT`, `UNTIL`. Unsupported parts (`BYDAY`, `BYMONTHDAY`, …) are
//! ignored, not errors — a documented limitation; a single-user PRM
//! rarely needs full RRULE and we keep the parser honest about what
//! it does. `COUNT` bounds total occurrences from the series start
//! (per RFC), independent of the query window.

use chrono::{DateTime, Datelike, Days, Duration, NaiveDate, NaiveDateTime, Utc};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RRule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<DateTime<Utc>>,
}

/// Parse `FREQ=WEEKLY;INTERVAL=1;COUNT=10` (case-insensitive keys).
/// `None` if there is no usable `FREQ` or a value is malformed.
pub fn parse_rrule(s: &str) -> Option<RRule> {
    let mut freq = None;
    let mut interval = 1u32;
    let mut count = None;
    let mut until = None;
    for part in s.split(';').filter(|p| !p.trim().is_empty()) {
        let (k, v) = part.split_once('=')?;
        match k.trim().to_ascii_uppercase().as_str() {
            "FREQ" => {
                freq = Some(match v.trim().to_ascii_uppercase().as_str() {
                    "DAILY" => Freq::Daily,
                    "WEEKLY" => Freq::Weekly,
                    "MONTHLY" => Freq::Monthly,
                    "YEARLY" => Freq::Yearly,
                    _ => return None,
                })
            }
            "INTERVAL" => interval = v.trim().parse().ok().filter(|n| *n >= 1)?,
            "COUNT" => count = Some(v.trim().parse().ok()?),
            "UNTIL" => until = Some(parse_until(v.trim())?),
            _ => {} // ignore unsupported parts (documented)
        }
    }
    Some(RRule {
        freq: freq?,
        interval,
        count,
        until,
    })
}

fn parse_until(v: &str) -> Option<DateTime<Utc>> {
    if let Ok(d) = DateTime::parse_from_rfc3339(v) {
        return Some(d.with_timezone(&Utc));
    }
    let v = v.trim_end_matches('Z');
    if let Ok(n) = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S") {
        return Some(DateTime::<Utc>::from_naive_utc_and_offset(n, Utc));
    }
    NaiveDate::parse_from_str(v, "%Y%m%d")
        .ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|n| DateTime::<Utc>::from_naive_utc_and_offset(n, Utc))
}

fn last_day_of_month(y: i32, m: u32) -> u32 {
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1)
        .and_then(|d| d.checked_sub_days(Days::new(1)))
        .map(|d| d.day())
        .unwrap_or(28)
}

/// Add `months` calendar months, clamping the day to the target
/// month's last valid day (Jan 31 + 1mo → Feb 28/29). Time-of-day is
/// preserved. Falls back to the input on the (impossible) bad date.
fn add_months(dt: DateTime<Utc>, months: i64) -> DateTime<Utc> {
    let m0 = dt.year() as i64 * 12 + dt.month0() as i64 + months;
    let y = m0.div_euclid(12) as i32;
    let m = m0.rem_euclid(12) as u32 + 1;
    let day = dt.day().min(last_day_of_month(y, m));
    NaiveDate::from_ymd_opt(y, m, day)
        .map(|d| d.and_time(dt.time()))
        .map(|n| DateTime::<Utc>::from_naive_utc_and_offset(n, Utc))
        .unwrap_or(dt)
}

/// Expand a series starting at `start` into the occurrence start
/// times that fall within `[from, to]` (inclusive). Bounded by
/// COUNT/UNTIL and a hard `cap` (runaway safety).
pub fn expand(
    start: DateTime<Utc>,
    rule: &RRule,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
    cap: usize,
) -> Vec<DateTime<Utc>> {
    let mut out = Vec::new();
    let mut i: u32 = 0;
    loop {
        if rule.count.is_some_and(|c| i >= c) || i as usize > cap {
            break;
        }
        // Each occurrence is computed from the ORIGINAL start (not
        // stepped from the previous one) — so a clamped month
        // (Jan 31 → Feb 28) doesn't drag later months off the
        // DTSTART day-of-month, per RFC 5545. `cur` is strictly
        // increasing in `i`, so the break tests below are sound.
        let step = rule.interval as i64 * i as i64;
        let cur = match rule.freq {
            Freq::Daily => start + Duration::days(step),
            Freq::Weekly => start + Duration::weeks(step),
            Freq::Monthly => add_months(start, step),
            Freq::Yearly => add_months(start, 12 * step),
        };
        if rule.until.is_some_and(|u| cur > u) || cur > to {
            break;
        }
        if cur >= from {
            out.push(cur);
        }
        i += 1;
    }
    out
}

/// Occurrences of an event in `[from, to]`. No/!parseable recurrence
/// ⇒ the single `start` if it falls in the window.
pub fn occurrences(
    start: DateTime<Utc>,
    recurrence: Option<&str>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    match recurrence.and_then(parse_rrule) {
        Some(rule) => expand(start, &rule, from, to, 10_000),
        None => {
            if start >= from && start <= to {
                vec![start]
            } else {
                vec![]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    #[test]
    fn parses_subset_and_ignores_unsupported() {
        let r = parse_rrule("FREQ=WEEKLY;INTERVAL=2;COUNT=5;BYDAY=MO").unwrap();
        assert_eq!(r.freq, Freq::Weekly);
        assert_eq!(r.interval, 2);
        assert_eq!(r.count, Some(5));
        assert!(r.until.is_none());
        assert!(parse_rrule("INTERVAL=2").is_none()); // no FREQ
        assert!(parse_rrule("FREQ=FORTNIGHTLY").is_none());
    }

    #[test]
    fn weekly_count_is_window_independent() {
        let start = dt("2026-05-04T09:00:00+00:00"); // Mon
        let occ = occurrences(
            start,
            Some("FREQ=WEEKLY;COUNT=3"),
            dt("2026-05-11T00:00:00+00:00"),
            dt("2027-01-01T00:00:00+00:00"),
        );
        // 3 total in the series (May 4/11/18); window excludes the
        // first, so 2 land in [May 11, …].
        assert_eq!(occ, vec![dt("2026-05-11T09:00:00+00:00"), dt("2026-05-18T09:00:00+00:00")]);
    }

    #[test]
    fn daily_until_inclusive() {
        let occ = occurrences(
            dt("2026-05-01T08:00:00+00:00"),
            Some("FREQ=DAILY;UNTIL=20260503T080000Z"),
            dt("2026-01-01T00:00:00+00:00"),
            dt("2026-12-31T00:00:00+00:00"),
        );
        assert_eq!(occ.len(), 3); // May 1, 2, 3 (UNTIL inclusive)
    }

    #[test]
    fn monthly_clamps_short_months() {
        let occ = occurrences(
            dt("2026-01-31T12:00:00+00:00"),
            Some("FREQ=MONTHLY;COUNT=3"),
            dt("2026-01-01T00:00:00+00:00"),
            dt("2026-12-31T00:00:00+00:00"),
        );
        assert_eq!(
            occ,
            vec![
                dt("2026-01-31T12:00:00+00:00"),
                dt("2026-02-28T12:00:00+00:00"), // clamped
                dt("2026-03-31T12:00:00+00:00"),
            ]
        );
    }

    #[test]
    fn non_recurring_passthrough() {
        let s = dt("2026-06-01T10:00:00+00:00");
        assert_eq!(
            occurrences(s, None, dt("2026-05-01T00:00:00+00:00"), dt("2026-07-01T00:00:00+00:00")),
            vec![s]
        );
        assert!(occurrences(
            s,
            None,
            dt("2026-07-01T00:00:00+00:00"),
            dt("2026-08-01T00:00:00+00:00")
        )
        .is_empty());
    }
}
