//! The background service that keeps the player registered with the hub, repeats
//! their game-server claim while they play, and reads who else on that server is
//! known. Nothing here blocks a frame: the viewer sends [`Command`]s through
//! [`Service`] and reads a [`Snapshot`].
//!
//! The service is inert while it is disabled or has no hub address: it makes no
//! request of any kind.

use crate::hub::{Hub, HubError};
use crate::keys::Identity;
use crate::report::{BugReport, WorldNote};
use crate::wire::{Presence, Profile, names_match};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// How often a claim is repeated; a claim lives 90 seconds at the hub.
const CLAIM_EVERY: Duration = Duration::from_secs(45);
/// How often the roster of known players is read.
const POLL_EVERY: Duration = Duration::from_secs(15);
/// First wait after a failure, doubled up to [`RETRY_MAX`].
const RETRY_MIN: Duration = Duration::from_secs(10);
const RETRY_MAX: Duration = Duration::from_secs(120);
/// Longest the worker sleeps without looking at its commands.
const IDLE_MAX: Duration = Duration::from_secs(60);

/// What the player has configured.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Settings {
    /// Whether the identity features are on.
    pub enabled: bool,
    /// The hub's address; empty means no hub, so nothing is sent.
    pub hub_url: String,
}

/// Where the player is: the game server they are connected to and how the game
/// shows them there.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Location {
    /// The server's address as connected.
    pub server: SocketAddr,
    /// The player's own client number there.
    pub slot: u8,
    /// The player's name as the game shows it.
    pub name: String,
}

/// How the service is doing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    /// Turned off in the settings.
    Disabled,
    /// On, but there is no hub address.
    NoHub,
    /// Registering with the hub.
    Registering,
    /// Registered; claims and reads work.
    Online,
    /// The last hub request failed (the text says why); the service retries.
    Failed(String),
}

/// What the viewer may read.
#[derive(Clone, Debug)]
pub struct Snapshot {
    /// The service's status.
    pub status: Status,
    /// The player's own key id.
    pub key_id: String,
    /// The player's profile at the hub, once registered.
    pub me: Option<Profile>,
    /// The game server being claimed on, if any.
    pub server: Option<SocketAddr>,
    /// Known players on that server, by slot.
    pub players: Vec<Presence>,
    /// Profiles fetched with [`Service::look_up`].
    pub profiles: HashMap<String, Profile>,
    /// The outcome of the last profile change or lookup, for the page to show.
    pub notice: Option<String>,
    /// Counts changes to [`Snapshot::players`], so a reader that caches what it
    /// derived from them (scoreboard tags) knows when to derive again.
    pub revision: u64,
    /// The outcome of the last bug report sent with [`Service::report`].
    pub report: Option<ReportOutcome>,
    /// The outcome of the last world note sent with [`Service::note`].
    pub note: Option<ReportOutcome>,
}

/// What became of a bug report or a world note.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportOutcome {
    /// Counts reports sent, so a reader can tell a new outcome from the last one.
    pub serial: u64,
    /// The hub stored it.
    pub sent: bool,
    /// The hub's report number, or why it was not stored.
    pub message: String,
}

impl Snapshot {
    fn new(key_id: String) -> Self {
        Self {
            status: Status::Disabled,
            key_id,
            me: None,
            server: None,
            players: Vec::new(),
            profiles: HashMap::new(),
            notice: None,
            revision: 0,
            report: None,
            note: None,
        }
    }

    /// The known player in `slot` whose claimed name matches `shown`, the name the
    /// game shows there. A claim for another name is somebody else's and is ignored.
    pub fn badge(&self, slot: u8, shown: &str) -> Option<&Presence> {
        self.players
            .iter()
            .find(|player| player.slot == slot && names_match(&player.claimed_name, shown))
    }
}

