// NOTES:
// 1. There is a chance that we can pack the (incarnation_number, status) inside an atomic variable
//    instead of using a `Mutex`.
// 2. We can probably store the state to restore partial executions (this is suggested in the
//    paper).
// 3. Mermory ordering needs to be reviewed. To play it safe, I'm using SeqCst, which provides
//    stricter ordering, but it comes with some overhead.
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
        // TODO
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
}
