use crate::jar_importer::{JarImportOptions, JarImportReport, import_mod_archive_file};
use crate::mc_model::{
    AssetSource, DirAssetSource, ResolvedMcAsset, build_local_visual_index_from_dir,
    load_visual_index_file, resolve_visual_entry_asset,
};
use crate::model::{FactoryProject, SolverResult, VisualEntrySummary};
use crate::nbt_parser::{ParsedNbtSchematic, parse_nbt_schematic_bytes};
use crate::solver::{SolverCatalog, solve_factory_project};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

pub type WakeupFn = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone)]
pub enum BackgroundJob {
    SolveProject {
        project: FactoryProject,
    },
    LoadVisualIndex {
        index_json_path: Option<PathBuf>,
        scan_dirs: Vec<PathBuf>,
    },
    ResolveMcAsset {
        entry: VisualEntrySummary,
        asset_roots: Vec<PathBuf>,
    },
    ParseNbtFile {
        path: PathBuf,
    },
    ImportModArchive {
        path: PathBuf,
        options: JarImportOptions,
    },
}

#[derive(Debug, Clone)]
pub enum BackgroundOutput {
    Progress {
        task_id: u64,
        done: usize,
        total: usize,
        message: String,
    },
    Solved {
        task_id: u64,
        result: Box<SolverResult>,
    },
    VisualIndexReady {
        task_id: u64,
        entries: Vec<VisualEntrySummary>,
        source_label: String,
    },
    McAssetReady {
        task_id: u64,
        entry_id: String,
        asset: Box<ResolvedMcAsset>,
    },
    NbtParsed {
        task_id: u64,
        path: PathBuf,
        schematic: Box<ParsedNbtSchematic>,
    },
    ModArchiveImported {
        task_id: u64,
        report: Box<JarImportReport>,
    },
    Failed {
        task_id: u64,
        error: String,
    },
}

struct QueuedTask {
    task_id: u64,
    cancel: Arc<AtomicBool>,
    job: BackgroundJob,
}

struct SharedQueue {
    tasks: VecDeque<QueuedTask>,
    shutdown: bool,
}

struct CombinedAssetSource {
    roots: Vec<DirAssetSource>,
}

impl AssetSource for CombinedAssetSource {
    fn read_json(&self, rel_path: &str) -> Option<serde_json::Value> {
        for r in &self.roots {
            if let Some(v) = r.read_json(rel_path) {
                return Some(v);
            }
        }
        None
    }

    fn exists(&self, rel_path: &str) -> bool {
        self.roots.iter().any(|r| r.exists(rel_path))
    }
}