/// A request to the worker.
enum Command {
    Configure(Settings),
    Enter(Location),
    Leave,
    /// The in-game name the player now wears.
    Name(String),
    SetBio(String),
    LookUp(String),
    Report(BugReport),
    /// A world note and the tag its picture will come with.
    Note(u64, WorldNote),
    /// The picture of the note tagged so.
    NoteImage(u64, Vec<u8>),
    Stop,
}

/// Notes whose pictures may still come, kept by tag: the newest few only.
const NOTES_AWAITING_PICTURES: usize = 8;

/// Builds the hub client for an address; `Err` for an address that cannot be used.
pub type HubFactory = Box<dyn Fn(&str) -> Result<Box<dyn Hub>, HubError> + Send>;

/// The worker's logic, advanced by [`Worker::handle`] and [`Worker::tick`] with
/// an explicit clock so it can be tested without waiting.
struct Worker {
    identity: Identity,
    make_hub: HubFactory,
    settings: Settings,
    hub: Option<Box<dyn Hub>>,
    location: Option<Location>,
    /// The server address the hub holds a claim for, to withdraw it.
    claimed: Option<String>,
    registered: bool,
    /// The in-game name the player wears, and the one the hub was last told.
    name: Option<String>,
    name_sent: Option<String>,
    snapshot: Arc<Mutex<Snapshot>>,
    due_register: Instant,
    due_name: Instant,
    due_claim: Instant,
    due_poll: Instant,
    backoff: Duration,
    lookups: Vec<String>,
    /// Notes the hub took, as (tag, hub id), while their pictures may come.
    notes_sent: Vec<(u64, i64)>,
}

/// What the player is told of a report or a note: what the hub stored it as, or why
/// it did not.
fn outcome_of(serial: u64, outcome: Result<String, HubError>) -> ReportOutcome {
    match outcome {
        Ok(message) => ReportOutcome {
            serial,
            sent: true,
            message,
        },
        Err(HubError::Rejected { message, .. }) => ReportOutcome {
            serial,
            sent: false,
            message,
        },
        Err(error) => ReportOutcome {
            serial,
            sent: false,
            message: error.to_string(),
        },
    }
}

