//! The ADR lifecycle: the five states and the moves between them (ADR 0001).

use std::fmt;
use std::str::FromStr;

/// Where an ADR is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    /// Written and awaiting a decision. Incomplete.
    Proposed,
    /// Agreed but not yet built. Incomplete.
    Accepted,
    /// Agreed and built. Complete.
    Implemented,
    /// Considered and turned down. Complete.
    Rejected,
    /// Replaced by a later ADR. Complete.
    Superseded,
}

/// A value that is not one of the five statuses.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(
    "unknown status {value:?}, expected one of: proposed, accepted, implemented, rejected, superseded"
)]
pub struct StatusError {
    value: String,
}

/// A move the lifecycle does not allow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionError {
    from: Status,
    to: Status,
}

impl Status {
    /// Every status, in lifecycle order.
    pub const ALL: [Self; 5] = [
        Self::Proposed,
        Self::Accepted,
        Self::Implemented,
        Self::Rejected,
        Self::Superseded,
    ];

    /// Whether `decider check` treats an ADR in this status as finished.
    ///
    /// Only `proposed` and `accepted` are incomplete.
    pub fn is_complete(self) -> bool {
        matches!(self, Self::Implemented | Self::Rejected | Self::Superseded)
    }

    /// The statuses an ADR can move to from this one.
    pub fn next_states(self) -> &'static [Self] {
        match self {
            Self::Proposed => &[Self::Accepted, Self::Rejected],
            Self::Accepted => &[Self::Implemented, Self::Superseded],
            Self::Implemented => &[Self::Superseded],
            Self::Rejected | Self::Superseded => &[],
        }
    }

    /// Move to `target`, if the lifecycle allows it.
    ///
    /// # Errors
    ///
    /// Returns [`TransitionError`] naming both statuses and the moves that are
    /// allowed from the current one.
    pub fn transition(self, target: Self) -> Result<Self, TransitionError> {
        if self.next_states().contains(&target) {
            Ok(target)
        } else {
            Err(TransitionError {
                from: self,
                to: target,
            })
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Implemented => "implemented",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
        };
        formatter.write_str(name)
    }
}

impl FromStr for Status {
    type Err = StatusError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|status| status.to_string() == text)
            .ok_or_else(|| StatusError {
                value: text.to_owned(),
            })
    }
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "cannot move an ADR from {} to {}: ",
            self.from, self.to
        )?;
        match self.from.next_states() {
            [] => write!(formatter, "{} is final", self.from),
            moves => {
                let names: Vec<String> = moves.iter().map(ToString::to_string).collect();
                write!(
                    formatter,
                    "from {} it can move to {}",
                    self.from,
                    names.join(" or ")
                )
            }
        }
    }
}

impl std::error::Error for TransitionError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_proposed_and_accepted_are_incomplete() {
        let incomplete: Vec<Status> = Status::ALL
            .into_iter()
            .filter(|status| !status.is_complete())
            .collect();
        assert_eq!(
            incomplete,
            [Status::Proposed, Status::Accepted],
            "incomplete set"
        );
    }

    #[test]
    fn allowed_moves_match_the_lifecycle_table() {
        assert_eq!(
            Status::Proposed.next_states(),
            [Status::Accepted, Status::Rejected],
            "proposed"
        );
        assert_eq!(
            Status::Accepted.next_states(),
            [Status::Implemented, Status::Superseded],
            "accepted"
        );
        assert_eq!(
            Status::Implemented.next_states(),
            [Status::Superseded],
            "implemented"
        );
        assert!(
            Status::Rejected.next_states().is_empty(),
            "rejected is final"
        );
        assert!(
            Status::Superseded.next_states().is_empty(),
            "superseded is final"
        );
    }

    #[test]
    fn transition_succeeds_for_every_allowed_move() {
        for from in Status::ALL {
            for &to in from.next_states() {
                assert_eq!(from.transition(to), Ok(to), "{from} to {to}");
            }
        }
    }

    #[test]
    fn transition_fails_for_every_move_the_table_does_not_allow() {
        for from in Status::ALL {
            for to in Status::ALL {
                if !from.next_states().contains(&to) {
                    assert!(from.transition(to).is_err(), "{from} to {to} should fail");
                }
            }
        }
    }

    #[test]
    fn proposed_cannot_skip_to_implemented() {
        let error = Status::Proposed
            .transition(Status::Implemented)
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot move an ADR from proposed to implemented: from proposed it can move to accepted or rejected",
            "the message should say what is allowed"
        );
    }

    #[test]
    fn final_states_say_they_are_final() {
        let error = Status::Rejected.transition(Status::Accepted).unwrap_err();
        assert_eq!(
            error.to_string(),
            "cannot move an ADR from rejected to accepted: rejected is final",
            "terminal states cannot move"
        );
    }

    #[test]
    fn a_status_cannot_move_to_itself() {
        assert!(
            Status::Accepted.transition(Status::Accepted).is_err(),
            "no self move"
        );
    }

    #[test]
    fn parses_and_displays_every_status() {
        for status in Status::ALL {
            assert_eq!(
                status.to_string().parse::<Status>(),
                Ok(status),
                "round trip of {status}"
            );
        }
    }

    #[test]
    fn rejects_unknown_or_differently_cased_statuses() {
        for text in ["", "Accepted", "ACCEPTED", "superseded by 0009", "done"] {
            assert!(
                text.parse::<Status>().is_err(),
                "{text:?} should be rejected"
            );
        }
    }

    #[test]
    fn the_parse_error_lists_the_valid_statuses() {
        let message = "done".parse::<Status>().unwrap_err().to_string();
        assert!(
            message.contains("\"done\""),
            "names the bad value: {message}"
        );
        assert!(
            message.contains("superseded"),
            "lists the choices: {message}"
        );
    }
}
