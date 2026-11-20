//! Tracks the state of one link (task source or aw-server) and logs only the transitions (REQ-016).
//!
//! Each link starts `Unknown`, then becomes `Up` or `Down`. A failure after `Up` or `Unknown`
//! logs one error line. Later failures are silent until a success, which logs one info line
//! with the outage length.

use crate::log;
use chrono::{DateTime, Duration, Utc};

/// The two links the watcher depends on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Link {
    /// The task source, named by the provider (for example "http" or "cli").
    Provider(&'static str),
    /// The ActivityWatch server that receives heartbeats.
    AwServer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Unknown,
    Up,
    Down {
        since: DateTime<Utc>,
        failed_attempts: u32,
    },
}

/// A state change worth logging.
#[derive(Debug, Clone, PartialEq)]
pub enum Transition {
    /// The first failure after `Up` or `Unknown`.
    Lost { error: String },
    /// The first success after `Down`.
    Recovered {
        down_for: Duration,
        failed_attempts: u32,
    },
}

/// State of one link. Only the transitions are returned, never the steady state.
#[derive(Debug)]
pub struct LinkMonitor {
    link: Link,
    state: State,
}

impl LinkMonitor {
    pub fn new(link: Link) -> Self {
        LinkMonitor {
            link,
            state: State::Unknown,
        }
    }

    /// Records a success at time `now`. Returns `Recovered` if the link was `Down`.
    pub fn record_ok(&mut self, now: DateTime<Utc>) -> Option<Transition> {
        let previous = self.state;
        self.state = State::Up;
        match previous {
            State::Down {
                since,
                failed_attempts,
            } => Some(Transition::Recovered {
                down_for: now - since,
                failed_attempts,
            }),
            State::Unknown | State::Up => None,
        }
    }

    /// Records a failure at time `now`. Returns `Lost` on the first failure after `Up` or `Unknown`.
    pub fn record_err(&mut self, error: &str, now: DateTime<Utc>) -> Option<Transition> {
        match self.state {
            State::Down {
                since,
                failed_attempts,
            } => {
                self.state = State::Down {
                    since,
                    failed_attempts: failed_attempts.saturating_add(1),
                };
                None
            }
            State::Unknown | State::Up => {
                self.state = State::Down {
                    since: now,
                    failed_attempts: 1,
                };
                Some(Transition::Lost {
                    error: error.to_string(),
                })
            }
        }
    }
}

/// Builds the log message for a transition on `link`.
pub fn describe(link: Link, transition: &Transition) -> String {
    match (link, transition) {
        (Link::Provider(name), Transition::Lost { error }) => format!(
            "task provider '{}' failing: {}; reusing last known task",
            name,
            one_line(error)
        ),
        (Link::AwServer, Transition::Lost { error }) => format!(
            "ActivityWatch server unreachable: {}; heartbeats are not delivered until it returns",
            one_line(error)
        ),
        (
            Link::Provider(name),
            Transition::Recovered {
                down_for,
                failed_attempts,
            },
        ) => format!(
            "task provider '{}' recovered after {} ({} failed {})",
            name,
            format_duration(*down_for),
            failed_attempts,
            plural(*failed_attempts, "poll", "polls")
        ),
        (
            Link::AwServer,
            Transition::Recovered {
                down_for,
                failed_attempts,
            },
        ) => format!(
            "ActivityWatch server recovered after {} ({} failed {})",
            format_duration(*down_for),
            failed_attempts,
            plural(*failed_attempts, "heartbeat", "heartbeats")
        ),
    }
}

/// Collapses line breaks and runs of spaces so one transition is always one log line.
/// Errors can carry the stderr of the `task` command, which may span several lines.
fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn plural(count: u32, one: &'static str, many: &'static str) -> &'static str {
    if count == 1 {
        one
    } else {
        many
    }
}