fn lock(snapshot: &Mutex<Snapshot>) -> MutexGuard<'_, Snapshot> {
    snapshot
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Worker {
    fn new(
        identity: Identity,
        make_hub: HubFactory,
        snapshot: Arc<Mutex<Snapshot>>,
        now: Instant,
    ) -> Self {
        Self {
            identity,
            make_hub,
            settings: Settings::default(),
            hub: None,
            location: None,
            claimed: None,
            registered: false,
            name: None,
            name_sent: None,
            snapshot,
            due_register: now,
            due_name: now,
            due_claim: now,
            due_poll: now,
            backoff: RETRY_MIN,
            lookups: Vec::new(),
            notes_sent: Vec::new(),
        }
    }

    fn update(&self, change: impl FnOnce(&mut Snapshot)) {
        change(&mut lock(&self.snapshot));
    }

    fn failed(&mut self, error: &HubError, now: Instant) -> Instant {
        self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()));
        let due = now + self.backoff;
        self.backoff = (self.backoff * 2).min(RETRY_MAX);
        due
    }

    /// Withdraw the claim the hub holds, if any, ignoring a failure: it expires.
    fn release(&mut self) {
        if let (Some(server), Some(hub)) = (self.claimed.take(), self.hub.as_mut()) {
            let _ = hub.release(&self.identity, &server);
        }
        self.update(|snapshot| {
            snapshot.server = None;
            snapshot.players.clear();
            snapshot.revision += 1;
        });
    }

    fn handle(&mut self, command: Command, now: Instant) {
        match command {
            Command::Configure(settings) => self.configure(settings, now),
            Command::Enter(location) => {
                if self.location.as_ref() == Some(&location) {
                    return;
                }
                if self.location.as_ref().map(|old| old.server) != Some(location.server) {
                    self.release();
                }
                self.update(|snapshot| snapshot.server = Some(location.server));
                self.location = Some(location);
                self.due_claim = now;
                self.due_poll = now;
            }
            Command::Leave => {
                self.release();
                self.location = None;
            }
            Command::Name(name) => {
                let name = Some(name).filter(|name| !name.trim().is_empty());
                if name != self.name {
                    self.name = name;
                    self.due_name = now;
                }
            }
            Command::SetBio(bio) => self.set_bio(&bio),
            Command::Report(mut report) => {
                if report.name.is_empty() {
                    report.name = self.name.clone().unwrap_or_default();
                }
                self.report(&report);
            }
            Command::Note(tag, mut note) => {
                if note.name.is_empty() {
                    note.name = self.name.clone().unwrap_or_default();
                }
                self.note(tag, &note);
            }
            Command::NoteImage(tag, jpeg) => self.note_image(tag, &jpeg),
            Command::LookUp(key_id) => self.lookups.push(key_id),
            Command::Stop => self.release(),
        }
    }

    fn configure(&mut self, settings: Settings, now: Instant) {
        if settings == self.settings {
            return;
        }
        // Anything that changes the hub or switches the feature off ends the claim
        // first, on the hub that holds it.
        self.release();
        self.hub = None;
        self.registered = false;
        self.name_sent = None;
        self.update(|snapshot| snapshot.me = None);
        if settings.enabled && !settings.hub_url.trim().is_empty() {
            match (self.make_hub)(settings.hub_url.trim()) {
                Ok(hub) => self.hub = Some(hub),
                Err(error) => {
                    self.update(|snapshot| snapshot.status = Status::Failed(error.to_string()))
                }
            }
        }
        self.settings = settings;
        self.backoff = RETRY_MIN;
        self.due_register = now;
        self.due_claim = now;
        self.due_poll = now;
    }

    fn note(&mut self, tag: u64, note: &WorldNote) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.note(&self.identity, note),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        if let Ok(id) = outcome {
            if self.notes_sent.len() >= NOTES_AWAITING_PICTURES {
                self.notes_sent.remove(0);
            }
            self.notes_sent.push((tag, id));
        }
        self.update(|snapshot| {
            let serial = snapshot.note.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.note = Some(outcome_of(serial, outcome.map(|id| format!("note #{id}"))));
        });
    }

    /// Attach `jpeg` to the note tagged `tag`, if the hub took that note. A picture
    /// that fails leaves the note as it is.
    fn note_image(&mut self, tag: u64, jpeg: &[u8]) {
        let Some(index) = self.notes_sent.iter().position(|(sent, _)| *sent == tag) else {
            return;
        };
        let (_, id) = self.notes_sent.remove(index);
        if let Some(hub) = self.hub.as_mut() {
            let _ = hub.note_image(&self.identity, id, jpeg);
        }
    }

    fn report(&mut self, report: &BugReport) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.report(&self.identity, report),
            _ => Err(HubError::Protocol(
                "not connected to the hub (is identity on, cl_identity 1?)".to_owned(),
            )),
        };
        self.update(|snapshot| {
            let serial = snapshot.report.as_ref().map_or(1, |last| last.serial + 1);
            snapshot.report = Some(outcome_of(
                serial,
                outcome.map(|id| format!("report #{id}")),
            ));
        });
    }

    fn set_bio(&mut self, bio: &str) {
        let outcome = match (self.hub.as_mut(), self.registered) {
            (Some(hub), true) => hub.set_bio(&self.identity, bio),
            _ => {
                self.update(|snapshot| {
                    snapshot.notice = Some("not connected to the hub".to_owned())
                });
                return;
            }
        };
        self.update(|snapshot| match outcome {
            Ok(profile) => {
                snapshot.me = Some(profile);
                snapshot.notice = Some("saved".to_owned());
            }
            Err(error) => snapshot.notice = Some(error.to_string()),
        });
    }

    /// Do whatever is due at `now`; return how long to wait for the next thing.
    fn tick(&mut self, now: Instant) -> Duration {
        if !self.settings.enabled {
            self.update(|snapshot| snapshot.status = Status::Disabled);
            return IDLE_MAX;
        }
        if self.hub.is_none() {
            if self.settings.hub_url.trim().is_empty() {
                self.update(|snapshot| snapshot.status = Status::NoHub);
            }
            return IDLE_MAX;
        }
        if !self.registered {
            if now < self.due_register {
                return self.due_register - now;
            }
            self.update(|snapshot| {
                if !matches!(snapshot.status, Status::Failed(_)) {
                    snapshot.status = Status::Registering;
                }
            });
            let name = self.name.clone();
            let outcome = self
                .hub
                .as_mut()
                .map(|hub| hub.register(&self.identity, name.as_deref()));
            match outcome {
                Some(Ok(profile)) => {
                    self.registered = true;
                    self.name_sent = name;
                    self.backoff = RETRY_MIN;
                    self.update(|snapshot| {
                        snapshot.me = Some(profile);
                        snapshot.status = Status::Online;
                    });
                }
                Some(Err(error)) => {
                    self.due_register = self.failed(&error, now);
                    return self.due_register - now;
                }
                None => {}
            }
        }
        self.run_due(now);
        let mut wait = IDLE_MAX;
        if self.location.is_some() {
            wait = wait
                .min(self.due_claim.saturating_duration_since(now))
                .min(self.due_poll.saturating_duration_since(now));
        }
        if self.name != self.name_sent {
            wait = wait.min(self.due_name.saturating_duration_since(now));
        }
        if !self.lookups.is_empty() {
            wait = Duration::ZERO;
        }
        wait
    }

    /// A new name, the claim, the roster read and the pending lookups, where due.
    fn run_due(&mut self, now: Instant) {
        let Some(hub) = self.hub.as_mut() else { return };
        let mut error = None;
        // A name worn since registering joins the key's history at the hub (a
        // registration of a known key only adds the name). A failure waits and retries.
        if self.name != self.name_sent && now >= self.due_name {
            match hub.register(&self.identity, self.name.as_deref()) {
                Ok(profile) => {
                    self.name_sent = self.name.clone();
                    lock(&self.snapshot).me = Some(profile);
                }
                Err(failure) => {
                    self.due_name = now + self.backoff;
                    error = Some(failure);
                }
            }
        }
        if let Some(location) = self.location.clone() {
            let server = location.server.to_string();
            if now >= self.due_claim {
                match hub.claim(&self.identity, &server, location.slot, &location.name) {
                    Ok(()) => {
                        self.claimed = Some(server.clone());
                        self.due_claim = now + CLAIM_EVERY;
                    }
                    Err(failure) => {
                        self.due_claim = now + self.backoff;
                        error = Some(failure);
                    }
                }
            }
            if now >= self.due_poll {
                match hub.presence(&server) {
                    Ok(players) => {
                        self.due_poll = now + POLL_EVERY;
                        let mut snapshot = lock(&self.snapshot);
                        if snapshot.players != players {
                            snapshot.players = players;
                            snapshot.revision += 1;
                        }
                    }
                    Err(failure) => {
                        self.due_poll = now + self.backoff;
                        error = Some(failure);
                    }
                }
            }
        }
        for key_id in std::mem::take(&mut self.lookups) {
            match hub.profile(&key_id) {
                Ok(profile) => {
                    lock(&self.snapshot)
                        .profiles
                        .insert(profile.key_id.clone(), profile);
                }
                Err(failure) => {
                    lock(&self.snapshot).notice = Some(failure.to_string());
                }
            }
        }
        match error {
            Some(failure) => {
                self.backoff = (self.backoff * 2).min(RETRY_MAX);
                self.update(|snapshot| snapshot.status = Status::Failed(failure.to_string()));
            }
            None => {
                self.backoff = RETRY_MIN;
                self.update(|snapshot| {
                    if matches!(snapshot.status, Status::Failed(_)) {
                        snapshot.status = Status::Online;
                    }
                });
            }
        }
    }
}

