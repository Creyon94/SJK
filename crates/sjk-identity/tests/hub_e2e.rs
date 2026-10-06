//! The real HTTP client against a running hub. Ignored by default: start
//! `sjk-hub serve` on this machine and run
//!
//! ```text
//! SJK_HUB_TEST_URL=http://127.0.0.1:8787 cargo test -p sjk-identity --test hub_e2e -- --ignored
//! ```
//!
//! Every run makes fresh keys and a fresh name, so it can repeat against one
//! database (the hub allows 5 registrations an hour per address, so a run
//! registers two).

use sjk_identity::{HttpHub, Hub, HubError, Identity};

fn hub() -> HttpHub {
    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    HttpHub::new(&url, "sjk-identity-test").unwrap()
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_player_registers_names_themselves_claims_and_leaves() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    let other = Identity::generate().unwrap();
    let suffix = &me.key_id()[..8];
    let name = format!("Test{suffix}");

    let profile = hub.register(&me).unwrap();
    assert_eq!(profile.key_id, me.key_id());
    assert!(!profile.verified);
    assert_eq!(
        hub.register(&me).unwrap(),
        profile,
        "registering again changes nothing"
    );

    let saved = hub.set_profile(&me, &name, "line one\nline two").unwrap();
    assert_eq!(saved.name, name);
    assert_eq!(hub.profile(&me.key_id()).unwrap().bio, "line one\nline two");

    hub.register(&other).unwrap();
    let clash = hub
        .set_profile(&other, &name.to_uppercase(), "")
        .unwrap_err();
    assert!(
        matches!(clash, HubError::Rejected { ref code, .. } if code == "name_taken"),
        "{clash:?}"
    );

    let server = format!("10.99.{}.{}:29070", &suffix[..2].len(), 7);
    hub.claim(&me, &server, 3, "^1Test").unwrap();
    let players = hub.presence(&server).unwrap();
    assert_eq!(players.len(), 1);
    assert_eq!(
        (players[0].slot, players[0].key_id.as_str()),
        (3, me.key_id().as_str())
    );
    assert_eq!(players[0].claimed_name, "^1Test");

    let taken = hub.claim(&other, &server, 3, "^1Test").unwrap_err();
    assert!(
        matches!(taken, HubError::Rejected { ref code, .. } if code == "slot_taken"),
        "{taken:?}"
    );

    hub.release(&me, &server).unwrap();
    assert!(hub.presence(&server).unwrap().is_empty());
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn an_ipv6_server_address_round_trips() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    hub.register(&me).unwrap();
    hub.claim(&me, "[2001:db8::7]:29070", 1, "v6").unwrap();
    assert_eq!(hub.presence("[2001:db8::7]:29070").unwrap().len(), 1);
    hub.release(&me, "[2001:db8::7]:29070").unwrap();
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_missing_player_is_a_rejection_not_a_crash() {
    let mut hub = hub();
    let error = hub.profile("0000000000000000").unwrap_err();
    assert!(
        matches!(error, HubError::Rejected { status: 404, .. }),
        "{error:?}"
    );
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn the_service_registers_claims_tags_and_releases_on_shutdown() {
    use sjk_identity::{Location, Service, Settings, Status};
    use std::time::{Duration, Instant};

    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    let make: sjk_identity::HubFactory = Box::new(|url| {
        HttpHub::new(url, "sjk-identity-test").map(|hub| Box::new(hub) as Box<dyn Hub>)
    });
    let me = Identity::generate().unwrap();
    let key_id = me.key_id();
    let service = Service::start(me, make);
    service.configure(Settings {
        enabled: true,
        hub_url: url.clone(),
    });
    let server: std::net::SocketAddr = "10.98.0.7:29070".parse().unwrap();
    service.enter(Location {
        server,
        slot: 5,
        name: "^2Svc".to_owned(),
    });

    let wait_for = |what: &str, done: &dyn Fn(&sjk_identity::Snapshot) -> bool| {
        let until = Instant::now() + Duration::from_secs(10);
        while Instant::now() < until {
            if service.with_snapshot(|snapshot| done(snapshot)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("timed out waiting for {what}: {:?}", service.snapshot());
    };
    wait_for("online", &|s| s.status == Status::Online);
    wait_for("itself in the roster", &|s| {
        s.badge(5, "svc")
            .is_some_and(|player| player.key_id == key_id)
    });
    assert!(service.snapshot().badge(5, "Someone else").is_none());
    assert!(service.snapshot().badge(6, "Svc").is_none());

    // Another client sees the claim at the hub until the service shuts down.
    let mut watcher = HttpHub::new(&url, "sjk-identity-test").unwrap();
    assert_eq!(watcher.presence(&server.to_string()).unwrap().len(), 1);
    service.shutdown(Duration::from_secs(5));
    assert!(watcher.presence(&server.to_string()).unwrap().is_empty());
}
