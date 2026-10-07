use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex, Weak,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::Duration,
};

use signex_resource::{
    DecodedResource, ResourceContext, ResourceDecoder, ResourceError, ResourceServer, ResourceType,
    ThreadedResourceServer,
};

static NEXT_DIR: AtomicUsize = AtomicUsize::new(0);
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn create_with_decoder(
    context: ResourceContext,
    decoder: Arc<dyn ResourceDecoder>,
) -> ThreadedResourceServer {
    for _ in 0..100 {
        match ThreadedResourceServer::with_decoder(context.clone(), decoder.clone()) {
            Ok(server) => return server,
            Err(ResourceError::AlreadyRunning) => thread::sleep(Duration::from_millis(10)),
            Err(error) => panic!("{error}"),
        }
    }
    panic!("previous resource server did not stop");
}

fn create(context: ResourceContext) -> ThreadedResourceServer {
    create_with_decoder(context, Arc::new(signex_resource::AssetDecoder))
}

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "signex-resource-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.0.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn context(&self) -> ResourceContext {
        ResourceContext::new(self.0.clone(), vec!["current".into(), "base".into()], 0).unwrap()
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[derive(Default)]
struct TextDecoder {
    calls: AtomicUsize,
    threads: Mutex<Vec<thread::ThreadId>>,
}

impl ResourceDecoder for TextDecoder {
    fn decode(
        &self,
        _kind: ResourceType,
        _path: &Path,
        bytes: &[u8],
    ) -> Result<DecodedResource, String> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.threads.lock().unwrap().push(thread::current().id());
        Ok(DecodedResource::Other(Box::new(
            String::from_utf8(bytes.to_vec()).unwrap(),
        )))
    }
}

fn text(data: &DecodedResource) -> &str {
    match data {
        DecodedResource::Other(value) => value.downcast_ref::<String>().unwrap(),
        _ => panic!("expected text"),
    }
}

#[test]
fn resolves_each_append_before_trying_the_next() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.png", b"current png");
    dir.put("base/g00/hero.g00", b"base g00");
    let decoder = Arc::new(TextDecoder::default());
    let server = create_with_decoder(dir.context(), decoder.clone());
    let handle = server.open(ResourceType::Picture, "hero").unwrap();
    assert_eq!(text(&server.get_data(&handle).unwrap()), "current png");
    assert_ne!(decoder.threads.lock().unwrap()[0], thread::current().id());
}

#[test]
fn context_requires_an_absolute_root() {
    assert!(matches!(
        ResourceContext::new("relative-root".into(), vec![PathBuf::new()], 0),
        Err(ResourceError::InvalidContext)
    ));
}

#[test]
fn resolves_sound_voice_and_movie_families() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/wav/theme.ogg", b"sound");
    dir.put("base/wav/theme.wav", b"later sound");
    dir.put("current/koe/2036/z203600522.nwa", b"voice");
    dir.put("current/mov/opening.omv", b"object movie");
    dir.put("current/mov/opening.mpg", b"system movie");
    let server = create_with_decoder(dir.context(), Arc::new(TextDecoder::default()));

    let sound = server.open(ResourceType::Sound, "theme").unwrap();
    let voice = server.open(ResourceType::Voice, "203600522").unwrap();
    let object = server.open(ResourceType::ObjectMovie, "opening").unwrap();
    let system = server.open(ResourceType::SystemMovie, "opening").unwrap();
    assert_eq!(text(&server.get_data(&sound).unwrap()), "sound");
    assert_eq!(text(&server.get_data(&voice).unwrap()), "voice");
    assert_eq!(text(&server.get_data(&object).unwrap()), "object movie");
    assert_eq!(text(&server.get_data(&system).unwrap()), "system movie");
    assert_ne!(object.resource().id, system.resource().id);
    assert!(matches!(
        server.open(ResourceType::Voice, "not-a-number"),
        Err(ResourceError::InvalidName)
    ));
}

#[test]
fn font_and_thumbnails_use_their_original_directories() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/dat/title.ttf", b"font");
    dir.put("savedata/0007.bmp", b"save thumb");
    dir.put("savedata/0000000007.png", b"thumb");
    dir.put("current/0007.g00", b"wrong append");
    let context = dir.context().with_save_dir(dir.0.join("savedata")).unwrap();
    let server = create_with_decoder(context, Arc::new(TextDecoder::default()));

    let font = server.open(ResourceType::Font, "title.ttf").unwrap();
    let save = server.open(ResourceType::SaveThumbnail, "7").unwrap();
    let thumb = server.open(ResourceType::Thumbnail, "7").unwrap();
    assert_eq!(text(&server.get_data(&font).unwrap()), "font");
    assert_eq!(text(&server.get_data(&save).unwrap()), "save thumb");
    assert_eq!(text(&server.get_data(&thumb).unwrap()), "thumb");
    assert_ne!(save.resource().id, thumb.resource().id);
}