/// The running service: commands go in, a [`Snapshot`] comes out.
pub struct Service {
    commands: Sender<Command>,
    snapshot: Arc<Mutex<Snapshot>>,
    finished: Mutex<Receiver<()>>,
    /// The last tag given to a note ([`Service::note`]).
    note_tags: std::sync::atomic::AtomicU64,
}

impl Service {
    /// Start the worker thread for `identity`. `make_hub` builds a hub client for
    /// the configured address.
    pub fn start(identity: Identity, make_hub: HubFactory) -> Self {
        let snapshot = Arc::new(Mutex::new(Snapshot::new(identity.key_id())));
        let (commands, inbox) = channel();
        let (done, finished) = channel();
        let mut worker = Worker::new(identity, make_hub, Arc::clone(&snapshot), Instant::now());
        let spawned = std::thread::Builder::new()
            .name("sjk-identity".to_owned())
            .spawn(move || {
                loop {
                    let wait = worker.tick(Instant::now());
                    match inbox.recv_timeout(wait) {
                        Ok(Command::Stop) | Err(RecvTimeoutError::Disconnected) => {
                            worker.handle(Command::Stop, Instant::now());
                            break;
                        }
                        Ok(command) => worker.handle(command, Instant::now()),
                        Err(RecvTimeoutError::Timeout) => {}
                    }
                }
                let _ = done.send(());
            });
        if let Err(error) = spawned {
            lock(&snapshot).status =
                Status::Failed(format!("cannot start the identity thread: {error}"));
        }
        Self {
            commands,
            snapshot,
            finished: Mutex::new(finished),
            note_tags: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Apply the player's settings; the service starts or stops talking to the hub.
    pub fn configure(&self, settings: Settings) {
        let _ = self.commands.send(Command::Configure(settings));
    }

    /// The player is on a game server (call again if `slot` or `name` changes).
    pub fn enter(&self, location: Location) {
        let _ = self.commands.send(Command::Enter(location));
    }

    /// The player left their game server.
    pub fn leave(&self) {
        let _ = self.commands.send(Command::Leave);
    }

    /// The in-game name the player wears: sent with the registration and again
    /// whenever it changes, for the key's name history at the hub.
    pub fn set_name(&self, name: String) {
        let _ = self.commands.send(Command::Name(name));
    }

    /// Change the player's bio at the hub.
    pub fn set_bio(&self, bio: String) {
        let _ = self.commands.send(Command::SetBio(bio));
    }

    /// Send a bug report; its outcome arrives in [`Snapshot::report`].
    pub fn report(&self, report: BugReport) {
        let _ = self.commands.send(Command::Report(report));
    }

    /// Send a world note; its outcome arrives in [`Snapshot::note`]. The answer tags
    /// the note for [`Service::note_image`].
    pub fn note(&self, note: WorldNote) -> u64 {
        let tag = self
            .note_tags
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1;
        let _ = self.commands.send(Command::Note(tag, note));
        tag
    }

    /// The picture of the note tagged `tag`, a JPEG; sent once the hub took the note.
    pub fn note_image(&self, tag: u64, jpeg: Vec<u8>) {
        let _ = self.commands.send(Command::NoteImage(tag, jpeg));
    }

    /// Fetch a player's profile (their bio) into [`Snapshot::profiles`].
    pub fn look_up(&self, key_id: String) {
        let _ = self.commands.send(Command::LookUp(key_id));
    }

    /// A copy of the current state.
    pub fn snapshot(&self) -> Snapshot {
        lock(&self.snapshot).clone()
    }

    /// Read the current state without copying it; keep `read` short, the worker
    /// waits for the lock.
    pub fn with_snapshot<R>(&self, read: impl FnOnce(&Snapshot) -> R) -> R {
        read(&lock(&self.snapshot))
    }

    /// Withdraw the claim and stop, waiting at most `timeout` for it.
    pub fn shutdown(&self, timeout: Duration) {
        let _ = self.commands.send(Command::Stop);
        let finished = self
            .finished
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let _ = finished.recv_timeout(timeout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// A hub that records what it is asked.
    #[derive(Clone, Default)]
    struct Fake {
        log: Arc<Mutex<Vec<String>>>,
        fail: Arc<AtomicBool>,
        roster: Arc<Mutex<Vec<Presence>>>,
    }

    impl Fake {
        fn log(&self) -> Vec<String> {
            self.log.lock().unwrap().clone()
        }

        fn record(&self, line: String) -> Result<(), HubError> {
            self.log.lock().unwrap().push(line);
            if self.fail.load(Ordering::SeqCst) {
                Err(HubError::Network("down".to_owned()))
            } else {
                Ok(())
            }
        }
    }

    fn profile(name: &str) -> Profile {
        Profile {
            key_id: "0123456789abcdef".to_owned(),
            key: String::new(),
            name: name.to_owned(),
            bio: String::new(),
            verified: false,
            created: 0,
            names: Vec::new(),
        }
    }

    impl Hub for Fake {
        fn register(&mut self, _: &Identity, name: Option<&str>) -> Result<Profile, HubError> {
            match name {
                Some(name) => self
                    .record(format!("register {name}"))
                    .map(|()| profile(name)),
                None => self.record("register".to_owned()).map(|()| profile("")),
            }
        }
        fn set_bio(&mut self, _: &Identity, bio: &str) -> Result<Profile, HubError> {
            self.record(format!("bio {bio}")).map(|()| profile(""))
        }
        fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
            self.record(format!("lookup {key_id}"))
                .map(|()| profile("Other"))
        }
        fn claim(
            &mut self,
            _: &Identity,
            server: &str,
            slot: u8,
            name: &str,
        ) -> Result<(), HubError> {
            self.record(format!("claim {server} {slot} {name}"))
        }
        fn release(&mut self, _: &Identity, server: &str) -> Result<(), HubError> {
            self.record(format!("release {server}"))
        }
        fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
            self.record(format!("presence {server}"))
                .map(|()| self.roster.lock().unwrap().clone())
        }
        fn report(&mut self, _: &Identity, report: &BugReport) -> Result<i64, HubError> {
            self.record(format!("report {}", report.text))?;
            self.record(format!("report name {}", report.name))
                .map(|()| 7)
        }
        fn note(&mut self, _: &Identity, note: &WorldNote) -> Result<i64, HubError> {
            Fake::record(self, format!("note {} {}", note.text, note.shader))?;
            Fake::record(self, format!("note name {}", note.name)).map(|()| 9)
        }
        fn note_image(&mut self, _: &Identity, id: i64, jpeg: &[u8]) -> Result<(), HubError> {
            Fake::record(self, format!("image {id} {} bytes", jpeg.len()))
        }
    }

    fn worker(fake: &Fake, now: Instant) -> (Worker, Arc<Mutex<Snapshot>>) {
        let identity = Identity::from_seed([1; 32]);
        let snapshot = Arc::new(Mutex::new(Snapshot::new(identity.key_id())));
        let factory = fake.clone();
        let make: HubFactory = Box::new(move |url| {
            if url.starts_with("https://") {
                Ok(Box::new(factory.clone()))
            } else {
                Err(HubError::Network("bad address".to_owned()))
            }
        });
        (
            Worker::new(identity, make, Arc::clone(&snapshot), now),
            snapshot,
        )
    }

    fn on(url: &str) -> Settings {
        Settings {
            enabled: true,
            hub_url: url.to_owned(),
        }
    }

    fn here(slot: u8, name: &str) -> Location {
        Location {
            server: "1.2.3.4:29070".parse().unwrap(),
            slot,
            name: name.to_owned(),
        }
    }

    #[test]
    fn nothing_is_sent_while_disabled_or_without_a_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::Disabled);
        worker.handle(Command::Configure(on("")), t0);
        worker.tick(t0);
        assert_eq!(lock(&snapshot).status, Status::NoHub);
        worker.handle(
            Command::Configure(Settings {
                enabled: false,
                hub_url: "https://h".into(),
            }),
            t0,
        );
        worker.tick(t0);
        assert!(fake.log().is_empty());
    }

