//! A refresh or a sync on a thread of its own, one at a time.
//!
//! The client starts a [`Job`] and reads [`Job::progress`] each frame it draws
//! the card; nothing here blocks the caller.

use crate::download::{self, FetchError};
use crate::plan::{Plan, Selection};
use crate::source::Source;
use crate::store::{Snapshot, Store};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Where and from what a job works.
#[derive(Clone, Debug)]
pub struct Settings {
    /// SJK's JoF folder; files go in its `base`.
    pub root: PathBuf,
    /// The game's `base`, read to find files JoF's launcher put there.
    pub game_base: Option<PathBuf>,
    pub sources: Vec<Source>,
    /// The client's version, for the hosts' logs.
    pub version: String,
}

/// What the job is doing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Phase {
    #[default]
    Idle,
    /// Asking the hosts for the manifest.
    Manifest,
    /// Hashing files already on disk.
    Checking {
        done: u64,
        total: u64,
    },
    Removing {
        done: usize,
        total: usize,
    },
    Downloading {
        /// 1-based position of the file in the plan.
        file: usize,
        files: usize,
        name: String,
        file_done: u64,
        file_size: u64,
        /// Over the whole plan.
        done: u64,
        total: u64,
    },
    Finished(Outcome),
    Failed(String),
    Cancelled,
}

impl Phase {
    pub fn running(&self) -> bool {
        !matches!(
            self,
            Self::Idle | Self::Finished(_) | Self::Failed(_) | Self::Cancelled
        )
    }
}

/// How a job ended.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// True for a sync, false for a refresh.
    pub synced: bool,
    pub downloaded: usize,
    pub downloaded_bytes: u64,
    pub removed: usize,
    /// Files that could not be fetched or removed, with why.
    pub failures: Vec<String>,
    /// The host that gave the manifest.
    pub source: &'static str,
}

/// The latest state, shared with the job's thread.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub phase: Phase,
    /// The latest look at the disk.
    pub snapshot: Option<Arc<Snapshot>>,
}

/// One running or finished job.
#[derive(Default)]
pub struct Job {
    shared: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    /// Raised on every change, so the client redraws only then.
    revision: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
}

impl Job {
    /// Fetch the manifest and measure the disk.
    pub fn refresh(&mut self, settings: Settings) -> bool {
        self.start(settings, None)
    }

    /// Refresh, then fetch and remove what `selection` asks.
    pub fn sync(&mut self, settings: Settings, selection: Selection) -> bool {
        self.start(settings, Some(selection))
    }

    /// Whether a job's thread is still at work (a stopped one may take a
    /// moment to notice).
    pub fn busy(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    /// Ask the running job to stop; a `.part` is kept to continue later.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn progress(&self) -> Progress {
        self.shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    fn start(&mut self, settings: Settings, selection: Option<Selection>) -> bool {
        if self.busy() {
            return false;
        }
        self.cancel = Arc::default();
        let worker = Worker {
            shared: Arc::clone(&self.shared),
            cancel: Arc::clone(&self.cancel),
            revision: Arc::clone(&self.revision),
        };
        worker.set(Phase::Manifest);
        self.thread = std::thread::Builder::new()
            .name("jof-content".into())
            .spawn(move || worker.run(settings, selection))
            .ok();
        self.thread.is_some()
    }
}

struct Worker {
    shared: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
    revision: Arc<AtomicU64>,
}

impl Worker {
    fn lock(&self) -> std::sync::MutexGuard<'_, Progress> {
        self.shared
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn set(&self, phase: Phase) {
        self.lock().phase = phase;
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    fn publish(&self, snapshot: Snapshot) -> Arc<Snapshot> {
        let snapshot = Arc::new(snapshot);
        self.lock().snapshot = Some(Arc::clone(&snapshot));
        self.revision.fetch_add(1, Ordering::Relaxed);
        snapshot
    }

    fn cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }

    fn run(self, settings: Settings, selection: Option<Selection>) {
        let phase = match self.work(&settings, selection) {
            Ok(outcome) => Phase::Finished(outcome),
            Err(FetchError::Cancelled) => Phase::Cancelled,
            Err(FetchError::Failed(reason)) => Phase::Failed(reason),
        };
        self.set(phase);
    }

