//! The ADR lifecycle (ADR 0004): four states and the moves between them.

use std::fmt;
use std::str::FromStr;

/// Where an ADR is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Written, waiting for a decision.
    Proposed,
    /// Agreed, not yet built.
    Accepted,
    /// Agreed and built.
    Implemented,
    /// Turned down.
    Rejected,
}

/// A command that moves an ADR from one state to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// `accept`: from `proposed` to `accepted`.
    Accept,
    /// `reject`: from `proposed` to `rejected`.
    Reject,
    /// `implement`: from `accepted` to `implemented`.
    Implement,
}

impl Status {
    /// The state after `movement`, if the ADR is in the state it needs.
    ///
    /// # Errors
    ///
    /// Returns a message naming the current state and the state the move needs.
    pub fn after(self, movement: Move) -> Result<Self, String> {
        let (needs, becomes) = match movement {
            Move::Accept => (Self::Proposed, Self::Accepted),
            Move::Reject => (Self::Proposed, Self::Rejected),
            Move::Implement => (Self::Accepted, Self::Implemented),
        };
        if self == needs {
            Ok(becomes)
        } else {
            Err(format!(
                "the ADR is {self}, and `{movement}` needs it to be {needs}"
            ))
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Proposed => "proposed",
            Self::Accepted => "accepted",
            Self::Implemented => "implemented",
            Self::Rejected => "rejected",
        })
    }
}

impl fmt::Display for Move {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Accept => "accept",
            Self::Reject => "reject",
            Self::Implement => "implement",
        })
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "proposed" => Ok(Self::Proposed),
            "accepted" => Ok(Self::Accepted),
            "implemented" => Ok(Self::Implemented),
            "rejected" => Ok(Self::Rejected),
            other => Err(format!(
                "unknown status {other:?}, expected proposed, accepted, implemented or rejected"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Status; 4] = [
        Status::Proposed,
        Status::Accepted,
        Status::Implemented,
        Status::Rejected,
    ];

    #[test]
    fn accept_and_reject_need_proposed_and_implement_needs_accepted() {
        assert_eq!(
            Status::Proposed.after(Move::Accept),
            Ok(Status::Accepted),
            "accept"
        );
        assert_eq!(
            Status::Proposed.after(Move::Reject),
            Ok(Status::Rejected),
            "reject"
        );
        assert_eq!(
            Status::Accepted.after(Move::Implement),
            Ok(Status::Implemented),
            "implement"
        );
    }

    #[test]
    fn every_other_move_is_refused() {
        let allowed = [
            (Status::Proposed, Move::Accept),
            (Status::Proposed, Move::Reject),
            (Status::Accepted, Move::Implement),
        ];
        for status in ALL {
            for movement in [Move::Accept, Move::Reject, Move::Implement] {
                if !allowed.contains(&(status, movement)) {
                    assert!(status.after(movement).is_err(), "{status} then {movement}");
                }
            }
        }
    }

    #[test]
    fn a_refused_move_names_the_state_and_what_the_move_needs() {
        assert_eq!(
            Status::Rejected.after(Move::Accept).unwrap_err(),
            "the ADR is rejected, and `accept` needs it to be proposed",
            "message"
        );
    }

    #[test]
    fn a_final_state_allows_no_move() {
        for status in [Status::Implemented, Status::Rejected] {
            for movement in [Move::Accept, Move::Reject, Move::Implement] {
                assert!(status.after(movement).is_err(), "{status} then {movement}");
            }
        }
    }

    #[test]
    fn a_status_round_trips_through_text() {
        for status in ALL {
            assert_eq!(status.to_string().parse::<Status>(), Ok(status), "{status}");
        }
    }

    #[test]
    fn text_that_is_not_a_status_is_refused() {
        for text in ["", "Accepted", "superseded", "accepted "] {
            assert!(text.parse::<Status>().is_err(), "{text:?}");
        }
        let message = "done".parse::<Status>().unwrap_err();
        assert!(
            message.contains("\"done\"") && message.contains("rejected"),
            "{message}"
        );
    }
}
