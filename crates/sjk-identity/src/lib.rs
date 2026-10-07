//! Player identity for SJK: an Ed25519 key the player keeps, and the client side
//! of the SJK hub's protocol (`PROTOCOL.md` in Sol-Vulpes/SJK-hub) that turns it
//! into a profile and a badge on game servers.
//!
//! The crate has no window, renderer or game-network knowledge. The viewer
//! creates a [`Service`], feeds it the player's [`Settings`] and [`Location`],
//! and reads its [`Snapshot`].

#![warn(missing_docs)]

pub mod hub;
mod keys;
pub mod report;
pub mod service;
pub mod wire;

pub use hub::{HttpHub, Hub, HubError, valid_base_url};
pub use keys::{Identity, KeyError};
pub use report::{BugReport, Category, PlayerReport, WorldNote};
pub use service::{HubFactory, Location, ReportOutcome, Service, Settings, Snapshot, Status};
pub use wire::{Presence, Profile, WornName, names_match, normal_form};