#[test]
fn independent_handles_preserve_the_record_and_reload_preserves_old_revision() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.g00", b"first");
    let server = create_with_decoder(dir.context(), Arc::new(TextDecoder::default()));
    let logic = server.open(ResourceType::Picture, "hero").unwrap();
    let render = server.open(ResourceType::Picture, "hero").unwrap();
    assert_eq!(logic.resource(), render.resource());
    drop(logic);
    let first_data = server.get_data(&render).unwrap();
    assert_eq!(text(&first_data), "first");

    dir.put("current/g00/hero.g00", b"second");
    let newer = server.reload(ResourceType::Picture, "hero").unwrap();
    assert_eq!(render.resource().id, newer.resource().id);
    assert_eq!(
        newer.resource().revision.0,
        render.resource().revision.0 + 1
    );
    assert_eq!(text(&first_data), "first");
    assert_eq!(text(&server.get_data(&newer).unwrap()), "second");
}

#[test]
fn non_g00_data_is_released_after_first_successful_get() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/dat/title.ttf", b"first");
    let decoder = Arc::new(TextDecoder::default());
    let server = create_with_decoder(dir.context(), decoder.clone());
    let handle = server.open(ResourceType::Font, "title.ttf").unwrap();

    let first = server.get_data(&handle).unwrap();
    assert_eq!(text(&first), "first");
    dir.put("current/dat/title.ttf", b"second");
    let second = server.get_data(&handle).unwrap();
    assert_eq!(text(&second), "second");
    assert_eq!(text(&first), "first");
    assert_eq!(decoder.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn dropping_logic_handle_does_not_release_render_handle_on_another_thread() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.g00", b"frame");
    let server = Arc::new(create_with_decoder(
        dir.context(),
        Arc::new(TextDecoder::default()),
    ));
    let logic = server.open(ResourceType::Picture, "hero").unwrap();
    let render = server.open(ResourceType::Picture, "hero").unwrap();
    let render_server = server.clone();
    let (continue_sender, continue_receiver) = mpsc::channel();
    let renderer = thread::spawn(move || {
        continue_receiver.recv().unwrap();
        text(&render_server.get_data(&render).unwrap()).to_owned()
    });
    drop(logic);
    continue_sender.send(()).unwrap();
    assert_eq!(renderer.join().unwrap(), "frame");
}

#[test]
fn evicted_resource_reads_its_original_path() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("base/g00/hero.g00", b"base");
    let server = create_with_decoder(dir.context(), Arc::new(TextDecoder::default()));
    let handle = server.open(ResourceType::Picture, "hero").unwrap();
    assert_eq!(text(&server.get_data(&handle).unwrap()), "base");
    dir.put("current/g00/hero.g00", b"new current");
    dir.put("base/g00/hero.g00", b"updated base");
    server.evict(&handle).unwrap();
    assert_eq!(text(&server.get_data(&handle).unwrap()), "updated base");
}

#[test]
fn missing_and_unsupported_data_are_explicit_errors() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    let server = create(dir.context());
    assert!(matches!(
        server.open(ResourceType::Picture, "missing"),
        Err(ResourceError::NotFound { .. })
    ));
    dir.put("current/g00/hero.png", b"not decoded");
    let handle = server.open(ResourceType::Picture, "hero").unwrap();
    assert!(
        matches!(server.get_data(&handle), Err(ResourceError::DecodeFailed { path, .. }) if path.ends_with("hero.png"))
    );
}

#[test]
fn reload_revision_survives_dropping_the_last_handle() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.g00", b"first");
    let server = create_with_decoder(dir.context(), Arc::new(TextDecoder::default()));
    let old = server.open(ResourceType::Picture, "hero").unwrap();
    let id = old.resource().id;
    drop(old);
    let newer = server.reload(ResourceType::Picture, "hero").unwrap();
    assert_eq!(newer.resource().id, id);
    assert_eq!(newer.resource().revision.0, 1);
}

struct BlockingDecoder {
    started: mpsc::Sender<()>,
    gate: Arc<(Mutex<bool>, Condvar)>,
}

impl ResourceDecoder for BlockingDecoder {
    fn decode(
        &self,
        _kind: ResourceType,
        _path: &Path,
        bytes: &[u8],
    ) -> Result<DecodedResource, String> {
        if bytes == b"block" {
            let _ = self.started.send(());
            let (lock, ready) = &*self.gate;
            let mut open = lock.lock().unwrap();
            while !*open {
                open = ready.wait(open).unwrap();
            }
        }
        Ok(DecodedResource::Other(Box::new(
            String::from_utf8(bytes.to_vec()).unwrap(),
        )))
    }
}

