//! The real HTTP client against a running hub. Ignored by default: start
//! `sjk-hub serve` on this machine and run
//!
//! ```text
//! SJK_HUB_TEST_URL=http://127.0.0.1:8787 cargo test -p sjk-identity --test hub_e2e -- --ignored
//! ```
//!
//! Every run makes fresh keys and a fresh name, so it can repeat against one
//! database. The hub allows 5 registrations an hour per address and a whole run
//! makes more: run a few tests at a time (name them after `--ignored`), restarting
//! the hub in between (its limits live in memory).

use sjk_identity::{HttpHub, Hub, HubError, Identity};

fn hub() -> HttpHub {
    let url = std::env::var("SJK_HUB_TEST_URL").expect("SJK_HUB_TEST_URL");
    HttpHub::new(&url, "sjk-identity-test").unwrap()
}

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_player_registers_with_the_name_they_wear_claims_and_leaves() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    let other = Identity::generate().unwrap();
    let suffix = &me.key_id()[..8];
    let name = format!("^1Test{suffix}");

    let profile = hub.register(&me, Some(&name)).unwrap();
    assert_eq!(profile.key_id, me.key_id());
    assert!(!profile.verified);
    assert_eq!(profile.name, name, "the worn name is the display name");
    assert_eq!(profile.names[0].name, name);
    let again = hub.register(&me, None).unwrap();
    assert_eq!(
        (again.created, again.name.as_str()),
        (profile.created, name.as_str()),
        "registering again keeps the profile"
    );

    let saved = hub.set_bio(&me, "line one\nline two").unwrap();
    assert_eq!(saved.name, name, "a bio change keeps the name");
    assert_eq!(hub.profile(&me.key_id()).unwrap().bio, "line one\nline two");

    // Two keys may wear the same name: the name proves nothing, the key does.
    hub.register(&other, Some(&name)).unwrap();

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
    hub.register(&me, None).unwrap();
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

#[test]
#[ignore = "needs a running hub (SJK_HUB_TEST_URL)"]
fn a_world_note_and_its_picture_reach_the_hub() {
    let mut hub = hub();
    let me = Identity::generate().unwrap();
    hub.register(&me, Some("NoteTester")).unwrap();
    let note = sjk_identity::WorldNote {
        text: format!("too shiny {}", &me.key_id()[..6]),
        map: "maps/mp/ffa1.bsp".to_owned(),
        build: "test".to_owned(),
        view: Some([2807.0, 726.0, 872.0, 283.0]),
        hit: Some([2900.0, 700.0, 860.5]),
        normal: Some([0.0, 0.0, 1.0]),
        shader: "textures/vjun/newfloor_vjun".to_owned(),
        surface: Some(943),
        lighting: "lightmapped".to_owned(),
        distance: Some(252.0),
        ..sjk_identity::WorldNote::default()
    };
    let id = hub.note(&me, &note).unwrap();
    assert!(id > 0);
    // The markers of a minimal 2 x 2 JPEG: the hub checks the frame, not the pixels.
    let jpeg = [
        0xFF, 0xD8, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11, 0x00,
        0xFF, 0xDA, 0x00, 0x02, 0xFF, 0xD9,
    ];
    hub.note_image(&me, id, &jpeg).unwrap();
    let again = hub.note_image(&me, id, &jpeg).unwrap_err();
    assert!(
        matches!(again, HubError::Rejected { ref code, .. } if code == "image_taken"),
        "{again:?}"
    );
}
