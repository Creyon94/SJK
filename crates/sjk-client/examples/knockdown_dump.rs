// Throwaway: dump the recorder's own state around JA+ view locks / knockdowns.
use sjk_client::DemoPlayback;
fn main() {
    for path in std::env::args().skip(1) {
        let file = std::io::BufReader::new(std::fs::File::open(&path).unwrap());
        let mut demo = DemoPlayback::new(file).unwrap();
        println!("== {path}");
        let mut last_print = false;
        let mut prev_delta = [0i32; 3];
        loop {
            match demo.next_snapshot() {
                Ok(Some(_)) => {}
                Ok(None) => break,
                Err(e) => {
                    println!("err {e}");
                    break;
                }
            }
            let Some(snap) = demo.latest_snapshot() else {
                continue;
            };
            let p = &snap.player;
            let name = |a: u16| sjk_game_jka::legacy_animation_name(usize::from(a)).unwrap_or("?");
            let locked = sjk_game_jka::prediction_policy::japlus_view_locked(p);
            let kick = sjk_game_jka::prediction_policy::japlus_special_kick(p);
            let d = p.delta_angles();
            if locked || kick || last_print {
                println!(
                    "t={} ct={} legs={} torso={}({}) hand={} lock={} kick={} delta={:?} d_delta={:?} view={:?}",
                    snap.server_time,
                    p.command_time(),
                    name(p.leg_animation()),
                    name(p.torso_animation()),
                    p.torso_timer(),
                    p.force_hand_extend(),
                    locked,
                    kick,
                    d,
                    [
                        (d[0].wrapping_sub(prev_delta[0])) as i16,
                        (d[1].wrapping_sub(prev_delta[1])) as i16,
                        0
                    ],
                    p.view_angles().map(|a| a.round())
                );
            }
            last_print = locked || kick;
            if snap.server_time % 60000 < 60 {
                println!(
                    "t={} legs={} hand={}",
                    snap.server_time,
                    name(p.leg_animation()),
                    p.force_hand_extend()
                );
            }
            prev_delta = d;
        }
    }
}