    fn work(
        &self,
        settings: &Settings,
        selection: Option<Selection>,
    ) -> Result<Outcome, FetchError> {
        let agent = download::agent(&settings.version);
        let (manifest, source) = download::manifest(&agent, &settings.sources)?;
        let mut store = Store::open(&settings.root, settings.game_base.clone());
        if store.is_game_folder() {
            return Err(FetchError::Failed(
                "the JoF folder is the game's own folder; set jof.contentFolder to another one"
                    .into(),
            ));
        }
        let snapshot = self.scan(&mut store, manifest)?;
        let mut outcome = Outcome {
            synced: selection.is_some(),
            source: source.name,
            ..Outcome::default()
        };
        let Some(selection) = selection else {
            return Ok(outcome);
        };
        let plan = Plan::new(&snapshot, &selection);
        let content = store.content();
        for (index, name) in plan.removals.iter().enumerate() {
            self.set(Phase::Removing {
                done: index,
                total: plan.removals.len(),
            });
            let path = content.join(name);
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    store.forget(&path);
                    outcome.removed += 1;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => outcome
                    .failures
                    .push(format!("{name}: cannot remove it ({error})")),
            }
        }
        let mut done = 0;
        for (index, &(group, asset)) in plan.downloads.iter().enumerate() {
            let asset = &snapshot.manifest.groups[group].assets[asset];
            let mut report = |file_done: u64| {
                self.set(Phase::Downloading {
                    file: index + 1,
                    files: plan.downloads.len(),
                    name: asset.name.clone(),
                    file_done,
                    file_size: asset.size,
                    done: done + file_done,
                    total: plan.download_bytes,
                })
            };
            report(0);
            match download::file(
                &agent,
                &settings.sources,
                asset,
                &content,
                &self.cancel,
                &mut report,
            ) {
                Ok(path) => {
                    store.remember(&path, &asset.md5);
                    store.save();
                    outcome.downloaded += 1;
                    outcome.downloaded_bytes += asset.size;
                }
                Err(FetchError::Cancelled) => {
                    store.save();
                    self.rescan(&mut store, &snapshot);
                    return Err(FetchError::Cancelled);
                }
                Err(FetchError::Failed(reason)) => outcome.failures.push(reason),
            }
            done += asset.size;
        }
        store.save();
        self.rescan(&mut store, &snapshot);
        Ok(outcome)
    }

    fn scan(
        &self,
        store: &mut Store,
        manifest: crate::Manifest,
    ) -> Result<Arc<Snapshot>, FetchError> {
        let snapshot = store.scan(manifest, &self.cancel, &mut |done, total| {
            self.set(Phase::Checking { done, total })
        });
        if self.cancelled() {
            return Err(FetchError::Cancelled);
        }
        Ok(self.publish(snapshot))
    }

    /// Measure again after a sync; every file written is in the record, so
    /// this hashes nothing new.
    fn rescan(&self, store: &mut Store, snapshot: &Snapshot) {
        let calm = AtomicBool::new(false);
        let fresh = store.scan(snapshot.manifest.clone(), &calm, &mut |_, _| {});
        self.publish(fresh);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::download::tests::serve;

    #[test]
    fn a_sync_fetches_the_chosen_group_and_reports_it() {
        use md5::{Digest, Md5};
        const BODY: &[u8] = b"jof";
        let digest: [u8; 16] = Md5::digest(BODY).into();
        let manifest: &'static str = Box::leak(
            format!(
                r#"{{"data":{{"assetgroups":[{{"id":1,"name":"One","type":"RECOMMENDED","assets":[
                {{"id":5,"name":"m.pk3","size":3,"hash":"{}","targetfolder":"Base","localfolder":"assets"}}]}}]}}}}"#,
                crate::md5_hex(&digest)
            )
            .into_boxed_str(),
        );
        let (manifest_address, manifest_server) = serve(manifest.as_bytes(), 1, false);
        let (file_address, file_server) = serve(BODY, 1, false);
        let directory = tempfile::tempdir().unwrap();
        let source = Source {
            name: "test",
            manifest: Box::leak(format!("{manifest_address}/manifest").into_boxed_str()),
            files: Box::leak(format!("{file_address}/repo").into_boxed_str()),
        };
        let mut job = Job::default();
        assert!(job.sync(
            Settings {
                root: directory.path().join("jof"),
                game_base: None,
                sources: vec![source],
                version: "test".into(),
            },
            Selection::default(),
        ));
        while job.busy() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let progress = job.progress();
        let Phase::Finished(outcome) = progress.phase else {
            panic!("{:?}", progress.phase)
        };
        assert_eq!((outcome.downloaded, outcome.removed), (1, 0));
        assert!(outcome.failures.is_empty());
        assert_eq!(
            std::fs::read(directory.path().join("jof/base/m.pk3")).unwrap(),
            BODY
        );
        assert!(progress.snapshot.unwrap().state(0, 0) == crate::AssetState::Installed);
        manifest_server.join().unwrap();
        file_server.join().unwrap();
    }
}