/// Bounded Rust worker pool for all heavy work (solver, .nbt parsing, .jar import, local visual indexing).
pub struct WorkerPool {
    queue: Arc<(Mutex<SharedQueue>, Condvar)>,
    out_tx: Arc<Mutex<VecDeque<BackgroundOutput>>>,
    next_id: AtomicU64,
    active_cancels: Arc<Mutex<Vec<(u64, Arc<AtomicBool>)>>>,
    wakeup: Arc<Mutex<Option<WakeupFn>>>,
    threads: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    pub fn new(requested_workers: Option<usize>) -> Self {
        let workers = requested_workers.unwrap_or_else(|| {
            thread::available_parallelism()
                .map(|n| n.get().clamp(2, 4))
                .unwrap_or(2)
        });
        let queue = Arc::new((
            Mutex::new(SharedQueue {
                tasks: VecDeque::new(),
                shutdown: false,
            }),
            Condvar::new(),
        ));
        let out_tx = Arc::new(Mutex::new(VecDeque::with_capacity(64)));
        let active_cancels = Arc::new(Mutex::new(Vec::new()));
        let wakeup: Arc<Mutex<Option<WakeupFn>>> = Arc::new(Mutex::new(None));

        let mut threads = Vec::with_capacity(workers);
        for idx in 0..workers {
            let q = Arc::clone(&queue);
            let out = Arc::clone(&out_tx);
            let cancels = Arc::clone(&active_cancels);
            let wake = Arc::clone(&wakeup);

            let handle = thread::Builder::new()
                .name(format!("fa-worker-{idx}"))
                .spawn(move || {
                    loop {
                        let task = {
                            let (lock, cv) = &*q;
                            let mut guard = match lock.lock() {
                                Ok(g) => g,
                                Err(_) => return,
                            };
                            while guard.tasks.is_empty() && !guard.shutdown {
                                guard = match cv.wait(guard) {
                                    Ok(g) => g,
                                    Err(_) => return,
                                };
                            }
                            if guard.shutdown && guard.tasks.is_empty() {
                                return;
                            }
                            guard.tasks.pop_front()
                        };

                        let Some(task) = task else {
                            continue;
                        };
                        if task.cancel.load(Ordering::Relaxed) {
                            continue;
                        }

                        let emit = |msg: BackgroundOutput| {
                            if let Ok(mut o) = out.lock() {
                                o.push_back(msg);
                            }
                            if let Ok(w) = wake.lock()
                                && let Some(cb) = w.as_ref()
                            {
                                cb();
                            }
                        };

                        match task.job {
                            BackgroundJob::SolveProject { project } => {
                                let catalog = SolverCatalog::from_project(&project);
                                let res = solve_factory_project(&project, &catalog, None);
                                if !task.cancel.load(Ordering::Relaxed) {
                                    emit(BackgroundOutput::Solved {
                                        task_id: task.task_id,
                                        result: Box::new(res),
                                    });
                                }
                            }
                            BackgroundJob::LoadVisualIndex {
                                index_json_path,
                                scan_dirs,
                            } => {
                                let mut entries = index_json_path
                                    .as_deref()
                                    .map(load_visual_index_file)
                                    .unwrap_or_default();
                                let mut source_label = if entries.is_empty() {
                                    "no-local-index".to_owned()
                                } else {
                                    "visual-index.json".to_owned()
                                };
                                for dir in scan_dirs {
                                    if task.cancel.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    let scanned = build_local_visual_index_from_dir(&dir);
                                    if !scanned.is_empty() {
                                        source_label = dir.display().to_string();
                                        let mut by_id: std::collections::HashMap<String, VisualEntrySummary> =
                                            entries.into_iter().map(|e| (e.id.clone(), e)).collect();
                                        for e in scanned {
                                            by_id.insert(e.id.clone(), e);
                                        }
                                        entries = by_id.into_values().collect();
                                        entries.sort_by(|a, b| a.id.cmp(&b.id));
                                    }
                                }
                                if !task.cancel.load(Ordering::Relaxed) {
                                    emit(BackgroundOutput::VisualIndexReady {
                                        task_id: task.task_id,
                                        entries,
                                        source_label,
                                    });
                                }
                            }
                            BackgroundJob::ResolveMcAsset { entry, asset_roots } => {
                                let src = CombinedAssetSource {
                                    roots: asset_roots
                                        .into_iter()
                                        .map(|p| DirAssetSource { root_dir: p })
                                        .collect(),
                                };
                                let asset = resolve_visual_entry_asset(&entry, &src);
                                if !task.cancel.load(Ordering::Relaxed) {
                                    emit(BackgroundOutput::McAssetReady {
                                        task_id: task.task_id,
                                        entry_id: entry.id,
                                        asset: Box::new(asset),
                                    });
                                }
                            }
                            BackgroundJob::ParseNbtFile { path } => match fs::read(&path) {
                                Ok(bytes) => match parse_nbt_schematic_bytes(&bytes) {
                                    Ok(sch) => emit(BackgroundOutput::NbtParsed {
                                        task_id: task.task_id,
                                        path,
                                        schematic: Box::new(sch),
                                    }),
                                    Err(err) => emit(BackgroundOutput::Failed {
                                        task_id: task.task_id,
                                        error: err,
                                    }),
                                },
                                Err(err) => emit(BackgroundOutput::Failed {
                                    task_id: task.task_id,
                                    error: format!("Cannot read {}: {err}", path.display()),
                                }),
                            },
                            BackgroundJob::ImportModArchive { path, options } => {
                                let tid = task.task_id;
                                match import_mod_archive_file(
                                    &path,
                                    &options,
                                    Some(task.cancel.as_ref()),
                                    |done, total, msg| {
                                        emit(BackgroundOutput::Progress {
                                            task_id: tid,
                                            done,
                                            total,
                                            message: msg.to_owned(),
                                        });
                                    },
                                ) {
                                    Ok(report) => emit(BackgroundOutput::ModArchiveImported {
                                        task_id: tid,
                                        report: Box::new(report),
                                    }),
                                    Err(err) => emit(BackgroundOutput::Failed {
                                        task_id: tid,
                                        error: err,
                                    }),
                                }
                            }
                        }

                        if let Ok(mut list) = cancels.lock() {
                            list.retain(|(id, _)| *id != task.task_id);
                        }
                    }
                })
                .expect("failed to spawn worker thread");
            threads.push(handle);
        }

        Self {
            queue,
            out_tx,
            next_id: AtomicU64::new(1),
            active_cancels,
            wakeup,
            threads,
        }
    }

    pub fn set_wakeup(&self, cb: WakeupFn) {
        if let Ok(mut guard) = self.wakeup.lock() {
            *guard = Some(cb);
        }
    }

    pub fn worker_count(&self) -> usize {
        self.threads.len()
    }

    pub fn submit(&self, job: BackgroundJob) -> u64 {
        let task_id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let cancel = Arc::new(AtomicBool::new(false));
        if let Ok(mut c) = self.active_cancels.lock() {
            c.push((task_id, Arc::clone(&cancel)));
        }
        let (lock, cv) = &*self.queue;
        if let Ok(mut q) = lock.lock() {
            q.tasks.push_back(QueuedTask {
                task_id,
                cancel,
                job,
            });
            cv.notify_one();
        }
        task_id
    }

    pub fn cancel_task(&self, task_id: u64) {
        if let Ok(list) = self.active_cancels.lock() {
            for (id, flag) in list.iter() {
                if *id == task_id {
                    flag.store(true, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn cancel_all(&self) {
        if let Ok(list) = self.active_cancels.lock() {
            for (_, flag) in list.iter() {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }

    pub fn poll_outputs(&self) -> Vec<BackgroundOutput> {
        let mut drained = Vec::new();
        if let Ok(mut guard) = self.out_tx.lock() {
            while let Some(item) = guard.pop_front() {
                drained.push(item);
            }
        }
        drained
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        self.cancel_all();
        let (lock, cv) = &*self.queue;
        if let Ok(mut q) = lock.lock() {
            q.shutdown = true;
            cv.notify_all();
        }
        for h in self.threads.drain(..) {
            let _ = h.join();
        }
    }
}
