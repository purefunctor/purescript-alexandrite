//! Implements the core build system for the query-based compiler
//!
//! Our implementation is inspired by the verifying step traces described in
//! the [Build systems à la carte: Theory and practice] paper. However, the
//! implementation has two key differences: we only retain the latest step
//! trace for any given query; and more significantly, we use structural
//! equality instead of hashing to compare cached and fresh values.
//!
//! Unlike traditional phase-based compilation, query-based compilers are
//! designed to have its intermediate states be observed directly using a
//! convenient API.
//!
//! The build system is designed to be pure and hermetic—the current state of
//! the workspace e.g. file contents are stored in-memory to make dependency
//! tracking easier to manage.
//!
//! Our implementation also borrows a few techniques used by [salsa] such as
//! using global query lock for ordering query reads and input writes, and
//! future-promise-based work deduplication. These techniques enable parallel
//! computation with cancellation and work deduplication!
//!
//! [Build systems à la carte: Theory and practice]: https://www.cambridge.org/core/journals/journal-of-functional-programming/article/build-systems-a-la-carte-theory-and-practice/097CE52C750E69BD16B78C318754C7A4
//! [salsa]: https://github.com/salsa-rs/salsa

mod graph;
mod promise;

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::hash::{BuildHasher, Hash};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use building_types::{
    ModuleNameId, ModuleNameInterner, QueryError, QueryKey, QueryProxy, QueryResult,
};
use checking::CheckedModule;
use documenting::DocumentedModule;
use elaborating::CoreModule;
use files::FileId;
use graph::SnapshotGraph;
use indexing::IndexedModule;
use lock_api::{RawRwLock, RawRwLockRecursive};
use lowering::{GroupedModule, LoweredModule};
use parking_lot::{Mutex, RwLock, RwLockUpgradableReadGuard};
use parsing::FullParsedModule;
use promise::{Future, Promise};
use resolving::ResolvedModule;
use rustc_hash::{FxBuildHasher, FxHashMap, FxHashSet};
use stabilizing::StabilizedModule;
use thread_local::ThreadLocal;

#[derive(Debug, Clone, Copy)]
struct Trace {
    /// Timestamp of when the query was last called.
    built: usize,
    /// Timestamp of when the query was last recomputed.
    changed: usize,
}

#[derive(Debug, Default)]
enum DerivedState<T> {
    #[default]
    NotComputed,
    InProgress {
        id: SnapshotId,
        promises: Mutex<Vec<Promise<T>>>,
    },
    Computed {
        computed: T,
        trace: Trace,
        dependencies: Arc<[QueryKey]>,
    },
}

impl<T> DerivedState<T> {
    fn in_progress(id: SnapshotId) -> DerivedState<T> {
        DerivedState::InProgress { id, promises: Mutex::default() }
    }
}

#[derive(Debug)]
struct InputState<T> {
    value: T,
    changed: usize,
}

const SHARDS: usize = 16;
const SHARD_MASK: usize = SHARDS - 1;

/// A [`SHARDS`]-way sharded [`FxHashMap`] with individual [`RwLock`].
struct Shards<K, V> {
    inner: [RwLock<FxHashMap<K, V>>; SHARDS],
}

impl<K, V> Default for Shards<K, V> {
    fn default() -> Shards<K, V> {
        Shards { inner: std::array::from_fn(|_| RwLock::new(FxHashMap::default())) }
    }
}

impl<K, V> Shards<K, V>
where
    K: Hash,
{
    fn shard(&self, key: &K) -> &RwLock<FxHashMap<K, V>> {
        let hash = FxBuildHasher.hash_one(key);
        &self.inner[(hash as usize) & SHARD_MASK]
    }
}

#[derive(Default)]
struct InputStorage {
    content: Shards<FileId, InputState<Arc<str>>>,
    module: Shards<ModuleNameId, InputState<FileId>>,
}

#[derive(Default)]
struct DerivedStorage {
    parsed: Shards<FileId, DerivedState<FullParsedModule>>,
    stabilized: Shards<FileId, DerivedState<Arc<StabilizedModule>>>,
    indexed: Shards<FileId, DerivedState<Arc<IndexedModule>>>,
    lowered: Shards<FileId, DerivedState<Arc<LoweredModule>>>,
    grouped: Shards<FileId, DerivedState<Arc<GroupedModule>>>,
    resolved: Shards<FileId, DerivedState<Arc<ResolvedModule>>>,
    bracketed: Shards<FileId, DerivedState<Arc<sugar::Bracketed>>>,
    sectioned: Shards<FileId, DerivedState<Arc<sugar::Sectioned>>>,
    checked: Shards<FileId, DerivedState<Arc<CheckedModule>>>,
    elaborated: Shards<FileId, DerivedState<Arc<CoreModule>>>,
    documented: Shards<FileId, DerivedState<Arc<DocumentedModule>>>,
}

#[derive(Default)]
struct InternedStorage {
    module: ModuleNameInterner,
    checking: checking::CoreInterners,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct SnapshotId(u32);

#[derive(Default)]
struct GlobalState {
    /// An atomic token that determines if query execution had been cancelled.
    cancelled: AtomicBool,
    /// A global read-write lock for enforcing the order of reads and writes.
    query_lock: RwLock<()>,
    /// A counter that tracks the current revision of the query engine.
    revision: AtomicUsize,
    /// A counter that tracks the next [`SnapshotId`],
    snapshot: AtomicU32,
    /// A graph that tracks dependencies between [`SnapshotId`]
    graph: Mutex<SnapshotGraph>,
}

impl GlobalState {
    fn next_snapshot(&self) -> SnapshotId {
        SnapshotId(self.snapshot.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Default)]
struct LocalState {
    inner: ThreadLocal<RefCell<LocalStateInner>>,
}

impl LocalState {
    fn with_current<T>(&self, current: QueryKey, f: impl FnOnce() -> T) -> T {
        let inner = self.inner.get_or_default();
        {
            let mut setup = inner.borrow_mut();
            setup.stack.push(current);
        }
        let result = f();
        {
            let mut cleanup = inner.borrow_mut();
            cleanup.stack.pop();
            cleanup.in_progress.remove(&current);
            cleanup.dependencies.remove(&current);
        }
        result
    }

    fn with_dependency(&self, dependency: QueryKey) {
        let mut inner = self.inner.get_or_default().borrow_mut();
        if let Some(&current) = inner.stack.last() {
            inner.dependencies.entry(current).or_default().insert(dependency);
        }
    }

    fn dependencies(&self, key: QueryKey) -> Arc<[QueryKey]> {
        let inner = &self.inner.get_or_default().borrow();
        inner
            .dependencies
            .get(&key)
            .map(|dependencies| dependencies.iter().copied())
            .unwrap_or_default()
            .collect()
    }

    fn stack(&self) -> Arc<[QueryKey]> {
        let inner = self.inner.get_or_default().borrow();
        inner.stack.as_slice().into()
    }

    fn add_in_progress(&self, key: QueryKey) {
        let mut inner = self.inner.get_or_default().borrow_mut();
        inner.in_progress.insert(key);
    }

