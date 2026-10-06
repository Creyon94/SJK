//! The background service that keeps the player registered with the hub, repeats
//! their game-server claim while they play, and reads who else on that server is
//! known. Nothing here blocks a frame: the viewer sends [`Command`]s through
//! [`Service`] and reads a [`Snapshot`].
//!
//! The service is inert while it is disabled or has no hub address: it makes no
//! request of any kind.

use crate::hub::{Hub, HubError};
use crate::keys::Identity;
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
    Stop,
}

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

        fn note(&self, line: String) -> Result<(), HubError> {
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
                    .note(format!("register {name}"))
                    .map(|()| profile(name)),
                None => self.note("register".to_owned()).map(|()| profile("")),
            }
        }
        fn set_bio(&mut self, _: &Identity, bio: &str) -> Result<Profile, HubError> {
            self.note(format!("bio {bio}")).map(|()| profile(""))
        }
        fn profile(&mut self, key_id: &str) -> Result<Profile, HubError> {
            self.note(format!("lookup {key_id}"))
                .map(|()| profile("Other"))
        }
        fn claim(
            &mut self,
            _: &Identity,
            server: &str,
            slot: u8,
            name: &str,
        ) -> Result<(), HubError> {
            self.note(format!("claim {server} {slot} {name}"))
        }
        fn release(&mut self, _: &Identity, server: &str) -> Result<(), HubError> {
            self.note(format!("release {server}"))
        }
        fn presence(&mut self, server: &str) -> Result<Vec<Presence>, HubError> {
            self.note(format!("presence {server}"))
                .map(|()| self.roster.lock().unwrap().clone())
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