#[test]
fn open_can_finish_while_another_decode_is_running() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/first.g00", b"block");
    dir.put("current/g00/second.g00", b"second");
    let (started_sender, started_receiver) = mpsc::channel();
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let decoder = Arc::new(BlockingDecoder {
        started: started_sender,
        gate: gate.clone(),
    });
    let server = Arc::new(create_with_decoder(dir.context(), decoder));
    let first = server.open(ResourceType::Picture, "first").unwrap();
    let first_server = server.clone();
    let waiter = thread::spawn(move || first_server.get_data(&first));
    started_receiver
        .recv_timeout(Duration::from_secs(2))
        .unwrap();

    let second_server = server.clone();
    let (reply, receiver) = mpsc::channel();
    let opener = thread::spawn(move || {
        let _ = reply.send(second_server.open(ResourceType::Picture, "second"));
    });
    let opened_before_decode_finished = receiver.recv_timeout(Duration::from_secs(1));
    *gate.0.lock().unwrap() = true;
    gate.1.notify_all();
    let second = opened_before_decode_finished
        .expect("open waited for an unrelated decode")
        .unwrap();
    opener.join().unwrap();
    assert_eq!(text(&waiter.join().unwrap().unwrap()), "block");
    assert_eq!(text(&server.get_data(&second).unwrap()), "second");
}

fn one_pixel_g00() -> Vec<u8> {
    let mut bytes = vec![0];
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&12u32.to_le_bytes());
    bytes.extend_from_slice(&4u32.to_le_bytes());
    bytes.extend_from_slice(&[0x01, 1, 2, 3]);
    bytes
}

#[test]
fn g00_decoder_does_not_depend_on_file_extension() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.png", &one_pixel_g00());
    let server = create(dir.context());
    let handle = server.open(ResourceType::Picture, "hero").unwrap();
    let first = server.get_data(&handle).unwrap();
    assert!(matches!(first.as_ref(), DecodedResource::G00(_)));
    let second = server.get_data(&handle).unwrap();
    assert!(Arc::ptr_eq(&first, &second));
}

#[test]
fn only_the_last_g00_stays_cached_after_eviction() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/a.g00", &one_pixel_g00());
    dir.put("current/g00/b.g00", &one_pixel_g00());
    let server = create(dir.context());
    let a = server.open(ResourceType::Picture, "a").unwrap();
    assert!(matches!(
        server.get_data(&a).unwrap().as_ref(),
        DecodedResource::G00(_)
    ));
    dir.put("current/g00/a.g00", b"broken");
    server.evict(&a).unwrap();
    assert!(matches!(
        server.get_data(&a).unwrap().as_ref(),
        DecodedResource::G00(_)
    ));

    let b = server.open(ResourceType::Picture, "b").unwrap();
    server.get_data(&b).unwrap();
    assert!(matches!(
        server.get_data(&a),
        Err(ResourceError::DecodeFailed { .. })
    ));
}

#[test]
fn only_one_server_runs_until_its_last_handle_is_dropped() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/a.g00", &one_pixel_g00());
    let server = create(dir.context());
    let handle = server.open(ResourceType::Picture, "a").unwrap();
    assert!(matches!(
        ThreadedResourceServer::new(dir.context()),
        Err(ResourceError::AlreadyRunning)
    ));
    drop(server);
    assert!(matches!(
        ThreadedResourceServer::new(dir.context()),
        Err(ResourceError::AlreadyRunning)
    ));
    drop(handle);
    let replacement = create(dir.context());
    let replacement_handle = replacement.open(ResourceType::Picture, "a").unwrap();
    assert!(matches!(
        replacement.get_data(&replacement_handle).unwrap().as_ref(),
        DecodedResource::G00(_)
    ));
}

#[derive(Default)]
struct ReentrantDecoder {
    server: Mutex<Option<Weak<ThreadedResourceServer>>>,
}

impl ResourceDecoder for ReentrantDecoder {
    fn decode(
        &self,
        _kind: ResourceType,
        _path: &Path,
        _bytes: &[u8],
    ) -> Result<DecodedResource, String> {
        self.server
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .upgrade()
            .unwrap()
            .open(ResourceType::Picture, "hero")
            .unwrap();
        unreachable!()
    }
}

#[test]
fn reentrant_decoder_panics_and_wakes_data_waiter() {
    let _test_lock = TEST_LOCK.lock().unwrap();
    let dir = TestDir::new();
    dir.put("current/g00/hero.g00", b"content");
    let decoder = Arc::new(ReentrantDecoder::default());
    let server = Arc::new(create_with_decoder(dir.context(), decoder.clone()));
    *decoder.server.lock().unwrap() = Some(Arc::downgrade(&server));
    let handle = server.open(ResourceType::Picture, "hero").unwrap();
    assert!(matches!(
        server.get_data(&handle),
        Err(ResourceError::WorkerStopped)
    ));
}