    fn is_in_progress(&self, key: QueryKey) -> bool {
        let inner = self.inner.get_or_default().borrow();
        inner.in_progress.contains(&key)
    }
}

#[derive(Debug, Default)]
struct LocalStateInner {
    stack: Vec<QueryKey>,
    in_progress: FxHashSet<QueryKey>,
    dependencies: FxHashMap<QueryKey, FxHashSet<QueryKey>>,
}

/// Custom guard that acquires a read lock from the [`GlobalState::query_lock`]
/// and releases it when dropped, effectively tying it to the lifetime of the
/// [`QueryControl`] it belongs to.
struct QueryControlGuard {
    global: Arc<GlobalState>,
}

impl QueryControlGuard {
    fn new(global: &Arc<GlobalState>) -> QueryControlGuard {
        // SAFETY: QueryControlGuard::drop
        unsafe { global.query_lock.raw().lock_shared_recursive() };
        QueryControlGuard { global: Arc::clone(global) }
    }
}

impl Drop for QueryControlGuard {
    fn drop(&mut self) {
        // SAFETY: QueryControlGuard::new
        unsafe { self.global.query_lock.raw().unlock_shared() }
    }
}

struct QueryControl {
    _guard: Option<QueryControlGuard>,
    id: SnapshotId,
    local: Arc<LocalState>,
    global: Arc<GlobalState>,
}

impl QueryControl {
    fn snapshot(&self) -> QueryControl {
        let _guard = Some(QueryControlGuard::new(&self.global));
        let local = Arc::new(LocalState::default());
        let global = Arc::clone(&self.global);
        let id = global.next_snapshot();
        QueryControl { _guard, id, local, global }
    }
}

impl Default for QueryControl {
    fn default() -> QueryControl {
        let _guard = None;
        let local = Arc::new(LocalState::default());
        let global = Arc::new(GlobalState::default());
        let id = global.next_snapshot();
        QueryControl { _guard, id, local, global }
    }
}

#[derive(Default)]
pub struct QueryEngine {
    input: Arc<InputStorage>,
    derived: Arc<DerivedStorage>,
    interned: Arc<InternedStorage>,
    control: QueryControl,
}

impl QueryEngine {
    /// Creates a snapshot of the [`QueryEngine`].
    ///
    /// Snapshots are read locks over the [`QueryEngine`] that must
    /// be sent across threads to perform query execution.
    ///
    /// As with read locks, keeping snapshots alive indefinitely is
    /// a logic error and will cause a deadlock on mutation or on a
    /// [cancellation request].
    ///
    /// [cancellation request]: QueryEngine::request_cancel
    pub fn snapshot(&self) -> QueryEngine {
        let input = self.input.clone();
        let derived = self.derived.clone();
        let interned = self.interned.clone();
        let control = self.control.snapshot();
        QueryEngine { input, derived, interned, control }
    }

    /// Creates a cancellation request for queries.
    ///
    /// Query cancellation is cooperative. A cancellation flag is read
    /// at some point during query execution. This function also waits
    /// for all snapshots to be dropped, as in the expected consequence
    /// of cancelling all queries running across all threads.
    pub fn request_cancel(&self) {
        self.control.global.cancelled.store(true, Ordering::Relaxed);
        let _query_lock = self.control.global.query_lock.write();
        self.control.global.cancelled.store(false, Ordering::Relaxed);
    }
}

impl QueryEngine {
    fn query<K, V, ShardsFn, ComputeFn>(
        &self,
        query: QueryKey,
        key: K,
        shards: ShardsFn,
        compute: ComputeFn,
    ) -> QueryResult<V>
    where
        K: Hash + Eq + Copy,
        ShardsFn: Fn(&DerivedStorage) -> &Shards<K, DerivedState<V>>,
        ComputeFn: Fn(&QueryEngine) -> QueryResult<V>,
        V: Eq + Clone,
    {
        self.control.local.with_dependency(query);
        self.control.local.with_current(query, || {
            // If query execution fails at any given point, clean up the state.
            self.query_core(query, key, &shards, &compute).inspect_err(|_| {
                if self.control.local.is_in_progress(query) {
                    let shard = shards(&self.derived).shard(&key);
                    let mut guard = shard.write();
                    if let Entry::Occupied(o) = guard.entry(key) {
                        if let DerivedState::InProgress { id, promises } = o.remove() {
                            let mut graph = self.control.global.graph.lock();
                            drop(promises);
                            drop(guard);
                            graph.remove_edge(id);
                        } else {
                            unreachable!("invariant violated: expected InProgress");
                        }
                    }
                }
            })
        })
    }

    /// Fulfills the promises of an [`DerivedState::InProgress`] query and
    /// replaces it with a [`DerivedState::Computed`] result in the store.
    fn fulfill_and_store<K, V, ShardsFn>(
        &self,
        key: K,
        shards: &ShardsFn,
        computed: V,
        trace: Trace,
        dependencies: Arc<[QueryKey]>,
    ) where
        K: Hash + Eq + Copy,
        ShardsFn: Fn(&DerivedStorage) -> &Shards<K, DerivedState<V>>,
        V: Clone,
    {
        let shard = shards(&self.derived).shard(&key);
        let mut guard = shard.write();
        if let Entry::Occupied(o) = guard.entry(key) {
            if let DerivedState::InProgress { id, promises } = o.remove() {
                let mut graph = self.control.global.graph.lock();
                let promises = promises.into_inner();
                promises.into_iter().for_each(|promise| {
                    let computed = V::clone(&computed);
                    promise.fulfill(computed);
                });
                graph.remove_edge(id);
            } else {
                unreachable!("invariant violated: expected InProgress");
            }
        }

        let state = DerivedState::Computed { computed, trace, dependencies };
        guard.insert(key, state);
    }

    fn compute_core<K, V, ShardsFn, ComputeFn>(
        &self,
        key: K,
        shards: &ShardsFn,
        compute: &ComputeFn,
        query: QueryKey,
        revision: usize,
        previous: Option<(V, Trace)>,
    ) -> QueryResult<V>
    where
        K: Hash + Eq + Copy,
        ShardsFn: Fn(&DerivedStorage) -> &Shards<K, DerivedState<V>>,
        ComputeFn: Fn(&QueryEngine) -> QueryResult<V>,
        V: Eq + Clone,
    {
        if self.control.global.cancelled.load(Ordering::Relaxed) {
            return Err(QueryError::Cancelled);
        }

        let computed = compute(self)?;

        // If the computed result is equal to the cached one, the changed
        // timestamp does not need to be updated. Likewise, we also insert
        // the previous value back into the cache. The latter is a niche,
        // but useful optimisation for when V = Arc<T>, since it enables
        // pointer equality.
        match previous {
            Some((previous, trace)) if computed == previous => {
                let trace = Trace { built: revision, changed: trace.changed };
                let dependencies = self.control.local.dependencies(query);
                self.fulfill_and_store(key, shards, V::clone(&previous), trace, dependencies);
                Ok(previous)
            }
            _ => {
                let trace = Trace { built: revision, changed: revision };
                let dependencies = self.control.local.dependencies(query);
                self.fulfill_and_store(key, shards, V::clone(&computed), trace, dependencies);
                Ok(computed)
            }
        }
    }

    /// Verifies the given dependencies by executing them, returning the
    /// timestamp of the most latest change.
    fn verify_core(&self, dependencies: &[QueryKey]) -> QueryResult<usize> {
        let mut latest = 0;

        macro_rules! input_changed {
            ($field:ident, $key:expr) => {{
                let shard = self.input.$field.shard($key).read();
                if let Some(InputState { changed, .. }) = shard.get($key) {
                    latest = latest.max(*changed);
                }
            }};
        }

        macro_rules! derived_changed {
            ($field:ident, $key:expr) => {{
                self.$field(*$key)?;
                let shard = self.derived.$field.shard($key).read();
                if let Some(DerivedState::Computed { trace, .. }) = shard.get($key) {
                    latest = latest.max(trace.changed);
                }
            }};
        }

        for dependency in dependencies {
            match dependency {
                QueryKey::Content(k) => input_changed!(content, k),
                QueryKey::Module(k) => input_changed!(module, k),
                QueryKey::Parsed(k) => derived_changed!(parsed, k),
                QueryKey::Stabilized(k) => derived_changed!(stabilized, k),
                QueryKey::Indexed(k) => derived_changed!(indexed, k),
                QueryKey::Lowered(k) => derived_changed!(lowered, k),
                QueryKey::Grouped(k) => derived_changed!(grouped, k),
                QueryKey::Resolved(k) => derived_changed!(resolved, k),
                QueryKey::Bracketed(k) => derived_changed!(bracketed, k),
                QueryKey::Sectioned(k) => derived_changed!(sectioned, k),
                QueryKey::Checked(k) => derived_changed!(checked, k),
                QueryKey::Elaborated(k) => derived_changed!(elaborated, k),
                QueryKey::Documented(k) => derived_changed!(documented, k),
            }
        }

        Ok(latest)
    }

