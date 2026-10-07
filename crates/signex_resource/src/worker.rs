use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use crate::decoder::{DecodeJob, DecodeResult, decode_job};
use crate::{
    DecodedResource, ResourceContext, ResourceDecoder, ResourceError, ResourceHandle, ResourceId,
    ResourceRef, ResourceRevision, ResourceServer, ResourceType,
};

type Key = (ResourceType, String);
type DataReply = mpsc::Sender<Result<Arc<DecodedResource>, ResourceError>>;

static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

struct SingletonLease;

impl SingletonLease {
    fn acquire() -> Result<Self, ResourceError> {
        SERVER_RUNNING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| ResourceError::AlreadyRunning)
    }
}

impl Drop for SingletonLease {
    fn drop(&mut self) {
        SERVER_RUNNING.store(false, Ordering::Release);
    }
}

pub(crate) enum Command {
    Open {
        kind: ResourceType,
        name: String,
        reload: bool,
        reply: mpsc::Sender<Result<ResourceRef, ResourceError>>,
    },
    GetData {
        resource: ResourceRef,
        reply: DataReply,
    },
    Evict {
        resource: ResourceRef,
        reply: mpsc::Sender<Result<(), ResourceError>>,
    },
    Release(ResourceRef),
}

struct Record {
    key: Key,
    path: PathBuf,
    refs: u64,
    state: State,
    waiters: Vec<DataReply>,
}

enum State {
    Pending,
    Ready(Arc<DecodedResource>),
    Evicted,
    Failed(ResourceError),
}

struct Worker {
    context: ResourceContext,
    decoder_sender: mpsc::Sender<DecodeJob>,
    current: HashMap<Key, ResourceRef>,
    records: HashMap<ResourceRef, Record>,
    ids: HashMap<ResourceId, Key>,
    revisions: HashMap<ResourceId, u64>,
    pending: usize,
    last_g00: Option<(ResourceRef, Arc<DecodedResource>)>,
}

impl Worker {
    fn new(context: ResourceContext, decoder_sender: mpsc::Sender<DecodeJob>) -> Self {
        Self {
            context,
            decoder_sender,
            current: HashMap::new(),
            records: HashMap::new(),
            ids: HashMap::new(),
            revisions: HashMap::new(),
            pending: 0,
            last_g00: None,
        }
    }