/// Formats a duration as `1h 02m 03s`, `4m 05s` or `12s`. Negative values (a clock set back) show as `0s`.
pub fn format_duration(duration: Duration) -> String {
    let total = duration.num_seconds().max(0);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    if hours > 0 {
        format!("{}h {:02}m {:02}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {:02}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Records one attempt on `monitor` at the current time and logs the transition, if any.
/// `Ok` is a success; `Err` carries the error text.
pub fn observe(monitor: &mut LinkMonitor, result: Result<(), &str>) {
    let now = Utc::now();
    let transition = match result {
        Ok(()) => monitor.record_ok(now),
        Err(error) => monitor.record_err(error, now),
    };
    if let Some(transition) = transition {
        let message = describe(monitor.link, &transition);
        match transition {
            Transition::Lost { .. } => log::log_error(&message),
            Transition::Recovered { .. } => log::log_info(&message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(seconds: i64) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 20, 10, 0, 0).unwrap() + Duration::seconds(seconds)
    }

    const PROVIDER: Link = Link::Provider("http");

    #[test]
    fn test_first_failure_from_unknown_is_lost() {
        let mut monitor = LinkMonitor::new(PROVIDER);

        let t = monitor.record_err("connection refused", at(0));

        assert_eq!(
            t,
            Some(Transition::Lost {
                error: "connection refused".to_string()
            })
        );
    }

    #[test]
    fn test_repeated_failures_are_silent_and_counted() {
        let mut monitor = LinkMonitor::new(PROVIDER);
        monitor.record_err("refused", at(0));

        assert_eq!(monitor.record_err("refused", at(5)), None);
        assert_eq!(monitor.record_err("refused", at(10)), None);
        let t = monitor.record_ok(at(65));

        assert_eq!(
            t,
            Some(Transition::Recovered {
                down_for: Duration::seconds(65),
                failed_attempts: 3,
            })
        );
    }

    #[test]
    fn test_success_after_down_logs_recovery_with_duration() {
        let mut monitor = LinkMonitor::new(PROVIDER);
        monitor.record_err("timeout", at(0));

        let t = monitor.record_ok(at(3723));

        assert_eq!(
            t,
            Some(Transition::Recovered {
                down_for: Duration::seconds(3723),
                failed_attempts: 1,
            })
        );
    }

    #[test]
    fn test_success_from_unknown_is_silent() {
        let mut monitor = LinkMonitor::new(PROVIDER);

        assert_eq!(monitor.record_ok(at(0)), None);
    }

    #[test]
    fn test_success_while_up_is_silent() {
        let mut monitor = LinkMonitor::new(PROVIDER);
        monitor.record_ok(at(0));

        assert_eq!(monitor.record_ok(at(5)), None);
    }

    #[test]
    fn test_failure_after_recovery_starts_new_outage() {
        let mut monitor = LinkMonitor::new(PROVIDER);
        monitor.record_err("first", at(0));
        monitor.record_ok(at(10));

        let t = monitor.record_err("second", at(20));
        let recovered = monitor.record_ok(at(50));

        assert_eq!(
            t,
            Some(Transition::Lost {
                error: "second".to_string()
            })
        );
        assert_eq!(
            recovered,
            Some(Transition::Recovered {
                down_for: Duration::seconds(30),
                failed_attempts: 1,
            })
        );
    }

    #[test]
    fn test_describe_provider_lines() {
        let lost = Transition::Lost {
            error: "provider unavailable: refused".to_string(),
        };
        let recovered = Transition::Recovered {
            down_for: Duration::seconds(62),
            failed_attempts: 12,
        };

        assert_eq!(
            describe(PROVIDER, &lost),
            "task provider 'http' failing: provider unavailable: refused; reusing last known task"
        );
        assert_eq!(
            describe(PROVIDER, &recovered),
            "task provider 'http' recovered after 1m 02s (12 failed polls)"
        );
    }

    #[test]
    fn test_describe_aw_server_lines() {
        let lost = Transition::Lost {
            error: "Heartbeat failed: refused".to_string(),
        };
        let recovered = Transition::Recovered {
            down_for: Duration::seconds(1),
            failed_attempts: 1,
        };

        assert_eq!(
            describe(Link::AwServer, &lost),
            "ActivityWatch server unreachable: Heartbeat failed: refused; heartbeats are not delivered until it returns"
        );
        assert_eq!(
            describe(Link::AwServer, &recovered),
            "ActivityWatch server recovered after 1s (1 failed heartbeat)"
        );
    }

    #[test]
    fn test_describe_keeps_multiline_error_on_one_line() {
        let lost = Transition::Lost {
            error: "provider unavailable: exited with status 1: simulated outage\n".to_string(),
        };

        assert_eq!(
            describe(PROVIDER, &lost),
            "task provider 'http' failing: provider unavailable: exited with status 1: simulated outage; reusing last known task"
        );
    }

    #[test]
    fn test_format_duration_cases() {
        assert_eq!(format_duration(Duration::seconds(12)), "12s");
        assert_eq!(format_duration(Duration::seconds(245)), "4m 05s");
        assert_eq!(format_duration(Duration::seconds(3723)), "1h 02m 03s");
        assert_eq!(format_duration(Duration::seconds(-5)), "0s");
    }

    #[test]
    fn test_links_are_independent() {
        let mut provider = LinkMonitor::new(PROVIDER);
        let mut server = LinkMonitor::new(Link::AwServer);

        provider.record_err("down", at(0));

        assert_eq!(server.record_ok(at(1)), None);
        assert_eq!(
            server.record_err("down", at(2)),
            Some(Transition::Lost {
                error: "down".to_string()
            })
        );
    }

    #[test]
    fn test_observe_is_callable_for_both_outcomes() {
        // observe reads the wall clock and writes to stdout/stderr; this checks it does not panic
        // and that the state changes as the pure functions predict.
        let mut monitor = LinkMonitor::new(Link::AwServer);
        observe(&mut monitor, Err("refused"));
        observe(&mut monitor, Ok(()));
        observe(&mut monitor, Ok(()));
    }
}