    fn create_future<T>(
        &self,
        to_id: SnapshotId,
        promises: &Mutex<Vec<Promise<T>>>,
    ) -> QueryResult<Future<T>> {
        {
            let mut graph = self.control.global.graph.lock();
            let stack = self.control.local.stack();
            if !graph.add_edge(self.control.id, to_id) {
                return Err(QueryError::Cycle { stack });
            }
        }

        let (future, promise) = Future::new();
        promises.lock().push(promise);
        Ok(future)
    }

    fn query_core<K, V, ShardsFn, ComputeFn>(
        &self,
        query: QueryKey,
        key: K,
        shards: &ShardsFn,
        compute: &ComputeFn,
    ) -> QueryResult<V>
    where
        K: Hash + Eq + Copy,
        ShardsFn: Fn(&DerivedStorage) -> &Shards<K, DerivedState<V>>,
        ComputeFn: Fn(&QueryEngine) -> QueryResult<V>,
        V: Eq + Clone,
    {
        if self.control.global.cancelled.load(Ordering::Relaxed) {
            return Err(QueryError::Cancelled);
        }

        let revision = self.control.global.revision.load(Ordering::Relaxed);
        let shard = shards(&self.derived).shard(&key);

        // Certain query states can be checked with only a read lock, and this
        // is an extremely useful optimisation because it allows threads to
        // skip their turn on acquiring an upgradable read lock.
        //
        // For computed queries, we can skip dependency verification if the
        // cached value was built during the current revision.
        //
        // For in-progress queries, we can simply push to the internally mutable
        // vector of promises and then wait on the future.
        {
            let guard = shard.read();
            match guard.get(&key).unwrap_or(&DerivedState::NotComputed) {
                DerivedState::Computed { computed, trace, .. } if trace.built == revision => {
                    return Ok(V::clone(computed));
                }
                DerivedState::InProgress { id, promises } => {
                    let future = self.create_future(*id, promises)?;

                    // Remember that Future::wait blocks the current thread!
                    drop(guard);

                    return future.wait().ok_or(QueryError::Cancelled);
                }
                _ => (),
            }
        }

        // Otherwise, we will have to perform computation or cache verification.
        // Instead of a write lock, we use an upgradable read lock for two reasons:
        // we want to ensure that only a single thread can observe the NotComputed
        // state for any given query while allowing read locks to be acquired for
        // the optimisation above.
        {
            let guard = shard.upgradable_read();
            match guard.get(&key).unwrap_or(&DerivedState::NotComputed) {
                DerivedState::NotComputed => {
                    // At the end of this block, threads waiting to acquire the
                    // upgradable read lock should read that the query is InProgress.
                    {
                        let mut guard = RwLockUpgradableReadGuard::upgrade(guard);
                        guard.insert(key, DerivedState::in_progress(self.control.id));
                        self.control.local.add_in_progress(query);
                    }

                    self.compute_core(key, shards, compute, query, revision, None)
                }
                DerivedState::InProgress { id, promises } => {
                    let future = self.create_future(*id, promises)?;

                    // Remember that Future::wait blocks the current thread!
                    drop(guard);

                    future.wait().ok_or(QueryError::Cancelled)
                }
                DerivedState::Computed { computed, trace, dependencies } => {
                    let computed = V::clone(computed);
                    let trace = *trace;
                    let dependencies = Arc::clone(dependencies);

                    // If the cached value was built during the current revision
                    // we can skip dependency verification entirely. This is also
                    // checked at the start of the query_core with a read lock.
                    if trace.built == revision {
                        return Ok(computed);
                    }

                    // Same as NotComputed, see comment above.
                    {
                        let mut guard = RwLockUpgradableReadGuard::upgrade(guard);
                        guard.insert(key, DerivedState::in_progress(self.control.id));
                        self.control.local.add_in_progress(query);
                    }

                    let latest = self.verify_core(&dependencies)?;

                    // If the cached value was built more recently the the
                    // latest change, we can update its built timestamp to
                    // the current revision. This allows the query to hit
                    // the fastest path if it's called in the same revision.
                    if trace.built >= latest {
                        let trace = Trace { built: revision, ..trace };
                        self.fulfill_and_store(
                            key,
                            shards,
                            V::clone(&computed),
                            trace,
                            dependencies,
                        );
                        return Ok(computed);
                    }

                    self.compute_core(
                        key,
                        shards,
                        compute,
                        query,
                        revision,
                        Some((computed, trace)),
                    )
                }
            }
        }
    }

    fn set_input<K, V, F>(&self, key: K, shards: F, value: V)
    where
        K: Hash + Eq + Copy,
        F: FnOnce(&InputStorage) -> &Shards<K, InputState<V>>,
    {
        self.control.global.cancelled.store(true, Ordering::Relaxed);
        let _query_lock = self.control.global.query_lock.write();

        let changed = self.control.global.revision.fetch_add(1, Ordering::Relaxed);
        let state = InputState { value, changed: changed + 1 };

        let shard = shards(&self.input).shard(&key);
        shard.write().insert(key, state);

        self.control.global.cancelled.store(false, Ordering::Relaxed);
    }

    fn get_input<K, V, F>(&self, query: QueryKey, key: K, shards: F) -> Option<V>
    where
        K: Hash + Eq,
        F: FnOnce(&InputStorage) -> &Shards<K, InputState<V>>,
        V: Clone,
    {
        self.control.local.with_dependency(query);
        let shard = shards(&self.input).shard(&key);
        let guard = shard.read();
        guard.get(&key).map(|state| V::clone(&state.value))
    }
}

impl QueryEngine {
    pub fn set_content(&self, id: FileId, content: impl Into<Arc<str>>) {
        self.set_input(id, |input| &input.content, content.into());
    }

    pub fn content(&self, id: FileId) -> Arc<str> {
        self.get_input(QueryKey::Content(id), id, |input| &input.content).unwrap_or_else(|| {
            panic!("invariant violated: set_content({id:?}, ..)");
        })
    }

    pub fn set_module_file(&self, name: &str, file_id: FileId) {
        let id = self.interned.module.intern(name);
        self.set_input(id, |input| &input.module, file_id);
    }

    pub fn module_file(&self, name: &str) -> Option<FileId> {
        let id = self.interned.module.lookup(name)?;
        self.get_input(QueryKey::Module(id), id, |input| &input.module)
    }

    pub fn parsed(&self, id: FileId) -> QueryResult<FullParsedModule> {
        self.query(
            QueryKey::Parsed(id),
            id,
            |derived| &derived.parsed,
            |this| {
                let content = this.content(id);

                let lexed = lexing::lex(&content);
                let tokens = lexing::layout(&lexed);
                let parsed = parsing::parse(&lexed, &tokens);

                Ok(parsed)
            },
        )
    }

    pub fn stabilized(&self, id: FileId) -> QueryResult<Arc<StabilizedModule>> {
        self.query(
            QueryKey::Stabilized(id),
            id,
            |derived| &derived.stabilized,
            |this| {
                let (parsed, _) = this.parsed(id)?;
                let node = parsed.syntax_node();
                Ok(Arc::new(stabilizing::stabilize_module(&node)))
            },
        )
    }

    pub fn indexed(&self, id: FileId) -> QueryResult<Arc<IndexedModule>> {
        self.query(
            QueryKey::Indexed(id),
            id,
            |derived| &derived.indexed,
            |this| {
                let content = this.content(id);
                let (parsed, _) = this.parsed(id)?;
                let stabilized = this.stabilized(id)?;

                let module = parsed.cst();
                let indexed = indexing::index_module(&content, &module, &stabilized);

                Ok(Arc::new(indexed))
            },
        )
    }