    fn run(
        mut self,
        receiver: mpsc::Receiver<Command>,
        completed: mpsc::Receiver<(ResourceRef, DecodeResult)>,
    ) {
        loop {
            loop {
                match completed.try_recv() {
                    Ok((resource, result)) => self.complete(resource, result),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        self.fail_pending();
                        break;
                    }
                }
            }
            let command = if self.pending == 0 {
                receiver.recv().ok()
            } else {
                match receiver.recv_timeout(Duration::from_millis(5)) {
                    Ok(command) => Some(command),
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => None,
                }
            };
            if let Some(command) = command {
                self.command(command);
            } else {
                break;
            }
        }
    }

    fn command(&mut self, command: Command) {
        match command {
            Command::Open {
                kind,
                name,
                reload,
                reply,
            } => {
                let _ = reply.send(self.open(kind, name, reload));
            }
            Command::GetData { resource, reply } => self.get_data(resource, reply),
            Command::Evict { resource, reply } => {
                let _ = reply.send(self.evict(resource));
            }
            Command::Release(resource) => self.release(resource),
        }
    }

    fn open(
        &mut self,
        kind: ResourceType,
        name: String,
        reload: bool,
    ) -> Result<ResourceRef, ResourceError> {
        let key = (kind, name);
        if !reload {
            if let Some(resource) = self.current.get(&key).copied() {
                self.records
                    .get_mut(&resource)
                    .expect("current resource has record")
                    .refs += 1;
                return Ok(resource);
            }
        }
        let path = self.context.resolve(kind, &key.1)?;
        let id = stable_id(&key);
        if self.ids.get(&id).is_some_and(|existing| existing != &key) {
            return Err(ResourceError::IdCollision(id));
        }
        let revision = if reload {
            self.revisions
                .get(&id)
                .copied()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(ResourceError::RevisionOverflow(id))?
        } else {
            self.revisions.get(&id).copied().unwrap_or(0)
        };
        let resource = ResourceRef {
            id,
            revision: ResourceRevision(revision),
        };
        self.enqueue(DecodeJob {
            resource,
            kind,
            path: path.clone(),
        })?;
        self.ids.insert(id, key.clone());
        self.revisions.insert(id, revision);
        self.current.insert(key.clone(), resource);
        self.records.insert(
            resource,
            Record {
                key,
                path,
                refs: 1,
                state: State::Pending,
                waiters: Vec::new(),
            },
        );
        Ok(resource)
    }

    fn get_data(&mut self, resource: ResourceRef, reply: DataReply) {
        if let Some((cached, data)) = &self.last_g00 {
            if *cached == resource && self.records.contains_key(&resource) {
                let _ = reply.send(Ok(Arc::clone(data)));
                return;
            }
        }
        let Some(record) = self.records.get_mut(&resource) else {
            let _ = reply.send(Err(self.missing(resource)));
            return;
        };
        match &record.state {
            State::Ready(data) => {
                let data = Arc::clone(data);
                record.state = State::Evicted;
                let _ = reply.send(Ok(data));
            }
            State::Failed(error) => {
                let _ = reply.send(Err(error.clone()));
            }
            State::Pending => record.waiters.push(reply),
            State::Evicted => {
                let job = DecodeJob {
                    resource,
                    kind: record.key.0,
                    path: record.path.clone(),
                };
                match self.decoder_sender.send(job) {
                    Ok(()) => {
                        self.pending += 1;
                        record.state = State::Pending;
                        record.waiters.push(reply);
                    }
                    Err(_) => {
                        let _ = reply.send(Err(ResourceError::WorkerStopped));
                    }
                }
            }
        }
    }

    fn evict(&mut self, resource: ResourceRef) -> Result<(), ResourceError> {
        let Some(record) = self.records.get_mut(&resource) else {
            return Err(self.missing(resource));
        };
        if matches!(record.state, State::Ready(_)) {
            record.state = State::Evicted;
        }
        Ok(())
    }

    fn release(&mut self, resource: ResourceRef) {
        let Some(record) = self.records.get_mut(&resource) else {
            return;
        };
        record.refs -= 1;
        if record.refs != 0 {
            return;
        }
        let record = self.records.remove(&resource).expect("record exists");
        if self.current.get(&record.key) == Some(&resource) {
            self.current.remove(&record.key);
        }
        if self
            .last_g00
            .as_ref()
            .is_some_and(|(cached, _)| *cached == resource)
        {
            self.last_g00 = None;
        }
        for waiter in record.waiters {
            let _ = waiter.send(Err(ResourceError::UnknownResource(resource)));
        }
    }

    fn enqueue(&mut self, job: DecodeJob) -> Result<(), ResourceError> {
        self.decoder_sender
            .send(job)
            .map_err(|_| ResourceError::WorkerStopped)?;
        self.pending += 1;
        Ok(())
    }

    fn complete(&mut self, resource: ResourceRef, result: DecodeResult) {
        self.pending -= 1;
        if let Some(record) = self.records.get_mut(&resource) {
            match &result {
                Ok(data) => {
                    if matches!(data.as_ref(), DecodedResource::G00(_)) {
                        self.last_g00 = Some((resource, Arc::clone(data)));
                        record.state = State::Evicted;
                    } else if record.waiters.is_empty() {
                        record.state = State::Ready(Arc::clone(data));
                    } else {
                        record.state = State::Evicted;
                    }
                }
                Err(error) => record.state = State::Failed(error.clone()),
            }
            for waiter in record.waiters.drain(..) {
                let _ = waiter.send(result.clone());
            }
        }
    }

    fn fail_pending(&mut self) {
        if self.pending == 0 {
            return;
        }
        self.pending = 0;
        for record in self.records.values_mut() {
            if matches!(record.state, State::Pending) {
                record.state = State::Failed(ResourceError::WorkerStopped);
                for waiter in record.waiters.drain(..) {
                    let _ = waiter.send(Err(ResourceError::WorkerStopped));
                }
            }
        }
    }

    fn missing(&self, requested: ResourceRef) -> ResourceError {
        if let Some(current) = self
            .current
            .values()
            .find(|resource| resource.id == requested.id)
        {
            ResourceError::StaleRevision {
                requested,
                current: *current,
            }
        } else {
            ResourceError::UnknownResource(requested)
        }
    }
}

