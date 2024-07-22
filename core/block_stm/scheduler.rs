// NOTES:
// 1. There is a chance that we can pack the (incarnation_number, status) inside an atomic variable
//    instead of using a `Mutex`.
// 2. We can probably store the state to restore partial executions (this is suggested in the
//    paper).
// 3. Mermory ordering needs to be reviewed. To play it safe, I'm using SeqCst, which provides
//    stricter ordering, but it comes with some overhead.
// 4. Maybe `txn_status` can become a `Vec<AtomicUsize>` with some work and see if that helps
//    reduce overhead.
#[derive(Default)]
enum TaskStatus {
    Ready,
    Executing,
    Executed,
    Aborting,
}

enum TaskKind {
    Execution,
    Validation,
}

#[derive(default)]
struct SchedulerInner {
    // Index tracking the next transaction to try and execute.
    pub execution_idx: AtomicUsize,
    // Index tracking the next transaction to try and validate.
    pub validation_idx: AtomicUsize,
    // Number of times validation_idx or execution_idx was decreased.
    pub decrease_cnt: AtomicUsize,
    // Number of ongoing validation and execution tasks.
    pub num_active_tasks: AtomicUsize,
    // Marker for completion.
    pub done_marker: AtomicBool,
    // Maps transaction index to a set of dependent transaction indices.
    // It has length equal to the number of transactions in the block.
    // All elements in the sets must be less than the number of transactions in the block.
    pub txn_dependency: Vec<Mutex<HashSet<usize>>>,
    // Maps transaction index to its (incarnation_number, status) pair.
    // It has length equal to the number of transactions in the block.
    pub txn_status: Vec<Mutex<(usize, TaskStatus)>>,
}

#[derive(Clone)]
pub struct Scheduler(Arc<SchedulerInner>);

impl Scheduler {
    pub fn new(block_size: usize) -> Scheduler {
        let inner = SchedulerInner {
            txn_dependency: vec![Mutex::default(); block_size],
            txn_status: vec![Mutex::default(); block_size],
            ..SchedulerInner::default(),
        };
        Sefl(Arc::new(inner))
    }

    pub fn decrease_execution_idx(&mut self, target_idx: usize) {
        self.0.execution_idx.fetch_min(target_idx, MemoryOrdering::SeqCst);
        self.0.decrease_cnt.fetch_add(1, MemoryOrdering::SeqCst);
    }

    pub fn decrease_validation_idx(&mut self, target_idx: usize) {
        self.0.validation_idx.fetch_min(target_idx, MemoryOrdering::SeqCst);
        self.0.decrease_cnt.fetch_add(1, MemoryOrdering::SeqCst);
    }

    pub fn check_done(&mut self) {
        let block_size = self.0.txn_status.len();
        let observed_cnt = self.0.decrease_cnt.load(MemoryOrdering::SeqCst);
        let exec_idx = self.0.execution_idx.load(MemoryOrdering::SeqCst);
        let val_idx = self.0.validation_idx.load(MemoryOrdering::SeqCst);
        let num_active_tasks = self.0.num_active_tasks.load(MemoryOrdering::SeqCst);
        // NOTE: decrease_cnt needs to be checked again after the others to make sure it didn't
        // change.
        // TODO: better explanation from the paper.
        let decrease_cnt = self.0.decrease_cnt.load(MemoryOrdering::SeqCst);

        if val_idx.min(exec_idx) >= block_size && num_active_tasks == 0 && observed_cnt == decreace_cnt {
            self.0.done_marker.store(true, MemoryOrdering::SeqCst);
        }
    }

    pub fn try_incarnate(&mut self, txn_idx: usize) -> Option<(usize, usize)> {
        if txn_idx < block_size {
            let lock = self.0.txn_status.lock().expect("poisoned mutex");
            let (incarnation_num, status) = *lock;
            if matches!(status, TaskStatus::Ready) {
                lock.1 = TaskStatus::Executing;
                return Some((txn_idx, incarnation_num));
            }
        }
        self.0.num_active_tasks.fetch_sub(1, MemoryOrdering::SeqCst);
        None
    }

    pub fn next_version_to_execute(&mut self) -> Option<(usize, usize)> {
        let exec_idx = self.0.execution_idx.load(MemoryOrdering::SeqCst);
        let block_size = self.0.txn_status.len();
        if exec_idx >= block_size {
            self.check_done();
            return None;
        }
        self.0.num_active_tasks.fetch_add(1, MemoryOrdering::SeqCst);
        let idx_to_execute = self.0.execution_idx.fetch_add(1, MemoryOrdering::SeqCst);
        self.try_incarnate(idx_to_execute)
    }