    pub fn lowered(&self, id: FileId) -> QueryResult<Arc<LoweredModule>> {
        self.query(
            QueryKey::Lowered(id),
            id,
            |derived| &derived.lowered,
            |this| {
                let content = this.content(id);
                let (parsed, _) = this.parsed(id)?;

                let prim = {
                    let prim_id = this.prim_id();
                    this.resolved(prim_id)?
                };

                let stabilized = this.stabilized(id)?;
                let indexed = this.indexed(id)?;
                let resolved = this.resolved(id)?;

                let module = parsed.cst();
                let lowered = lowering::lower_module(
                    id,
                    &content,
                    &module,
                    &prim,
                    &stabilized,
                    &indexed,
                    &resolved,
                );

                Ok(Arc::new(lowered))
            },
        )
    }

    pub fn grouped(&self, id: FileId) -> QueryResult<Arc<GroupedModule>> {
        self.query(
            QueryKey::Grouped(id),
            id,
            |derived| &derived.grouped,
            |this| {
                let lowered = this.lowered(id)?;
                let indexed = this.indexed(id)?;
                let groups = lowering::group_module(&indexed, &lowered);
                Ok(Arc::new(groups))
            },
        )
    }

    pub fn resolved(&self, id: FileId) -> QueryResult<Arc<ResolvedModule>> {
        self.query(
            QueryKey::Resolved(id),
            id,
            |derived| &derived.resolved,
            |this| {
                let resolved = resolving::resolve_module(this, id)?;
                Ok(Arc::new(resolved))
            },
        )
    }

    pub fn bracketed(&self, id: FileId) -> QueryResult<Arc<sugar::Bracketed>> {
        self.query(
            QueryKey::Bracketed(id),
            id,
            |derived| &derived.bracketed,
            |this| {
                let lowered = this.lowered(id)?;
                let bracketed = sugar::bracketed(this, &lowered)?;
                Ok(Arc::new(bracketed))
            },
        )
    }

    pub fn sectioned(&self, id: FileId) -> QueryResult<Arc<sugar::Sectioned>> {
        self.query(
            QueryKey::Sectioned(id),
            id,
            |derived| &derived.sectioned,
            |this| {
                let lowered = this.lowered(id)?;
                let sectioned = sugar::sectioned(&lowered);
                Ok(Arc::new(sectioned))
            },
        )
    }

    pub fn checked(&self, id: FileId) -> QueryResult<Arc<CheckedModule>> {
        self.query(
            QueryKey::Checked(id),
            id,
            |derived| &derived.checked,
            |this| {
                let checked = checking::check_module(this, id)?;
                Ok(Arc::new(checked))
            },
        )
    }

    pub fn elaborated(&self, id: FileId) -> QueryResult<Arc<CoreModule>> {
        self.query(
            QueryKey::Elaborated(id),
            id,
            |derived| &derived.elaborated,
            |this| {
                let indexed = this.indexed(id)?;
                let lowered = this.lowered(id)?;
                let grouped = this.grouped(id)?;
                let bracketed = this.bracketed(id)?;
                let sectioned = this.sectioned(id)?;
                let checked = this.checked(id)?;

                let input = elaborating::ElaborationInput {
                    file_id: id,
                    indexed: &indexed,
                    lowered: &lowered,
                    grouped: &grouped,
                    bracketed: &bracketed,
                    sectioned: &sectioned,
                    checked: &checked,
                };

                Ok(Arc::new(elaborating::elaborate_module(input)))
            },
        )
    }

    pub fn documented(&self, id: FileId) -> QueryResult<Arc<DocumentedModule>> {
        self.query(
            QueryKey::Documented(id),
            id,
            |derived| &derived.documented,
            |this| {
                let content = this.content(id);
                let (parsed, _) = this.parsed(id)?;
                let stabilized = this.stabilized(id)?;
                let indexed = this.indexed(id)?;
                Ok(documenting::document_module(&content, &parsed, &stabilized, &indexed))
            },
        )
    }
}

impl QueryEngine {
    pub fn prim_id(&self) -> FileId {
        self.module_file("Prim").expect("invariant violated: prim::configure")
    }
}

impl QueryProxy for QueryEngine {
    type Parsed = FullParsedModule;

    type Stabilized = Arc<StabilizedModule>;

    type Indexed = Arc<IndexedModule>;

    type Lowered = Arc<LoweredModule>;

    type Grouped = Arc<GroupedModule>;

    type Resolved = Arc<ResolvedModule>;

    type Bracketed = Arc<sugar::Bracketed>;

    type Sectioned = Arc<sugar::Sectioned>;

    type Checked = Arc<checking::CheckedModule>;

    type Elaborated = Arc<elaborating::CoreModule>;

    type Documented = Arc<documenting::DocumentedModule>;

    fn parsed(&self, id: FileId) -> QueryResult<Self::Parsed> {
        QueryEngine::parsed(self, id)
    }

    fn stabilized(&self, id: FileId) -> QueryResult<Self::Stabilized> {
        QueryEngine::stabilized(self, id)
    }

    fn indexed(&self, id: FileId) -> QueryResult<Self::Indexed> {
        QueryEngine::indexed(self, id)
    }

    fn lowered(&self, id: FileId) -> QueryResult<Self::Lowered> {
        QueryEngine::lowered(self, id)
    }

    fn grouped(&self, id: FileId) -> QueryResult<Self::Grouped> {
        QueryEngine::grouped(self, id)
    }

    fn resolved(&self, id: FileId) -> QueryResult<Self::Resolved> {
        QueryEngine::resolved(self, id)
    }

    fn bracketed(&self, id: FileId) -> QueryResult<Self::Bracketed> {
        QueryEngine::bracketed(self, id)
    }

    fn sectioned(&self, id: FileId) -> QueryResult<Self::Sectioned> {
        QueryEngine::sectioned(self, id)
    }

    fn checked(&self, id: FileId) -> QueryResult<Arc<checking::CheckedModule>> {
        QueryEngine::checked(self, id)
    }

    fn elaborated(&self, id: FileId) -> QueryResult<Arc<elaborating::CoreModule>> {
        QueryEngine::elaborated(self, id)
    }

    fn documented(&self, id: FileId) -> QueryResult<Arc<documenting::DocumentedModule>> {
        QueryEngine::documented(self, id)
    }

    fn prim_id(&self) -> FileId {
        QueryEngine::prim_id(self)
    }

    fn module_file(&self, name: &str) -> Option<FileId> {
        QueryEngine::module_file(self, name)
    }
}

impl checking::PrettyQueries for QueryEngine {
    fn lookup_type(&self, id: checking::TypeId) -> checking::Type {
        self.interned.checking.lookup_type(id)
    }

    fn lookup_forall_binder(
        &self,
        id: checking::core::ForallBinderId,
    ) -> checking::core::ForallBinder {
        self.interned.checking.lookup_forall_binder(id)
    }

    fn lookup_row_type(&self, id: checking::core::RowTypeId) -> checking::core::RowType {
        self.interned.checking.lookup_row_type(id)
    }

    fn lookup_smol_str(&self, id: checking::core::SmolStrId) -> smol_str::SmolStr {
        self.interned.checking.lookup_smol_str(id)
    }
}

impl checking::ExternalQueries for QueryEngine {
    fn intern_type(&self, t: checking::Type) -> checking::TypeId {
        self.interned.checking.intern_type(t)
    }

    fn intern_forall_binder(
        &self,
        binder: checking::core::ForallBinder,
    ) -> checking::core::ForallBinderId {
        self.interned.checking.intern_forall_binder(binder)
    }

    fn intern_row_type(&self, row: checking::core::RowType) -> checking::core::RowTypeId {
        self.interned.checking.intern_row_type(row)
    }

    fn intern_smol_str(&self, s: smol_str::SmolStr) -> checking::core::SmolStrId {
        self.interned.checking.intern_smol_str(s)
    }
}

impl resolving::ExternalQueries for QueryEngine {}

impl sugar::ExternalQueries for QueryEngine {}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;
    use std::sync::Arc;
    use std::sync::atomic::Ordering;

