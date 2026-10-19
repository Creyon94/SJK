//! Test packs on a chosen player's PC (`PROTOCOL.md`, "Test packs"). The identity worker
//! keeps each pack's locked body in a folder beside `identity.key` ([`FOLDER`]), as
//! `<id>.body`, downloaded in parts (`<id>.part`, resumed after a break) and checked
//! against the list's length and SHA-256 before it takes its name. Nothing else is ever
//! written: the envelope and label are opened in memory each time the list is read, and
//! the pack's key lives only in [`Opened`], which the viewer mounts the body through. A
//! body the list no longer shows is deleted with its part, so a pack the owner took
//! away cannot be opened again.

use crate::hub::{Hub, HubError};
use crate::keys::Identity;
use crate::test_packs::{self, BodyReader, Listed, TestPackError};
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The folder's name, in the settings folder beside `identity.key`.
pub const FOLDER: &str = "test_packs";
/// The bytes one download request asks for: well under the hub's [`test_packs::RANGE_MAX`], so a
/// slow connection finishes each one within the request's two minutes.
const RANGE: u64 = 8 * 1024 * 1024;
/// The most bytes one check downloads, so the worker goes back to its other work; a
/// pack left unfinished is taken up again a moment later.
const SLICE: u64 = 64 * 1024 * 1024;

/// A test pack ready to mount: its locked body on this PC and, in memory only, the key
/// its envelope held.
#[derive(Clone, Eq, PartialEq)]
pub struct Opened {
    /// The pack's id, its body's file name.
    pub id: String,
    /// The file name the owner gave it ([`test_packs::valid_file`]), what it mounts as.
    pub file: String,
    /// The owner's note.
    pub note: String,
    /// The locked body.
    pub body: PathBuf,
    key: [u8; 32],
}

impl fmt::Debug for Opened {
    // Never prints the key.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Opened({} as {})", self.id, self.file)
    }
}

impl Opened {
    /// The plain PK3, decrypted as it is read.
    pub fn reader(&self) -> Result<BodyReader<File>, TestPackError> {
        BodyReader::open(File::open(&self.body)?, &self.key, &self.id)
    }
}

/// What one check of the test packs did.
#[derive(Debug, Default)]
pub(crate) struct Refresh {
    /// The packs ready, when the list was read; `None` keeps what was there before.
    pub(crate) opened: Option<Vec<Opened>>,
    /// How many packs the list showed.
    pub(crate) listed: usize,
    /// Bytes downloaded this time.
    pub(crate) downloaded: u64,
    /// A download left unfinished, to take up again in a moment.
    pub(crate) more: bool,
    /// Bodies and parts deleted because their pack left the list.
    pub(crate) removed: usize,
    /// What fell short, if anything.
    pub(crate) problem: Option<String>,
}

impl Refresh {
    /// A line for the log.
    pub(crate) fn summary(&self) -> String {
        let ready = self.opened.as_ref().map_or(0, Vec::len);
        let mut text = format!("test packs: {ready} ready of {} listed", self.listed);
        if self.downloaded > 0 {
            text.push_str(&format!(", {} MiB downloaded", self.downloaded >> 20));
        }
        if self.removed > 0 {
            text.push_str(&format!(", {} removed", self.removed));
        }
        if let Some(problem) = &self.problem {
            text.push_str("; ");
            text.push_str(problem);
        }
        text
    }
}

fn body_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.body"))
}

fn part_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.part"))
}