fn stable_id(key: &Key) -> ResourceId {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let (tag, other) = match key.0 {
        ResourceType::Picture => (0_u8, 0),
        ResourceType::NumberAtlas => (1, 0),
        ResourceType::WeatherAtlas => (2, 0),
        ResourceType::Mesh => (3, 0),
        ResourceType::SaveThumbnail => (4, 0),
        ResourceType::Thumbnail => (5, 0),
        ResourceType::Font => (6, 0),
        ResourceType::Mask => (7, 0),
        ResourceType::Sound => (9, 0),
        ResourceType::Voice => (10, 0),
        ResourceType::ObjectMovie => (11, 0),
        ResourceType::SystemMovie => (12, 0),
        ResourceType::Other(value) => (8, value),
    };
    for byte in [tag]
        .into_iter()
        .chain(other.to_le_bytes())
        .chain(key.1.bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    ResourceId(hash)
}

pub struct ThreadedResourceServer {
    sender: mpsc::Sender<Command>,
    worker_id: thread::ThreadId,
    decoder_id: thread::ThreadId,
}

impl ThreadedResourceServer {
    pub fn new(context: ResourceContext) -> Result<Self, ResourceError> {
        Self::with_decoder(context, Arc::new(crate::AssetDecoder))
    }

    pub fn with_decoder(
        context: ResourceContext,
        decoder: Arc<dyn ResourceDecoder>,
    ) -> Result<Self, ResourceError> {
        let lease = SingletonLease::acquire()?;
        let (sender, receiver) = mpsc::channel();
        let (job_sender, job_receiver) = mpsc::channel();
        let (completed_sender, completed_receiver) = mpsc::channel();
        let (decoder_id_sender, decoder_id_receiver) = mpsc::sync_channel(1);
        let decoder_thread = thread::Builder::new()
            .name("signex-resource-decode".into())
            .spawn(move || {
                let _ = decoder_id_sender.send(thread::current().id());
                while let Ok(job) = job_receiver.recv() {
                    let _ = completed_sender.send(decode_job(decoder.as_ref(), job));
                }
            })
            .map_err(|_| ResourceError::WorkerStopped)?;
        let (id_sender, id_receiver) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("signex-resource".into())
            .spawn(move || {
                let _ = id_sender.send(thread::current().id());
                Worker::new(context, job_sender).run(receiver, completed_receiver);
                let _ = decoder_thread.join();
                drop(lease);
            })
            .map_err(|_| ResourceError::WorkerStopped)?;
        Ok(Self {
            sender,
            worker_id: id_receiver
                .recv()
                .map_err(|_| ResourceError::WorkerStopped)?,
            decoder_id: decoder_id_receiver
                .recv()
                .map_err(|_| ResourceError::WorkerStopped)?,
        })
    }

    pub fn reload(&self, kind: ResourceType, name: &str) -> Result<ResourceHandle, ResourceError> {
        self.open_inner(kind, name, true)
    }

    pub fn evict(&self, handle: &ResourceHandle) -> Result<(), ResourceError> {
        self.check_reentrant();
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(Command::Evict {
                resource: handle.resource,
                reply,
            })
            .map_err(|_| ResourceError::WorkerStopped)?;
        receiver.recv().map_err(|_| ResourceError::WorkerStopped)?
    }

    fn check_reentrant(&self) {
        if thread::current().id() == self.worker_id || thread::current().id() == self.decoder_id {
            panic!("ResourceServer called from its worker thread");
        }
    }

    fn open_inner(
        &self,
        kind: ResourceType,
        name: &str,
        reload: bool,
    ) -> Result<ResourceHandle, ResourceError> {
        self.check_reentrant();
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(Command::Open {
                kind,
                name: name.to_owned(),
                reload,
                reply,
            })
            .map_err(|_| ResourceError::WorkerStopped)?;
        let resource = receiver
            .recv()
            .map_err(|_| ResourceError::WorkerStopped)??;
        Ok(ResourceHandle {
            resource,
            sender: self.sender.clone(),
        })
    }
}

impl ResourceServer for ThreadedResourceServer {
    fn open(&self, kind: ResourceType, name: &str) -> Result<ResourceHandle, ResourceError> {
        self.open_inner(kind, name, false)
    }

    fn get_data(&self, handle: &ResourceHandle) -> Result<Arc<DecodedResource>, ResourceError> {
        self.check_reentrant();
        let (reply, receiver) = mpsc::channel();
        self.sender
            .send(Command::GetData {
                resource: handle.resource,
                reply,
            })
            .map_err(|_| ResourceError::WorkerStopped)?;
        receiver.recv().map_err(|_| ResourceError::WorkerStopped)?
    }
}