    pub fn next_version_to_validate(&mut self) -> Option<(usize, usize)> {
        let val_idx = self.0.validation_idx.load(MemoryOrdering::SeqCst);
        let block_size = self.0.txn_status.len();
        if val_idx >= block_size {
            self.check_done();
            return None;
        }
        self.0.num_active_tasks.fetch_add(1, MemoryOrdering::SeqCst);
        let idx_to_validate = self.0.validation_idx.fetch_add(1, MemoryOrdering::SeqCst);
        if idx_to_validate < block_size {
            let (incarnation_num, status) = *self.0.txn_status.lock().expect("poisoned mutex");
            if matches!(status, TaskStatus::Executed) {
                return Some((idx_to_validate, incarnation_num));
            }
        }
        self.0.num_active_tasks.fetch_sub(1, MemoryOrdering::SeqCst);
        None
    }

    pub fn next_task(&mut self) -> Option<(usize, usize, TaskKind)> {
        let val_idx = self.0.validation_idx.load(MemoryOrdering::SeqCst);
        let exec_idx = self.0.execution_idx.load(MemoryOrdering::SeqCst);
        if val_idx < exec_idx {
            self.next_version_to_validate()?
                .map(|(idx, inc_num)| (idx, inc_num, TaskKind::Validation);
        } else {
            self.next_version_to_execute()?
                .map(|(idx, inc_num)| (idx, inc_num, TaskKind::Execution);
        }
    }

    pub fn add_dependency(&mut self, txn_idx: usize, blocking_txn_idx: usize) -> bool {
        // NOTE: this lock could be taken at the time of update AFAICT, but the paper holds it from
        // the start up to just before updating the `num_active_tasts` field for some reason.
        let deps = self.0.txn_dependency[blocking_txn_idx].lock().expect("poisoned mutex");
        let (_, blocking_txn_status) = *self.0.txn_status[blocking_txn_idx].lock().expect("poisoned mutex");
        if matches!(blocking_txn_status, TaskStatus::Executed) {
            return false;
        }
        self.0.txn_status[txn_idx].lock().expect("poisoned mutex").1 = TaskStatus::Aborting;
        deps.insert(txn_idx);
        self.0.num_active_tasks.fetch_sub(1, MemoryOrdering::SeqCst);
        true
    }

    pub fn set_ready_status(&mut self, txn_idx: usize) {
        let lock = self.0.txn_status[txn_ids].lock().expect("poisoned mutex");
        let (current_incarnation, current_status) = *lock;
        debug_assert!(matches!(current_status, TaskStatus::Aborting);
        *lock = (current_incarnation + 1, TaskStatus::Ready);
    }

    pub fn resume_dependencies(&mut self, dependent_txn_indices: &HashSet<usize>) {
        // TODO: consider using a more efficient set for the kind of numbers we deal with.
        let mut min_dependency_idx = usize::MAX;
        for dep_txn_idx in dependent_txn_indices {
            min_dependency_idx = min_dependency_idx.min(dep_txn_idx);
            self.set_ready_status(*dep_txn_idx);
        }
        if min != usize::MAX {
            self.decrease_execution_idx(min_dependency_idx);
        }
    }

    pub fn finish_execution(&mut self, txn_idx: usize, wrote_new_path: bool) -> Option<(usize, usize, TaskKind)> {
        {
            let lock = self.0.txn_status[txn_idx].lock().expect("poisoned mutex");
            let (current_incarnation, current_status) = *lock;
            debug_assert!(matches!(current_status, TaskStatus::Executing));
            *lock = (current_incarnation, TaskStatus::Executed);
        }
        let deps = core::mem::take(&mut self.txn_depencency[txn_idx].lock().expect("poisoned_mutex"));
        self.resume_dependencies(&deps);
        let val_idx = self.0.validation_idx.load(MemoryOrdering::SeqCst);
        if val_idx > txn_idx {
            if wrote_new_path {
                self.decrease_validation_idx(txn_idx);
            } else {
                return Some((txn_idx, current_incarnation, TaskKind::Validation));
            }
        }
        self.0.num_active_tasks.fetch_sub(1, MemoryOrdering::SeqCst);
        None
    }

    pub fn try_validation_abort(&mut self, txn_idx: usize, incarnation: usize) -> bool {
        let lock = self.0.txn_status[txn_idx].lock().expect("poisoned mutex");
        let (current_incarnation, current_status) = *lock;
        if matches!(current_status, TaskStatus::Executed) {
            *lock = (current_incarnation, TaskStatus::Aborting);
            return true;
        }
        false
    }

    pub fn finish_validation(&mut self, txn_idx: usize, aborted: bool) -> Option<(usize, usize, TaskKind)> {
        if aborted {
            self.set_ready_status(txn_idx);
            self.decrease_validation_idx(txn_idx + 1);
            let exec_idx = self.0.execution_idx.load(MemoryOrdering::SeqCst);
            if exec_idx > txn_idx {
                return self.try_incarnate(txn_idx)
                    .map(|(idx, inc_num)| (idx, inc_num, TaskKind::Execution);
            }
        }
        None
    }
}