    #[test]
    fn it_registers_before_it_claims_and_repeats_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(
            fake.log(),
            [
                "register",
                "claim 1.2.3.4:29070 3 Sol",
                "presence 1.2.3.4:29070"
            ]
        );
        assert_eq!(lock(&snapshot).status, Status::Online);
        // Nothing is due for a while, then the roster, then the claim.
        worker.tick(t0 + Duration::from_secs(10));
        assert_eq!(fake.log().len(), 3);
        worker.tick(t0 + Duration::from_secs(16));
        assert_eq!(fake.log().last().unwrap(), "presence 1.2.3.4:29070");
        worker.tick(t0 + Duration::from_secs(46));
        assert!(
            fake.log()
                .iter()
                .filter(|line| line.starts_with("claim"))
                .count()
                == 2
        );
    }

    #[test]
    fn leaving_or_switching_off_withdraws_the_claim() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Leave, t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        worker.handle(Command::Configure(Settings::default()), t0);
        assert_eq!(fake.log().last().unwrap(), "release 1.2.3.4:29070");
        let before = fake.log().len();
        worker.tick(t0 + Duration::from_secs(500));
        assert_eq!(fake.log().len(), before);
    }

    #[test]
    fn a_new_name_or_slot_is_claimed_at_once_and_the_same_one_is_not_repeated() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, _) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        let claims = |fake: &Fake| fake.log().iter().filter(|l| l.starts_with("claim")).count();
        worker.handle(Command::Enter(here(3, "Sol")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 1);
        worker.handle(Command::Enter(here(3, "Fox")), t0);
        worker.tick(t0);
        assert_eq!(claims(&fake), 2);
        assert_eq!(
            fake.log()
                .iter()
                .filter(|l| l.starts_with("release"))
                .count(),
            0
        );
    }

    #[test]
    fn failures_back_off_and_recovery_clears_the_status() {
        let fake = Fake::default();
        fake.fail.store(true, Ordering::SeqCst);
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        let wait = worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert_eq!(wait, RETRY_MIN);
        worker.tick(t0 + Duration::from_secs(5));
        assert_eq!(fake.log().len(), 1, "no retry before the backoff ends");
        let wait = worker.tick(t0 + Duration::from_secs(11));
        assert_eq!(wait, RETRY_MIN * 2);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(40));
        assert_eq!(lock(&snapshot).status, Status::Online);
    }

    #[test]
    fn a_bad_address_is_reported_not_used() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("http://hub.example")), t0);
        worker.tick(t0);
        assert!(matches!(lock(&snapshot).status, Status::Failed(_)));
        assert!(fake.log().is_empty());
    }

    #[test]
    fn badges_match_by_slot_and_name_only() {
        let fake = Fake::default();
        *fake.roster.lock().unwrap() = vec![Presence {
            slot: 3,
            claimed_name: "^1Sol".to_owned(),
            key_id: "0123456789abcdef".to_owned(),
            name: "Sol".to_owned(),
            verified: true,
        }];
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.handle(Command::Enter(here(1, "Me")), t0);
        worker.tick(t0);
        let snapshot = lock(&snapshot);
        assert!(snapshot.badge(3, "sol").unwrap().verified);
        assert!(
            snapshot.badge(3, "Fox").is_none(),
            "another name in the slot"
        );
        assert!(snapshot.badge(4, "Sol").is_none(), "another slot");
    }

    #[test]
    fn bio_changes_and_lookups_go_through_the_hub() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::SetBio("x".into()), t0);
        assert_eq!(
            lock(&snapshot).notice.as_deref(),
            Some("not connected to the hub")
        );
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::SetBio("hello".into()), t0);
        assert_eq!(fake.log().last().unwrap(), "bio hello");
        assert_eq!(lock(&snapshot).notice.as_deref(), Some("saved"));
        worker.handle(Command::LookUp("0123456789abcdef".into()), t0);
        worker.tick(t0);
        assert!(lock(&snapshot).profiles.contains_key("0123456789abcdef"));
    }

    #[test]
    fn notes_go_to_the_hub_and_their_pictures_follow() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let note = WorldNote {
            text: "too shiny".to_owned(),
            shader: "textures/vjun/newfloor_vjun".to_owned(),
            ..WorldNote::default()
        };
        // Before registering, a note is refused and its picture goes nowhere.
        worker.handle(Command::Note(1, note.clone()), t0);
        worker.handle(Command::NoteImage(1, vec![0; 4]), t0);
        assert!(!lock(&snapshot).note.clone().unwrap().sent);
        assert!(fake.log().is_empty());
        worker.handle(Command::Name("^1Sol".to_owned()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Note(2, note), t0);
        let outcome = lock(&snapshot).note.clone().unwrap();
        assert!(outcome.sent);
        assert_eq!((outcome.serial, outcome.message.as_str()), (2, "note #9"));
        // A picture for an unknown tag is dropped; the note's goes to its hub id once.
        worker.handle(Command::NoteImage(5, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        worker.handle(Command::NoteImage(2, vec![0; 4]), t0);
        let log = fake.log();
        assert_eq!(
            log.iter()
                .filter(|line| line.starts_with("image"))
                .collect::<Vec<_>>(),
            ["image 9 4 bytes"]
        );
        assert!(log.contains(&"note too shiny textures/vjun/newfloor_vjun".to_owned()));
        // The note carries the name the player wears.
        assert!(log.contains(&"note name ^1Sol".to_owned()), "{log:?}");
    }

    #[test]
    fn bug_reports_go_to_the_hub_once_registered() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        let report = BugReport {
            text: "The door flickers".into(),
            ..BugReport::default()
        };
        // Before registering, nothing is sent and the outcome says why.
        worker.handle(Command::Report(report.clone()), t0);
        let first = lock(&snapshot).report.clone().unwrap();
        assert!(!first.sent && first.message.contains("not connected"));
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        worker.handle(Command::Report(report), t0);
        assert!(fake.log().contains(&"report The door flickers".to_owned()));
        let outcome = lock(&snapshot).report.clone().unwrap();
        assert_eq!((outcome.serial, outcome.sent), (first.serial + 1, true));
        assert_eq!(outcome.message, "report #7");
    }

    #[test]
    fn the_worn_name_goes_with_the_registration_and_again_when_it_changes() {
        let fake = Fake::default();
        let t0 = Instant::now();
        let (mut worker, snapshot) = worker(&fake, t0);
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.handle(Command::Configure(on("https://hub")), t0);
        worker.tick(t0);
        assert_eq!(fake.log(), ["register ^1Sol"]);
        assert_eq!(lock(&snapshot).me.as_ref().unwrap().name, "^1Sol");
        // The same name again sends nothing; an empty one is no name.
        worker.handle(Command::Name("^1Sol".into()), t0);
        worker.tick(t0);
        assert_eq!(fake.log().len(), 1);
        worker.handle(Command::Name("Fox".into()), t0);
        let wait = worker.tick(t0);
        assert_eq!(fake.log().last().unwrap(), "register Fox");
        assert_eq!(wait, IDLE_MAX, "nothing else is due");
        // A failure retries after the backoff, not at once.
        fake.fail.store(true, Ordering::SeqCst);
        worker.handle(Command::Name("Wolf".into()), t0);
        worker.tick(t0);
        let count = fake.log().len();
        worker.tick(t0 + Duration::from_secs(1));
        assert_eq!(fake.log().len(), count);
        fake.fail.store(false, Ordering::SeqCst);
        worker.tick(t0 + Duration::from_secs(30));
        assert_eq!(fake.log().last().unwrap(), "register Wolf");
    }
}