/// Read the list, open each listed pack's envelope and label, download what is missing
/// (at most [`SLICE`] bytes) and delete what left the list.
pub(crate) fn refresh(hub: &mut dyn Hub, identity: &Identity, dir: &Path) -> Refresh {
    let mut refresh = Refresh::default();
    let listed = match hub.test_packs(identity) {
        Ok(listed) => listed,
        Err(error) => {
            refresh.problem = Some(format!("cannot read the list: {error}"));
            return refresh;
        }
    };
    refresh.listed = listed.len();
    remove_unlisted(dir, &listed, &mut refresh);
    let mut opened = Vec::new();
    let mut budget = SLICE;
    for pack in &listed {
        let (key, label) = match open(identity, pack) {
            Ok(open) => open,
            Err(why) => {
                refresh.problem = Some(format!("pack {}: {why}", pack.id));
                continue;
            }
        };
        let body = body_path(dir, &pack.id);
        let ready = fs::metadata(&body).is_ok_and(|meta| meta.len() == pack.size);
        if !ready {
            if budget == 0 {
                refresh.more = true;
                continue;
            }
            match download(hub, identity, dir, pack, &mut budget, &mut refresh) {
                Ok(true) => {}
                Ok(false) => {
                    refresh.more = true;
                    continue;
                }
                Err(why) => {
                    refresh.problem = Some(format!("pack {}: {why}", pack.id));
                    continue;
                }
            }
        }
        opened.push(Opened {
            id: pack.id.clone(),
            file: label.file,
            note: label.note,
            body,
            key,
        });
    }
    refresh.opened = Some(opened);
    refresh
}

/// The pack's key and label, opened in memory.
fn open(identity: &Identity, pack: &Listed) -> Result<([u8; 32], test_packs::Label), String> {
    let key = test_packs::open_envelope(identity, &pack.id, &pack.envelope)
        .map_err(|error| error.to_string())?;
    let label = test_packs::open_label(&key, &pack.id, &pack.label).map_err(|e| e.to_string())?;
    Ok((key, label))
}

/// Delete the bodies and parts of packs the list no longer shows (and anything else
/// with those endings): only once the list was read.
fn remove_unlisted(dir: &Path, listed: &[Listed], refresh: &mut Refresh) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(id) = name
            .strip_suffix(".body")
            .or_else(|| name.strip_suffix(".part"))
        else {
            continue;
        };
        if !listed.iter().any(|pack| pack.id == id) && fs::remove_file(entry.path()).is_ok() {
            refresh.removed += 1;
        }
    }
}