    use building_types::{QueryError, QueryResult};
    use checking::evidence::{
        Evidence, EvidenceAbstractionSite, EvidenceApplicationSite, EvidenceState, EvidenceVarId,
        Evidences, InstanceCandidateOrigin,
    };
    use elaborating::{
        CoreBindingValue, CoreDerivedEvidence, CoreExpression, CorePattern, CoreSuperclassField,
        CoreTypeArgument, CoreVariable,
    };
    use files::{FileId, Files};
    use la_arena::RawIdx;
    use lowering::ExpressionKind;
    use resolving::ResolvedModule;

    use crate::prim;

    use super::{DerivedState, QueryEngine, QueryKey};

    #[derive(Debug)]
    struct Trace<'a> {
        built: usize,
        changed: usize,
        dependencies: &'a [QueryKey],
    }

    struct ShowTrace<'a, T>(&'a DerivedState<T>);

    impl<'a, T> Debug for ShowTrace<'a, T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match &self.0 {
                DerivedState::NotComputed => write!(f, "NotComputed"),
                DerivedState::InProgress { .. } => write!(f, "InProgress {{ .. }}"),
                DerivedState::Computed { trace, dependencies, .. } => f
                    .debug_struct("Trace")
                    .field("built", &trace.built)
                    .field("changed", &trace.changed)
                    .field("dependencies", dependencies)
                    .finish(),
            }
        }
    }

    impl<'a, 'b, T> PartialEq<Trace<'b>> for ShowTrace<'a, T> {
        fn eq(&self, other: &Trace<'b>) -> bool {
            match self.0 {
                DerivedState::NotComputed => false,
                DerivedState::InProgress { .. } => false,
                DerivedState::Computed { trace, dependencies, .. } => {
                    trace.built == other.built
                        && trace.changed == other.changed
                        && dependencies.as_ref() == other.dependencies
                }
            }
        }
    }

    #[test]
    fn test_pointer_equality() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nlife = 42");
        let content = files.content(id);

        engine.set_content(id, content);
        let index_a = engine.indexed(id).unwrap();
        let index_b = engine.indexed(id).unwrap();
        assert!(Arc::ptr_eq(&index_a, &index_b));

        let id = files.insert("./src/Main.purs", "module Main where\n\nlife = 42\n\n");
        let content = files.content(id);

        engine.set_content(id, content);
        let index_a = engine.indexed(id).unwrap();
        let index_b = engine.indexed(id).unwrap();
        assert!(Arc::ptr_eq(&index_a, &index_b));
    }

    #[test]
    fn test_indexed_depends_on_source_text() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nlife = 42");
        engine.set_content(id, files.content(id));
        let indexed = engine.indexed(id).unwrap();
        assert!(indexed.names.terms.lookup("life").is_some());

        let id = files.insert("./src/Main.purs", "module Main where\n\ntime = 42");
        engine.set_content(id, files.content(id));
        let indexed = engine.indexed(id).unwrap();
        assert!(indexed.names.terms.lookup("life").is_none());
        assert!(indexed.names.terms.lookup("time").is_some());
    }

    #[test]
    fn test_text_edit_preserves_structural_query_traces() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        macro_rules! assert_trace {
            ($engine:expr, $field:ident($id:expr) => $trace:expr) => {{
                let shard = $engine.derived.$field.shard(&$id);
                let guard = shard.read();
                assert_eq!(ShowTrace(guard.get(&$id).unwrap()), $trace);
            }};
        }

        let id = files.insert("./src/Main.purs", "module Main where\n\nlife = 42");
        engine.set_content(id, files.content(id));
        let stabilized_a = engine.stabilized(id).unwrap();
        let indexed_a = engine.indexed(id).unwrap();

        assert_trace!(engine, parsed(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Content(id)]
        });
        assert_trace!(engine, stabilized(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Parsed(id)]
        });
        assert_trace!(engine, indexed(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Content(id), QueryKey::Parsed(id), QueryKey::Stabilized(id)]
        });

        let id = files.insert("./src/Main.purs", "module Main where\n\ntime = 42");
        engine.set_content(id, files.content(id));
        let stabilized_b = engine.stabilized(id).unwrap();
        let indexed_b = engine.indexed(id).unwrap();

        assert_trace!(engine, parsed(id) => Trace {
            built: 20,
            changed: 19,
            dependencies: &[QueryKey::Content(id)]
        });
        assert_trace!(engine, stabilized(id) => Trace {
            built: 20,
            changed: 19,
            dependencies: &[QueryKey::Parsed(id)]
        });
        assert_trace!(engine, indexed(id) => Trace {
            built: 20,
            changed: 20,
            dependencies: &[QueryKey::Content(id), QueryKey::Parsed(id), QueryKey::Stabilized(id)]
        });

        assert!(Arc::ptr_eq(&stabilized_a, &stabilized_b));
        assert!(!Arc::ptr_eq(&indexed_a, &indexed_b));
    }

    #[test]
    fn test_verifying_step_traces() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        macro_rules! assert_trace {
            ($engine:expr, $field:ident($id:expr) => $trace:expr) => {{
                let shard = $engine.derived.$field.shard(&$id);
                let guard = shard.read();
                assert_eq!(ShowTrace(guard.get(&$id).unwrap()), $trace);
            }};
        }

        let id = files.insert("./src/Main.purs", "module Main where\n\nlife = 42");
        let content = files.content(id);

        engine.set_content(id, content);
        let indexed_a = engine.indexed(id).unwrap();
        let lowered_a = engine.lowered(id).unwrap();
        let resolved_a = engine.resolved(id).unwrap();

        assert_trace!(engine, parsed(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Content(id)]
        });
        assert_trace!(engine, indexed(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Content(id), QueryKey::Parsed(id), QueryKey::Stabilized(id)]
        });
        assert_trace!(engine, resolved(id) => Trace {
            built: 19,
            changed: 19,
            dependencies: &[QueryKey::Indexed(id)]
        });

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nlife = 42");
        let content = files.content(id);

        engine.set_content(id, content);
        let indexed_b = engine.indexed(id).unwrap();
        let lowered_b = engine.lowered(id).unwrap();
        let resolved_b = engine.resolved(id).unwrap();

        assert_trace!(engine, parsed(id) => Trace {
            built: 20,
            changed: 20,
            dependencies: &[QueryKey::Content(id)]
        });
        assert_trace!(engine, indexed(id) => Trace {
            built: 20,
            changed: 19,
            dependencies: &[QueryKey::Content(id), QueryKey::Parsed(id), QueryKey::Stabilized(id)]
        });
        assert_trace!(engine, resolved(id) => Trace {
            built: 20,
            changed: 19,
            dependencies: &[QueryKey::Indexed(id)]
        });

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nlife = 42\n\n");
        let content = files.content(id);

        engine.set_content(id, content);
        let indexed_c = engine.indexed(id).unwrap();
        let lowered_c = engine.lowered(id).unwrap();
        let resolved_c = engine.resolved(id).unwrap();

        assert_trace!(engine, parsed(id) => Trace {
            built: 21,
            changed: 21,
            dependencies: &[QueryKey::Content(id)]
        });
        assert_trace!(engine, indexed(id) => Trace {
            built: 21,
            changed: 19,
            dependencies: &[QueryKey::Content(id), QueryKey::Parsed(id), QueryKey::Stabilized(id)]
        });
        assert_trace!(engine, resolved(id) => Trace {
            built: 21,
            changed: 19,
            dependencies: &[QueryKey::Indexed(id)]
        });

        assert!(Arc::ptr_eq(&indexed_a, &indexed_b));
        assert!(Arc::ptr_eq(&indexed_b, &indexed_c));

        assert!(Arc::ptr_eq(&lowered_a, &lowered_b));
        assert!(Arc::ptr_eq(&lowered_b, &lowered_c));

        assert!(Arc::ptr_eq(&resolved_a, &resolved_b));
        assert!(Arc::ptr_eq(&resolved_b, &resolved_c));
    }

    #[test]
    fn test_local_state_cleanup() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nlife = 42");
        let content = files.content(id);

        engine.set_content(id, content);
        let key = QueryKey::Parsed(id);

        let indexed_a = engine.indexed(id).unwrap();
        assert!(!engine.control.local.is_in_progress(key));

        let indexed_b = engine.indexed(id).unwrap();
        assert!(!engine.control.local.is_in_progress(key));

        assert_eq!(indexed_a, indexed_b);
    }

    #[test]
    fn test_cancellation_cleanup() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nlife = 42");
        let key = QueryKey::Indexed(id);

        // Simulate the current thread starting a computation.
        {
            let shard = engine.derived.indexed.shard(&id);
            shard.write().insert(id, DerivedState::in_progress(engine.control.id));
            engine.control.local.add_in_progress(key);
        }

        // Finally, enable cancellation and run the query on this thread.
        engine.control.global.cancelled.store(true, Ordering::Relaxed);
        let result =
            engine.query(key, id, |derived| &derived.indexed, |_| unreachable!("impossible."));

        assert_eq!(result, Err(QueryError::Cancelled));

        // Observe that the storage has been edited.
        {
            let shard = engine.derived.indexed.shard(&id);
            assert!(!shard.read().contains_key(&id));
        }
    }

    #[test]
    fn test_cancellation_no_cleanup() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nlife = 42");
        let key = QueryKey::Indexed(id);

        // Simulate the current thread starting a computation.
        {
            let shard = engine.derived.indexed.shard(&id);
            shard.write().insert(id, DerivedState::in_progress(engine.control.id));
            engine.control.local.add_in_progress(key);
        }

        // Finally, enable cancellation and run the query on another thread.
        engine.control.global.cancelled.store(true, Ordering::Relaxed);
        let result = std::thread::scope(|scope| {
            let runtime = engine.snapshot();
            let thread = scope.spawn(move || {
                runtime.query(key, id, |derived| &derived.indexed, |_| unreachable!("impossible."))
            });
            thread.join().unwrap()
        });

        assert_eq!(result, Err(QueryError::Cancelled));

        // Observe that the storage is not edited.
        {
            let shard = engine.derived.indexed.shard(&id);
            assert!(shard.read().contains_key(&id));
        }

        let result =
            engine.query(key, id, |derived| &derived.indexed, |_| unreachable!("impossible."));

        assert_eq!(result, Err(QueryError::Cancelled));

        // Finally, observe that the storage is edited.
        {
            let shard = engine.derived.indexed.shard(&id);
            assert!(!shard.read().contains_key(&id));
        }
    }

    #[test]
    fn test_cycle_detection() {
        const ID: FileId = FileId::from_raw(RawIdx::from_u32(0));
        const KEY: QueryKey = QueryKey::Resolved(ID);

        fn fake_query_a(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(QueryKey::Resolved(ID), ID, |derived| &derived.resolved, fake_query_a)
        }

        let engine = QueryEngine::default();
        let result = fake_query_a(&engine);
        assert_eq!(result, Err(QueryError::Cycle { stack: [KEY, KEY].into() }));
    }

    #[test]
    fn test_cycle_recovery() {
        const ID: FileId = FileId::from_raw(RawIdx::from_u32(0));

        fn fake_query_a(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(
                QueryKey::Resolved(ID),
                ID,
                |derived| &derived.resolved,
                |engine| fake_query_a(engine).map_err(|_| QueryError::Cancelled),
            )
        }

        let engine = QueryEngine::default();
        let result = fake_query_a(&engine);
        assert!(matches!(result, Err(QueryError::Cancelled)));
    }

    #[test]
    fn test_snapshot_cycle_detection() {
        const ID_A: FileId = FileId::from_raw(RawIdx::from_u32(0));
        const ID_B: FileId = FileId::from_raw(RawIdx::from_u32(1));

        fn fake_query_a(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(QueryKey::Resolved(ID_A), ID_A, |derived| &derived.resolved, fake_query_b)
        }

        fn fake_query_b(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(QueryKey::Resolved(ID_B), ID_B, |derived| &derived.resolved, fake_query_a)
        }

        let engine = QueryEngine::default();

        let snapshot = engine.snapshot();
        let thread = std::thread::spawn(move || fake_query_b(&snapshot));

        let result_a = fake_query_a(&engine);
        let result_b = thread.join().unwrap();

        assert!(result_a.is_err());
        assert!(result_b.is_err());

        // Either result can return `Cancelled`, but at least one of should be `Cycle`
        assert!(
            [result_a, result_b]
                .iter()
                .any(|result| matches!(result, Err(QueryError::Cycle { .. })))
        );
    }

    #[test]
    fn test_snapshot_cycle_recovery() {
        const ID_A: FileId = FileId::from_raw(RawIdx::from_u32(0));
        const ID_B: FileId = FileId::from_raw(RawIdx::from_u32(1));

        fn fake_query_a(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(
                QueryKey::Resolved(ID_A),
                ID_A,
                |derived| &derived.resolved,
                |engine| fake_query_b(engine).map_err(|_| QueryError::Cancelled),
            )
        }

        fn fake_query_b(engine: &QueryEngine) -> QueryResult<Arc<ResolvedModule>> {
            engine.query(
                QueryKey::Resolved(ID_B),
                ID_B,
                |derived| &derived.resolved,
                |engine| fake_query_a(engine).map_err(|_| QueryError::Cancelled),
            )
        }

        let engine = QueryEngine::default();

        let snapshot = engine.snapshot();
        let thread = std::thread::spawn(move || fake_query_b(&snapshot));

        let result_a = fake_query_a(&engine);
        let result_b = thread.join().unwrap();

        assert!(matches!(result_a, Err(QueryError::Cancelled)));
        assert!(matches!(result_b, Err(QueryError::Cancelled)));
    }

    #[test]
    fn test_resolving_cycle() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let main = files.insert("Main.purs", "module Main where\n\nimport Lib (b)\n\na = 123");
        let library = files.insert("Lib.purs", "module Lib where\n\nimport Main (a)\n\nb = 123");

        engine.set_content(main, files.content(main));
        engine.set_content(library, files.content(library));
        engine.set_module_file("Main", main);
        engine.set_module_file("Lib", library);

        let result_a = engine.resolved(main);
        assert_eq!(
            result_a,
            Err(QueryError::Cycle {
                stack: [
                    QueryKey::Resolved(main),
                    QueryKey::Resolved(library),
                    QueryKey::Resolved(main)
                ]
                .into()
            })
        );

        let result_b = engine.resolved(library);
        assert_eq!(
            result_b,
            Err(QueryError::Cycle {
                stack: [
                    QueryKey::Resolved(library),
                    QueryKey::Resolved(main),
                    QueryKey::Resolved(library)
                ]
                .into()
            })
        );
    }

    #[test]
    fn test_grouped_identity() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nx = y\ny = 1");
        let content = files.content(id);
        engine.set_content(id, content);

        let groups_a = engine.grouped(id).unwrap();
        let groups_b = engine.grouped(id).unwrap();
        assert!(Arc::ptr_eq(&groups_a, &groups_b));
    }

    #[test]
    fn test_lowered_identity() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nx = 1");
        let content = files.content(id);
        engine.set_content(id, content);

        let lowered_a = engine.lowered(id).unwrap();
        let lowered_b = engine.lowered(id).unwrap();
        assert!(Arc::ptr_eq(&lowered_a, &lowered_b));
    }

    #[test]
    fn test_grouped_stable() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nx = 1");
        engine.set_content(id, files.content(id));
        let groups_a = engine.grouped(id).unwrap();

        let id = files.insert("./src/Main.purs", "module Main where\n\n\n\nx = 1");
        engine.set_content(id, files.content(id));
        let groups_b = engine.grouped(id).unwrap();

        assert_eq!(groups_a.term_scc, groups_b.term_scc);
        assert_eq!(groups_a.type_scc, groups_b.type_scc);
    }

    #[test]
    fn test_elaboration_removes_expression_wrappers_and_operator_chains() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

