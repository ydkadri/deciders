//! The use cases (ADR 0004): they sequence port calls and domain calls, and hold
//! only the rules about the order of those calls and what to do when one gives
//! nothing. The rules of the model are in `domain`. Each takes the ports it
//! needs as arguments.

pub mod init;
pub mod propose;