/// Go on with `pack`'s download into its part, spending at most `budget` bytes; `true`
/// once the body is complete, checked and named.
fn download(
    hub: &mut dyn Hub,
    identity: &Identity,
    dir: &Path,
    pack: &Listed,
    budget: &mut u64,
    refresh: &mut Refresh,
) -> Result<bool, String> {
    fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let part = part_path(dir, &pack.id);
    let mut have = fs::metadata(&part).map_or(0, |meta| meta.len());
    if have > pack.size {
        fs::remove_file(&part).map_err(|error| error.to_string())?;
        have = 0;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&part)
        .map_err(|error| error.to_string())?;
    while have < pack.size && *budget > 0 {
        let length = (pack.size - have).min(RANGE).min(*budget);
        let bytes = hub
            .test_pack_range(identity, &pack.id, have, have + length - 1)
            .map_err(|error: HubError| error.to_string())?;
        file.write_all(&bytes).map_err(|error| error.to_string())?;
        have += length;
        *budget -= length;
        refresh.downloaded += length;
    }
    if have < pack.size {
        return Ok(false);
    }
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    let sha256 = crate::assets::file_sha256(&part).map_err(|error| error.to_string())?;
    if sha256 != pack.sha256 {
        let _ = fs::remove_file(&part);
        return Err("the download differs from the list's SHA-256; it starts again".to_owned());
    }
    fs::rename(&part, body_path(dir, &pack.id)).map_err(|error| error.to_string())?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_packs::{
        Label, encrypt_body, new_pack_id, new_pack_key, seal_envelope, seal_label,
    };
    use std::io::Read;

    /// A hub holding one body and its listing, counting the bytes it hands out; it
    /// answers nothing else.
    #[derive(Default)]
    struct OneHub {
        listed: Vec<Listed>,
        body: Vec<u8>,
        served: u64,
        offline: bool,
    }

    fn offline<T>() -> Result<T, HubError> {
        Err(HubError::Network("offline".into()))
    }

    impl Hub for OneHub {
        fn register(
            &mut self,
            _: &Identity,
            _: Option<&str>,
        ) -> Result<crate::wire::Profile, HubError> {
            offline()
        }
        fn set_bio(&mut self, _: &Identity, _: &str) -> Result<crate::wire::Profile, HubError> {
            offline()
        }
        fn profile(&mut self, _: &str) -> Result<crate::wire::Profile, HubError> {
            offline()
        }
        fn claim(
            &mut self,
            _: &Identity,
            _: &str,
            _: u8,
            _: &str,
            _: bool,
        ) -> Result<(), HubError> {
            offline()
        }
        fn release(&mut self, _: &Identity, _: &str) -> Result<(), HubError> {
            offline()
        }
        fn presence(&mut self, _: &str) -> Result<Vec<crate::wire::Presence>, HubError> {
            offline()
        }
        fn test_packs(&mut self, _: &Identity) -> Result<Vec<Listed>, HubError> {
            if self.offline {
                return offline();
            }
            Ok(self.listed.clone())
        }
        fn test_pack_range(
            &mut self,
            _: &Identity,
            _: &str,
            first: u64,
            last: u64,
        ) -> Result<Vec<u8>, HubError> {
            self.served += last - first + 1;
            Ok(self.body[first as usize..=last as usize].to_vec())
        }
    }

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("sjk-test-packs-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn a_listed_pack_downloads_in_slices_opens_and_goes_when_unlisted() {
        let tester = Identity::from_seed([5; 32]);
        let id = new_pack_id().unwrap();
        let key = new_pack_key().unwrap();
        let plain: Vec<u8> = (0..(3 * test_packs::RANGE_MAX as usize + 11))
            .map(|i| (i % 253) as u8)
            .collect();
        let mut body = Vec::new();
        encrypt_body(&mut plain.as_slice(), &mut body, &key, &id).unwrap();
        let label = Label {
            file: "anim_test_2.pk3".to_owned(),
            note: String::new(),
        };
        let listed = Listed {
            id: id.clone(),
            size: body.len() as u64,
            sha256: crate::assets::sha256_hex(&body),
            label: seal_label(&key, &id, &label).unwrap(),
            envelope: seal_envelope(&key, &id, &tester.public_key()).unwrap(),
        };
        let mut hub = OneHub {
            listed: vec![listed],
            body,
            ..OneHub::default()
        };
        let dir = folder("slices");
        // Over two slices: unfinished, then done; nothing but the body stays.
        let first = refresh(&mut hub, &tester, &dir);
        assert!(first.more && first.opened.as_ref().unwrap().is_empty());
        let second = refresh(&mut hub, &tester, &dir);
        let opened = second.opened.unwrap();
        assert_eq!(opened.len(), 1);
        assert_eq!(opened[0].file, "anim_test_2.pk3");
        let names: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(names, vec![format!("{id}.body")]);
        let mut back = Vec::new();
        opened[0].reader().unwrap().read_to_end(&mut back).unwrap();
        assert_eq!(back, plain);
        // Ready now: nothing more is downloaded.
        let served = hub.served;
        assert_eq!(refresh(&mut hub, &tester, &dir).opened.unwrap().len(), 1);
        assert_eq!(hub.served, served);
        // Another key cannot open its envelope; the owner takes it away: the body goes.
        let other = Identity::from_seed([6; 32]);
        let refused = refresh(&mut hub, &other, &dir);
        assert!(refused.opened.unwrap().is_empty() && refused.problem.is_some());
        hub.listed.clear();
        let gone = refresh(&mut hub, &tester, &dir);
        assert_eq!(gone.removed, 1);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unreadable_list_keeps_what_was_there() {
        let mut hub = OneHub {
            offline: true,
            ..OneHub::default()
        };
        let refresh = refresh(&mut hub, &Identity::from_seed([1; 32]), &folder("none"));
        assert!(refresh.opened.is_none() && refresh.problem.is_some());
    }
}