add :: Int -> Int -> Int
add left _ = left

infixl 6 add as +

wrapped = ((1 :: Int))
symbolic = 1 + 2 + 3
backtick = 1 `add` 2
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let lowered = engine.lowered(id).unwrap();
        let grouped = engine.grouped(id).unwrap();
        let core = engine.elaborated(id).unwrap();

        let mut wrappers = 0;
        let mut chains = 0;
        for (expression, kind) in lowered.info.iter_expression() {
            match kind {
                ExpressionKind::Typed { expression: Some(inner), .. }
                | ExpressionKind::Parenthesized { parenthesized: Some(inner) } => {
                    wrappers += 1;
                    assert_eq!(
                        core.lookup_expression(expression),
                        core.lookup_expression(*inner),
                        "source-only wrappers must alias their inner Core expression",
                    );
                }
                ExpressionKind::OperatorChain { .. } | ExpressionKind::InfixChain { .. } => {
                    chains += 1;
                    let expression = core
                        .lookup_expression(expression)
                        .expect("operator expression must be elaborated");
                    assert!(matches!(core.expressions[expression], CoreExpression::Apply { .. }));
                }
                _ => {}
            }
        }

        assert!(wrappers >= 2);
        assert_eq!(chains, 2);
        assert_eq!(core.top_level.len(), grouped.term_scc.len());
        assert_eq!(
            core.items.len(),
            grouped.term_scc.iter().map(|group| group.as_slice().len()).sum()
        );
    }

    #[test]
    fn test_elaboration_preserves_explicit_type_arguments() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

identity :: forall @a. a -> a
identity value = value

visible = identity @Int 1
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);
        let core = engine.elaborated(id).unwrap();
        let argument = core.expressions.iter().find_map(|(_, expression)| {
            let CoreExpression::TypeApply { argument: CoreTypeArgument::Checked(argument), .. } =
                expression
            else {
                return None;
            };
            Some(*argument)
        });

        let argument = argument.expect("the visible application must retain its type argument");
        let rendered = checking::core::pretty::Pretty::new(&engine, &checked).render(argument);
        assert_eq!(rendered, "Int");
    }

    #[test]
    fn test_elaboration_orders_independent_bindings_by_source() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = "module Main where\n\nfirst = 1\nsecond = 2\nthird = 3";
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let indexed = engine.indexed(id).unwrap();
        let core = engine.elaborated(id).unwrap();
        let names = core
            .top_level
            .iter()
            .flat_map(|group| &core.binding_groups[*group].bindings)
            .filter_map(|binding| match core.bindings[*binding].source {
                elaborating::CoreBindingSource::Item(item) => indexed.items[item].name.as_deref(),
                elaborating::CoreBindingSource::Let(_)
                | elaborating::CoreBindingSource::Synthetic(_) => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(names, ["first", "second", "third"]);
    }

    #[test]
    fn test_elaboration_distinguishes_source_holes_from_missing_syntax() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let id = files.insert("./src/Main.purs", "module Main where\n\nhole = ?coreHole");
        engine.set_content(id, files.content(id));

        let core = engine.elaborated(id).unwrap();
        assert!(core.expressions.iter().any(|(_, expression)| matches!(
            expression,
            CoreExpression::Error(elaborating::CoreError::Hole)
        )));
    }

    #[test]
    fn test_elaboration_inserts_runtime_dictionary_evidence() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

class Eq a where
  eq :: a -> a -> Boolean

instance Eq Int where
  eq _ _ = true

concrete = eq 1 2
generalised left right = eq left right
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);
        assert!(checked.placements.applications.values().any(|evidence| !evidence.is_empty()));
        assert!(checked.placements.abstractions.values().any(|evidence| !evidence.is_empty()));

        let core = engine.elaborated(id).unwrap();
        assert!(core.expressions.iter().any(|(_, expression)| {
            matches!(expression, CoreExpression::Variable(CoreVariable::Instance(_)))
        }));
        assert!(core.expressions.iter().any(|(_, expression)| {
            matches!(expression, CoreExpression::Variable(CoreVariable::Evidence(_)))
        }));
    }

    #[test]
    fn test_elaboration_builds_constrained_instance_dictionary() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

data Box a = Box a

class Parent a where
  parent :: a -> Boolean

class (Partial, Parent a) <= Child a where
  child :: a -> Boolean
  childAgain :: a -> Boolean

instance parentBox :: Parent a => Parent (Box a) where
  parent _ = true

instance childBox :: (Partial, Parent a) => Child (Box a) where
  child _ = true
  childAgain value = child value
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);

        let core = engine.elaborated(id).unwrap();
        let (instance_origin, instance_binding) = core
            .instances
            .iter()
            .find(|entry| {
                let binding = *entry.1;
                let CoreBindingValue::Expression(expression) = core.bindings[binding].value else {
                    return false;
                };
                let CoreExpression::Lambda { body, .. } = core.expressions[expression] else {
                    return false;
                };
                matches!(
                    &core.expressions[body],
                    CoreExpression::Dictionary { superclasses, members }
                        if superclasses.len() == 2 && members.len() == 2
                )
            })
            .expect("the constrained Child instance must elaborate to a dictionary");

        assert!(
            matches!(instance_origin, InstanceCandidateOrigin::Instance(file, _) if *file == id)
        );
        let CoreBindingValue::Expression(instance_expression) =
            core.bindings[*instance_binding].value
        else {
            unreachable!("declared instances are expression bindings");
        };
        let CoreExpression::Lambda { pattern, body } = core.expressions[instance_expression] else {
            panic!("the prerequisite dictionary must abstract over the whole instance dictionary");
        };
        let CorePattern::Variable(CoreVariable::Evidence(prerequisite)) = core.patterns[pattern]
        else {
            panic!("the outer instance abstraction must bind its prerequisite dictionary");
        };
        let CoreExpression::Dictionary { superclasses, members } = &core.expressions[body] else {
            panic!("the shared prerequisite abstraction must immediately contain the dictionary");
        };

        assert_eq!(superclasses.len(), 2, "Child must retain both direct superclass slots");
        assert_eq!(
            superclasses[0],
            CoreSuperclassField::Erased,
            "compiler-known superclass slots remain explicit without a runtime expression",
        );
        assert_eq!(members.len(), 2);
        for member in members {
            if let CoreExpression::Lambda { pattern, .. } = core.expressions[member.value] {
                assert!(
                    !matches!(
                        core.patterns[pattern],
                        CorePattern::Variable(CoreVariable::Evidence(_))
                    ),
                    "the shared instance prerequisite must not be repeated on each member",
                );
            }
        }

        let CoreSuperclassField::Runtime(superclass) = superclasses[1] else {
            panic!("the runtime Parent superclass must not be erased");
        };
        let CoreExpression::Apply { function, argument } = core.expressions[superclass] else {
            panic!("the Parent superclass must use the local constrained Parent instance");
        };
        let CoreExpression::Variable(CoreVariable::Instance(parent_origin)) =
            core.expressions[function]
        else {
            panic!("superclass evidence must name the selected instance origin");
        };
        assert_eq!(
            core.expressions[argument],
            CoreExpression::Variable(CoreVariable::Evidence(prerequisite)),
            "the local Parent instance must receive the shared prerequisite dictionary",
        );
        assert!(
            matches!(parent_origin, InstanceCandidateOrigin::Instance(file, _) if file == id),
            "the superclass solver must select the module-local Parent instance",
        );
        assert!(
            core.instances.contains_key(&parent_origin),
            "instance evidence origins must resolve to their module-local Core binding",
        );

        let parent_binding = core.instances[&parent_origin];
        let group_of = |binding| {
            core.top_level
                .iter()
                .position(|group| core.binding_groups[*group].bindings.contains(&binding))
                .expect("every instance binding must belong to a top-level Core group")
        };
        assert!(
            group_of(parent_binding) < group_of(*instance_binding),
            "evidence-induced instance dependencies must participate in Core group ordering",
        );
        let child_group = core.top_level[group_of(*instance_binding)];
        assert!(
            core.binding_groups[child_group].recursive,
            "a member that selects its own instance must make the Core dictionary recursive",
        );
    }

    #[test]
    fn test_elaboration_scopes_ado_lets_inside_action_lambdas() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

map :: forall a b. (a -> b) -> a -> b
map function value = function value

apply :: forall a b. (a -> b) -> a -> b
apply function value = function value

pure :: forall a. a -> a
pure value = value

scoped = ado
  x <- pure 1
  let y = x
  in y
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);

        let core = engine.elaborated(id).unwrap();
        assert!(
            core.expressions.iter().any(|(_, expression)| {
                let CoreExpression::Lambda { body, .. } = expression else {
                    return false;
                };
                matches!(core.expressions[*body], CoreExpression::Let { .. })
            }),
            "an ado let after an action must be nested inside that action's continuation lambda"
        );
    }

    #[test]
    fn test_evidence_dedup_respects_local_dictionary_scopes() {
        fn resolved_binder(
            evidence: &Evidences,
            mut variable: EvidenceVarId,
        ) -> Option<checking::evidence::EvidenceBinderId> {
            for _ in 0..32 {
                let EvidenceState::Solved(term) = evidence.variable(variable).state else {
                    return None;
                };
                match evidence.evidence(term) {
                    Evidence::Variable(next) => variable = *next,
                    Evidence::Given(binder) => return Some(*binder),
                    Evidence::Instance { .. }
                    | Evidence::Superclass { .. }
                    | Evidence::Compiler => return None,
                }
            }
            panic!("evidence indirection must be acyclic");
        }

        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

class C a where
  member :: a

outer :: forall a. C a => { inner :: C a => a }
outer = { inner: member }

siblings :: forall a. C a => { left :: C a => a, right :: C a => a }
siblings = { left: member, right: member }
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);

        let mut local_binders = Vec::new();
        for (&site, binders) in &checked.placements.abstractions {
            let EvidenceAbstractionSite::Expression(expression) = site else {
                continue;
            };
            let applications = checked
                .placements
                .applications
                .get(&EvidenceApplicationSite::Expression(expression))
                .expect("the constrained field value must use its local dictionary");
            assert_eq!(binders.len(), 1);
            assert_eq!(applications.len(), 1);
            assert_eq!(
                resolved_binder(&checked.evidence, applications[0]),
                Some(binders[0]),
                "a wanted occurrence must resolve to the binder at its own expression site",
            );
            local_binders.push(binders[0]);
        }

        local_binders.sort_unstable();
        local_binders.dedup();
        assert_eq!(
            local_binders.len(),
            3,
            "the nested field and two sibling fields must retain distinct lexical binders",
        );
    }

    #[test]
    fn test_elaboration_transfers_derived_instance_requirements() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let eq = files.insert(
            "./src/Data.Eq.purs",
            r#"module Data.Eq where

class Eq a where
  eq :: a -> a -> Boolean
"#,
        );
        engine.set_content(eq, files.content(eq));
        engine.set_module_file("Data.Eq", eq);

        let main = files.insert(
            "./src/Main.purs",
            r#"module Main where

import Data.Eq (class Eq)

data Box a = Box a

derive instance Eq a => Eq (Box a)
"#,
        );
        engine.set_content(main, files.content(main));

        let checked = engine.checked(main).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);
        assert_eq!(checked.placements.derived_requirements.len(), 1);

        let core = engine.elaborated(main).unwrap();
        let (_, binding) = core
            .instances
            .iter()
            .find(|(origin, _)| matches!(origin, InstanceCandidateOrigin::Derive(file, _) if *file == main))
            .expect("the derived instance must have a module-local Core binding");
        assert!(core.binding_types.contains_key(binding));

        let CoreBindingValue::Expression(expression) = core.bindings[*binding].value else {
            panic!("derived dictionaries are semantic Core expressions");
        };
        let CoreExpression::Lambda { pattern, body } = core.expressions[expression] else {
            panic!("the declared Eq prerequisite must abstract over the derived dictionary");
        };
        let CorePattern::Variable(CoreVariable::Evidence(binder)) = core.patterns[pattern] else {
            panic!("the derived prerequisite must use an evidence binder");
        };
        let CoreExpression::DerivedDictionary { requirements, .. } = &core.expressions[body] else {
            panic!("a valid derive must elaborate to the derived-dictionary primitive");
        };
        assert_eq!(requirements.len(), 1);
        let CoreDerivedEvidence::Runtime(requirement) = requirements[0].evidence else {
            panic!("the generated field requirement must retain runtime evidence");
        };
        assert_eq!(
            core.expressions[requirement],
            CoreExpression::Variable(CoreVariable::Evidence(binder)),
        );
    }

    #[test]
    fn test_derived_delegate_requirements_bind_local_proof_assumptions() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let eq = files.insert(
            "./src/Data.Eq.purs",
            r#"module Data.Eq where

class Eq a where
  eq :: a -> a -> Boolean

class Eq1 f where
  eq1 :: forall a. Eq a => f a -> f a -> Boolean
"#,
        );
        engine.set_content(eq, files.content(eq));
        engine.set_module_file("Data.Eq", eq);

        let main = files.insert(
            "./src/Main.purs",
            r#"module Main where

import Data.Eq (class Eq, class Eq1)

data Id a = Id a

derive instance Eq a => Eq (Id a)
derive instance Eq1 Id
"#,
        );
        engine.set_content(main, files.content(main));

        let checked = engine.checked(main).unwrap();
        assert!(checked.errors.is_empty(), "{:#?}", checked.errors);

        let core = engine.elaborated(main).unwrap();
        let (local, requirement) = core
            .expressions
            .iter()
            .find_map(|(_, expression)| {
                let CoreExpression::DerivedDictionary { local_binders, requirements, .. } =
                    expression
                else {
                    return None;
                };
                match (local_binders.as_slice(), requirements.as_slice()) {
                    ([local], [requirement]) => Some((*local, *requirement)),
                    _ => None,
                }
            })
            .expect("the Eq1 delegate must expose its method-local Eq assumption");
        assert!(!local.erased);

        let CoreDerivedEvidence::Runtime(requirement) = requirement.evidence else {
            panic!("the delegate instance requirement must be retained at runtime");
        };
        let CoreExpression::Apply { function, argument } = core.expressions[requirement] else {
            panic!("the selected Eq (Id a) instance must receive the local Eq a assumption");
        };
        assert_eq!(
            core.expressions[argument],
            CoreExpression::Variable(CoreVariable::Evidence(local.binder)),
        );
        let CoreExpression::Variable(CoreVariable::Instance(origin)) = core.expressions[function]
        else {
            panic!("the delegate requirement must name its selected derived Eq instance");
        };
        assert!(core.instances.contains_key(&origin));
    }

    #[test]
    fn test_custom_failure_marks_wanted_evidence_as_errored() {
        let mut engine = QueryEngine::default();
        let mut files = Files::default();
        prim::configure(&mut engine, &mut files);

        let source = r#"module Main where

import Prim.TypeError (class Fail, Text)

boom :: Fail (Text "expected failure") => Int
boom = 1

use = boom
"#;
        let id = files.insert("./src/Main.purs", source);
        engine.set_content(id, files.content(id));

        let checked = engine.checked(id).unwrap();
        assert!(checked.errors.iter().any(|error| {
            matches!(error.kind, checking::error::ErrorKind::CustomFailure { .. })
        }));
        assert!(checked.evidence.variables().any(|(_, entry)| entry.state == EvidenceState::Error));
    }
}
