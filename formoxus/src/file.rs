use std::any::Any;
use std::fmt::Debug;
use std::fs::{self, File, OpenOptions};
use std::future::ready;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::{env, io};

use dioxus::html::FileData;
use dioxus_fullstack::StatusCode;
use dioxus_fullstack::extract::multipart::{Field, MultipartError};
use dioxus_fullstack::extract::{FromRequest, Multipart as AxumMultipart, Request};
use dioxus_fullstack::response::{IntoResponse, Response};
use dioxus_fullstack::{ClientRequest, ClientResponse, IntoRequest, RequestError};
use facet::{Facet, Partial, ReflectError};

use crate::path::NodePath;
use crate::wire::WireForm;

/// What `accept:` holds. Defined in `formoxus-spec`, so `form!` runs the same
/// parser at expansion; re-exported here so a dev can name it.
pub use formoxus_spec::{FileType, FileTypeError};

static UNIQUE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct StoredFile {
    value: Arc<dyn Any + Send + Sync>,
    type_name: &'static str,
    write: for<'p> fn(&(dyn Any + Send + Sync), Partial<'p>) -> Result<Partial<'p>, ReflectError>,
}

impl StoredFile {
    /// Erase `stored`'s type, keeping what's needed to write it back: the value,
    /// its type's name, and `write_stored` made for exactly this `S`. All three
    /// come from the one `S`, so the function can never be paired with a value
    /// of another type.
    ///
    /// `for<'a> Facet<'a>`, not `Facet<'static>`: `Partial<'p>::set` needs
    /// `S: Facet<'p>` for whatever `'p` the write happens at, and a derived
    /// type implements `Facet<'a>` for every `'a`, which this bound says.
    pub(crate) fn new<S>(stored: S) -> Self
    where
        S: for<'a> Facet<'a> + Clone + Send + Sync + 'static,
    {
        Self {
            value: Arc::new(stored),
            type_name: std::any::type_name::<S>(),
            // Naming `S` here makes the compiler build one copy of
            // `write_stored` for this type; the pointer is that copy.
            write: write_stored::<S>,
        }
    }

    pub(crate) fn write_into<'p>(&self, partial: Partial<'p>) -> Result<Partial<'p>, ReflectError> {
        (self.write)(&*self.value, partial)
    }
}

/// The type's name is all there is to show: the value is a `dyn Any`, and the
/// function pointer's address means nothing to a reader.
impl Debug for StoredFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredFile")
            .field("type_name", &self.type_name)
            .finish_non_exhaustive()
    }
}

/// `StoredFile::write` for one `S`: get the `S` back out of the `dyn Any`,
/// clone it (the `StoredFile` keeps its own) and set it into `partial`, which
/// `FileNode` has opened at `Received`'s `stored` field.
fn write_stored<'p, S>(
    value: &(dyn Any + Send + Sync),
    partial: Partial<'p>,
) -> Result<Partial<'p>, ReflectError>
where
    S: for<'a> Facet<'a> + Clone + Send + Sync + 'static,
{
    // A dev bug, not a user's doing: the model's `FileUpload<S>` and the
    // handler's `Stored` differ. Decision BB catches it before anything is
    // stored; this is the assert that BB's check was not skipped.
    assert!(
        partial.shape() == <S as Facet<'static>>::SHAPE,
        "this form's `FileUpload` holds `{}`, but the upload handler stores `{}`; \
         give `accept_multipart` a handler whose `Stored` is the model's `S`",
        partial.shape(),
        std::any::type_name::<S>(),
    );
    let stored = value
        .downcast_ref::<S>()
        .expect("`StoredFile::new` pairs each value with the `write_stored` for its type");
    partial.set(stored.clone())
}

/// What a [`FileNode`](crate::nodes::FileNode) knows about a file beyond its name: what `max_size` and
/// `accept` check. Present only for a file picked in this form or received by
/// the server; an `Existing` file has none.
#[derive(Clone, Debug)]
pub struct FileMeta {
    /// As the browser or the request reported it, which may be `None` or `""`
    /// for a type it didn't know.
    pub content_type: Option<String>,
    /// In bytes. On the client the browser's figure; on the server the count
    /// of bytes actually received, which is the one enforced.
    pub size: u64,
    pub stored: Option<StoredFile>,
}

/// `prefix`, this process's id and a counter: never the same twice in one
/// process, and different processes on one host have different ids.
fn unique_name(prefix: &str) -> String {
    let n = UNIQUE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{}-{n}", std::process::id())
}

/// A size as the "too large" message shows it: decimal units (1 kB is 1,000
/// bytes, as browsers and operating systems show sizes), at most one decimal
/// place, no trailing `.0`. `2 MB`, `1.5 MB`, `500 kB`, `999 bytes`.
///
/// ROUNDED DOWN, never to nearest: a limit of 2,099,999 bytes shown as "2.1 MB"
/// would promise a user more than it allows. Integer division gives that for
/// free, which is why this isn't `f64` and `{:.1}` (that rounds to nearest and
/// keeps the `.0`).
pub(crate) fn format_size(bytes: u64) -> String {
    const UNITS: [(u64, &str); 3] = [(1_000_000_000, "GB"), (1_000_000, "MB"), (1_000, "kB")];
    let Some(&(unit, name)) = UNITS.iter().find(|&&(unit, _)| bytes >= unit) else {
        return if bytes == 1 {
            "1 byte".to_string()
        } else {
            format!("{bytes} bytes")
        };
    };
    // `bytes / (unit / 10)`, not `bytes * 10 / unit`: the same number, but
    // multiplying first overflows past about 1.8 × 10^18 bytes.
    let tenths = bytes / (unit / 10);
    let (whole, tenth) = (tenths / 10, tenths % 10);
    if tenth == 0 {
        format!("{whole} {name}")
    } else {
        format!("{whole}.{tenth} {name}")
    }
}

#[derive(Clone, Debug, Facet, PartialEq)]
#[repr(u8)]
pub enum FileUpload<S = TemporaryUploadedFile> {
    Chosen {
        name: String,
        content_type: Option<String>,
        size: u64,
    },
    Received {
        name: String,
        content_type: Option<String>,
        size: u64,
        stored: S,
    },
    Existing {
        name: String,
    },
}

impl<S> FileUpload<S> {
    pub fn name(&self) -> &str {
        match self {
            FileUpload::Chosen { name, .. }
            | FileUpload::Received { name, .. }
            | FileUpload::Existing { name } => name,
        }
    }
}

/// What [`TemporaryUploadedFile::move_to`] does when `dest` already exists.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IfExists {
    /// Leave both files where they are and return `AlreadyExists`.
    Fail,
    /// Replace `dest`, atomically: the old file is there until the new one is.
    Replace,
    /// Keep both: `essay.pdf` is taken, so try `essay-2.pdf`, `essay-3.pdf`, ….
    Version,
}

/// A file received by `TemporaryFileUploadHandler`, in a temporary file until
/// [`move_to`](Self::move_to) puts it somewhere permanent. Deleted when the
/// last clone is dropped, unless it has been moved.
#[derive(Clone, Debug, Facet)]
pub struct TemporaryUploadedFile {
    // Opaque: facet has no shape for an `Arc` of a guard, and needs none, since
    // nothing reflects into a stored file.
    #[facet(opaque)]
    guard: Arc<Guard>,
}

/// Clones are one file, so equal when they share a guard. Not by comparing
/// paths: `x == x` would lock the same `Mutex` twice and deadlock.
impl PartialEq for TemporaryUploadedFile {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.guard, &other.guard)
    }
}

impl TemporaryUploadedFile {
    /// For the handler, once the file is fully written to `path`.
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            guard: Arc::new(Guard {
                location: Mutex::new(Location::Temporary(path)),
            }),
        }
    }

    /// Where the file is now: its temporary path, or wherever it was moved.
    pub fn path(&self) -> PathBuf {
        self.lock().path().to_path_buf()
    }

    /// Move the file to `dest` and stop deleting it on drop, for every clone.
    /// Returns where it ended up, which only `IfExists::Version` can change.
    ///
    /// Never build `dest` from the user's file name alone: two users can send
    /// the same name.
    ///
    /// Blocking I/O. On the temporary file's own filesystem this is a rename
    /// and instant; across filesystems it copies the whole file, so call it in
    /// `spawn_blocking` there, or use `in_dir` to keep it a rename.
    pub fn move_to(&self, dest: impl AsRef<Path>, if_exists: IfExists) -> io::Result<PathBuf> {
        // Held across the move, so two clones moving at once take turns, and
        // the second moves the file from where the first put it.
        let mut location = self.lock();
        let from = location.path().to_path_buf();
        let to = move_file(&from, dest.as_ref(), if_exists)?;
        *location = Location::Moved(to.clone());
        Ok(to)
    }

    fn lock(&self) -> MutexGuard<'_, Location> {
        // Poisoned only by a panic while locked, and `location` is written
        // only after a move succeeds, so what it holds is still true.
        self.guard
            .location
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Debug)]
struct Guard {
    location: Mutex<Location>,
}

/// Where the file is, and so whether dropping the guard deletes it. One
/// value, not a path and a flag, because `move_to` changes both at once.
#[derive(Debug)]
enum Location {
    Temporary(PathBuf),
    Moved(PathBuf),
}

impl Location {
    fn path(&self) -> &Path {
        match self {
            Self::Temporary(path) | Self::Moved(path) => path,
        }
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let location = self
            .location
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        if let Location::Temporary(path) = location {
            // Nowhere to report an error from `drop`, and a temporary file
            // already gone is no harm.
            let _ = fs::remove_file(path);
        }
    }
}

fn move_file(from: &Path, dest: &Path, if_exists: IfExists) -> io::Result<PathBuf> {
    match if_exists {
        IfExists::Fail => move_new(from, dest).map(|()| dest.to_path_buf()),
        IfExists::Replace => replace(from, dest).map(|()| dest.to_path_buf()),
        IfExists::Version => {
            let mut candidate = dest.to_path_buf();
            let mut n = 1;
            loop {
                match move_new(from, &candidate) {
                    Ok(()) => return Ok(candidate),
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                        n += 1;
                        candidate = versioned(dest, n);
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

/// Move to `to` only if nothing is there, atomically. Not "check, then
/// rename": `rename` overwrites on Unix, so two moves at once would both pass
/// the check. `hard_link` and `create_new` each fail if `to` exists.
fn move_new(from: &Path, to: &Path) -> io::Result<()> {
    match fs::hard_link(from, to) {
        Ok(()) => {}
        // Another filesystem, or one without hard links (FAT, some network
        // mounts): copy instead.
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::CrossesDevices | io::ErrorKind::Unsupported
            ) =>
        {
            copy_new(from, to)?;
        }
        Err(e) => return Err(e),
    }
    // The file is at `to`, so the move happened. A temporary copy left behind
    // is a leak for the sweep, not a reason to report failure.
    let _ = fs::remove_file(from);
    Ok(())
}

/// Move to `to`, replacing what is there in one step.
fn replace(from: &Path, to: &Path) -> io::Result<()> {
    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::CrossesDevices => {
            // Copy to a new name BESIDE `to`, on its filesystem, then rename
            // that over `to`, which is atomic.
            let staging = staging_path(to);
            copy_new(from, &staging)?;
            if let Err(e) = fs::rename(&staging, to) {
                let _ = fs::remove_file(&staging);
                return Err(e);
            }
            let _ = fs::remove_file(from);
            Ok(())
        }
        Err(e) => Err(e),
    }
}

/// Copy `from` into a new file at `to`, failing if `to` exists, and flush it
/// to disk. On any error after `to` is created, `to` is removed again.
fn copy_new(from: &Path, to: &Path) -> io::Result<()> {
    // `out` closes at the end of this block, before `to` is removed below:
    // Windows refuses to delete a file that is still open. A scope rather than
    // `drop(out)`, which clippy flags on wasm32 (`drop_non_drop`), where
    // `std::fs::File` is a stub with no `Drop`.
    let copied = {
        let mut out = OpenOptions::new().write(true).create_new(true).open(to)?;
        File::open(from)
            .and_then(|mut input| io::copy(&mut input, &mut out))
            .and_then(|_| out.sync_all())
    };
    if copied.is_err() {
        let _ = fs::remove_file(to);
    }
    copied
}

/// `essay.pdf` with `n = 2` is `essay-2.pdf`; `a.tar.gz` is `a.tar-2.gz`.
/// Built from `OsStr`s, so a name that isn't UTF-8 survives.
fn versioned(dest: &Path, n: u32) -> PathBuf {
    let mut name = dest.file_stem().unwrap_or_default().to_os_string();
    name.push(format!("-{n}"));
    if let Some(extension) = dest.extension() {
        name.push(".");
        name.push(extension);
    }
    dest.with_file_name(name)
}

/// A hidden name beside `to` that nothing else will choose (see
/// `unique_name`). `create_new` refuses it anyway if it were taken.
fn staging_path(to: &Path) -> PathBuf {
    let mut name = std::ffi::OsString::from(".");
    name.push(to.file_name().unwrap_or_default());
    name.push(".");
    name.push(unique_name(""));
    name.push(".part");
    to.with_file_name(name)
}

pub trait FileUploadHandler: Send + Sync {
    type Stored: Clone + Debug + PartialEq + for<'a> Facet<'a> + Send + Sync + 'static;
    type Writer: FileUploadWriter<Stored = Self::Stored> + Send;

    fn new_file(
        &self,
        path: &NodePath,
        name: &str,
        content_type: Option<&str>,
    ) -> impl Future<Output = io::Result<Self::Writer>> + Send;

    /// Delete (or try to delete) a file this handler stored. Because the function
    /// returns (), it is up to the individual handler to deal with any possible
    /// errors that occur during deleting.
    fn delete(&self, stored: Self::Stored) -> impl Future<Output = ()> + Send;
}

pub trait FileUploadWriter {
    type Stored;

    fn write_chunk(&mut self, chunk: &[u8]) -> impl Future<Output = io::Result<()>> + Send;

    fn finish(self) -> impl Future<Output = io::Result<Self::Stored>> + Send;

    fn abort(self) -> impl Future<Output = ()> + Send;
}

#[derive(Clone, Debug)]
pub struct TemporaryFileUploadHandler {
    dir: PathBuf,
}

impl TemporaryFileUploadHandler {
    pub fn new() -> Self {
        Self {
            dir: env::temp_dir(),
        }
    }

    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// A new, empty file in `dir`, under a name of formoxus's own (never the
    /// user's file name: see `unique_name`). `create_new` refuses a name
    /// already taken (another server sharing the directory), so try again.
    fn create_file(&self) -> io::Result<TemporaryFileWriter> {
        loop {
            let path = self.dir.join(unique_name("formoxus-upload-"));
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok(TemporaryFileWriter {
                        file,
                        path: Some(path),
                    });
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }
    }
}

impl Default for TemporaryFileUploadHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl FileUploadHandler for TemporaryFileUploadHandler {
    type Stored = TemporaryUploadedFile;
    type Writer = TemporaryFileWriter;

    // Every method here does its (blocking) work when called and returns a
    // future that is already finished, since there is nothing to await. See
    // WIDGETS_PLAN 11c, decision 1, for why not `async fn`.
    fn new_file(
        &self,
        _path: &NodePath,
        _name: &str,
        _content_type: Option<&str>,
    ) -> impl Future<Output = io::Result<Self::Writer>> + Send {
        ready(self.create_file())
    }

    fn delete(&self, stored: Self::Stored) -> impl Future<Output = ()> + Send {
        // The guard deletes the file once the last clone is dropped, which is
        // when `accept_multipart` drops the rejected form's state.
        drop(stored);
        ready(())
    }
}

#[derive(Debug)]
pub struct TemporaryFileWriter {
    /// Open while chunks arrive; closed when the writer is dropped.
    file: File,
    /// Where `file` is. `Some` until `finish` takes it, so `Drop` knows
    /// whether there's still a half-written file to delete.
    path: Option<PathBuf>,
}

impl Drop for TemporaryFileWriter {
    fn drop(&mut self) {
        // Still `Some`: never finished. Aborted, or the request's future was
        // dropped mid-stream (the client went away).
        if let Some(path) = self.path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

impl FileUploadWriter for TemporaryFileWriter {
    type Stored = TemporaryUploadedFile;

    fn write_chunk(&mut self, chunk: &[u8]) -> impl Future<Output = io::Result<()>> + Send {
        ready(self.file.write_all(chunk))
    }

    fn finish(mut self) -> impl Future<Output = io::Result<Self::Stored>> + Send {
        let path = self
            .path
            .take()
            .expect("`finish` consumes the writer, so `path` is still `Some`");
        ready(Ok(TemporaryUploadedFile::new(path)))
    }

    fn abort(self) -> impl Future<Output = ()> + Send {
        // `self` drops when this returns, and its `Drop` removes the file.
        ready(())
    }
}

/// The user's file name as formoxus keeps it (`WIDGETS_PLAN.md`, "The user's
/// file name"): Django's `sanitize_file_name` without its `html.unescape`.
/// The last segment after any `/` or `\`, with control characters and
/// [`hides_text`] characters dropped; `None` if that leaves `""`, `.` or `..`.
/// Called on the name in the wire part, never the file part's `filename=`.
pub(crate) fn clean_file_name(raw: &str) -> Option<String> {
    // `rsplit` yields from the END, so its first item is the last segment.
    // (`next_back` or `last` would walk to the front: the FIRST segment.)
    let file_name = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = file_name
        .chars()
        .filter(|c| !c.is_control() && !hides_text(*c))
        .collect();
    if cleaned.is_empty() || &cleaned == "." || &cleaned == ".." {
        None
    } else {
        Some(cleaned)
    }
}

/// The invisible characters that can make a file name look like another:
/// the bidi controls, which reorder what follows (`U+202E` shows
/// `fdp.exe` as `exe.pdf`), the zero-width ones, the byte-order mark and
/// the soft hyphen.
///
/// A hand list, not Django's whole rule. Django keeps only `isprintable`
/// characters, dropping every Unicode format character (`Cf`, about 170)
/// and every separator but the space; Rust has no such predicate, and no
/// crate here gives general categories. These are the ones used to spoof
/// a name (decided 2026-11-17, Todd).
const fn hides_text(c: char) -> bool {
    matches!(
        c,
        '\u{00AD}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// How [`store_part`] ended for one file part.
#[derive(Debug)]
pub(crate) enum PartOutcome<Stored> {
    /// Stored whole. `size` is the bytes COUNTED, the one `max_size` is
    /// enforced against, never the size the wire part stated.
    Stored { stored: Stored, size: u64 },
    /// Past `max_size`: aborted. `size` is what was counted when it passed,
    /// so it is over `max_size`, which is all `FileNode::validate` needs.
    TooLarge { size: u64 },
    /// The request passed the body limit (a 413, trap 12). The writer was
    /// aborted; the request can't be read any further.
    BodyLimit,
    /// The request broke off or was malformed (any other error from `chunk`).
    /// The writer was aborted; the request can't be read any further.
    Broken(
        #[expect(dead_code, reason = "to be logged; the driver drops it for now")] MultipartError,
    ),
    /// The handler failed (`new_file`, `write_chunk` or `finish`). Whatever it
    /// had written was aborted. The request can still be read and answered.
    Failed(#[expect(dead_code, reason = "to be logged; the driver drops it for now")] io::Error),
}

/// Read one file part, through `handler`, counting, until it ends, passes
/// `max_size`, or something fails.
///
/// **It does not drain what it leaves unread, because multer does:** the
/// next `next_field` reads through the rest of an unfinished part first
/// (multer 3.1.0, `multipart.rs`, from line 270). So what drains a REQUEST,
/// which a server must do before answering or the reply is lost
/// (`WIDGETS_PLAN.md` box 10 (d)), is the driver calling `next_field` until
/// `None`, including on every early return.
///
/// `name` is the cleaned name from the wire part, never the part's own
/// `filename=` (see "The name comes from the wire part" in the plan).
/// Checking a STATED size and `accept` is the caller's, before calling this,
/// so a file refused up front never reaches `new_file`.
pub(crate) async fn store_part<H: FileUploadHandler>(
    field: &mut Field<'_>,
    handler: &H,
    path: &NodePath,
    name: &str,
    content_type: Option<&str>,
    max_size: Option<u64>,
) -> PartOutcome<H::Stored> {
    let mut writer = match handler.new_file(path, name, content_type).await {
        Ok(writer) => writer,
        Err(e) => return PartOutcome::Failed(e),
    };
    let mut size: u64 = 0;
    loop {
        let chunk = match field.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(e) => {
                writer.abort().await;
                return if is_body_limit(&e) {
                    PartOutcome::BodyLimit
                } else {
                    PartOutcome::Broken(e)
                };
            }
        };
        size += chunk.len() as u64;
        if max_size.is_some_and(|max| size > max) {
            writer.abort().await;
            return PartOutcome::TooLarge { size };
        }
        if let Err(e) = writer.write_chunk(&chunk).await {
            writer.abort().await;
            return PartOutcome::Failed(e);
        }
    }
    match writer.finish().await {
        Ok(stored) => PartOutcome::Stored { stored, size },
        Err(e) => PartOutcome::Failed(e),
    }
}

/// Read the rest of a request, part by part, discarding it: what the driver
/// does before every early return, since a server that answers before
/// reading the whole request loses its reply (`WIDGETS_PLAN.md` box 10 (d)).
/// Each `next_field` reads through the part before it (see [`store_part`]).
///
/// Stops at the first error: past the body limit, or broken off, the request
/// can't be read any further, and there is nothing left to drain.
pub(crate) async fn drain(multipart: &mut AxumMultipart) {
    while let Ok(Some(_)) = multipart.next_field().await {}
}

/// Whether a multipart error is the body limit (axum's 413, trap 12). For
/// `store_part`'s `chunk` errors and for the driver's `next_field` ones,
/// since the limit can trip in either: wherever multer's buffer is reading
/// when the body passes it.
pub(crate) fn is_body_limit(e: &MultipartError) -> bool {
    e.status() == StatusCode::PAYLOAD_TOO_LARGE
}

/// The name of a multipart request's first part, the form's [`WireForm`] as
/// JSON. Every other part is a file, named by its path's string
/// (`answers.#3.file`).
///
/// `pub(crate)` while both ends are formoxus's own (`WIDGETS_PLAN.md` 11g,
/// decision MM). It would need to be public for a dev to build a request by
/// hand, say to test their own server function.
pub(crate) const WIRE_PART: &str = "__formoxus_wire_form";

/// A form with files on its way to the server: what a server function takes
/// in place of a [`WireForm`] when the form has a
/// [`FileUpload`] field. Made by
/// [`Form::to_multipart`](crate::form::Form::to_multipart).
///
/// One request: first the wire part (the values, the errors, and each sent
/// file's size), then one part per file the user picked.
///
/// **On the server** it is the server function's argument as well, since an
/// argument is one type on both ends. Taking it out of the request reads
/// nothing yet: the parts stay in the request until
/// [`Submission::accept_multipart`](crate::submission::Submission::accept_multipart)
/// reads them, once, storing each file as it arrives. So a server function
/// passes it straight there. axum's 2 MB default body limit applies, and
/// `accept_multipart`'s docs say how to raise it.
pub struct Multipart<T>(pub(crate) MultipartFormData<T>);

impl<T> Debug for Multipart<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            MultipartFormData::OnClient { wire_form, files } => f
                .debug_struct("Multipart::OnClient")
                .field("wire_form", wire_form)
                .field("files", files)
                .finish(),
            MultipartFormData::OnServer(axum_mp) => {
                f.debug_tuple("Multipart::OnServer").field(axum_mp).finish()
            }
        }
    }
}

/// What [`Form::to_multipart`](crate::form::Form::to_multipart) put in, for
/// the suite, which is outside the crate and so cannot see the fields.
///
/// Test-only (`__test-util`), not public: nothing but the suite needs them
/// yet, and on 11h's server half they would have nothing to borrow until the
/// request is read, so a public version would have to return `Option`s.
/// Making them public later breaks nothing (`WIDGETS_PLAN.md` 11g, LL).
#[cfg(any(test, feature = "__test-util"))]
impl<T> Multipart<T> {
    /// The wire part, sizes included, before it is serialized.
    ///
    /// # Panics
    ///
    /// On a `Multipart` read on the server, whose wire part is still unread
    /// in the request: only `accept_multipart` reads it, once.
    pub fn wire(&self) -> &WireForm<T> {
        match &self.0 {
            MultipartFormData::OnClient { wire_form, .. } => wire_form,
            MultipartFormData::OnServer(_) => panic!(
                "a `Multipart` read on the server has no wire until \
                 `accept_multipart` reads it"
            ),
        }
    }

    /// The file parts, in the order they are sent. None on a `Multipart`
    /// read on the server, since nothing has been read from it yet.
    pub fn files(&self) -> impl Iterator<Item = (&NodePath, &FileData)> {
        let files: &[(NodePath, FileData)] = match &self.0 {
            MultipartFormData::OnClient { files, .. } => files,
            MultipartFormData::OnServer(_) => &[],
        };
        files.iter().map(|(path, file)| (path, file))
    }
}

/// What a [`Multipart`] holds, by which end it is on. Private, so 11h adds
/// the server's variant without changing anything public. Both variants hold
/// what the CLIENT sent; they are named for where the value lives.
// `allow`, not `expect`: on wasm32 `OnClient` is small enough that the lint
// doesn't fire, and an unfulfilled `expect` is an error there.
#[allow(
    clippy::large_enum_variant,
    reason = "one per submit, moved once into the request; boxing saves nothing that matters"
)]
pub(crate) enum MultipartFormData<T> {
    /// Built by `to_multipart`, not yet serialized: [`IntoRequest`] does
    /// that, at the send.
    OnClient {
        wire_form: WireForm<T>,
        files: Vec<(NodePath, FileData)>,
    },
    OnServer(AxumMultipart),
}

/// `T: 'static` because the returned future owns the `WireForm<T>`, whose
/// `PhantomData<fn() -> T>` is `'static` only when `T` is. Every model is
/// `Facet<'static>`, so it costs nothing.
impl<T: 'static> IntoRequest for Multipart<T> {
    async fn into_request(self, req: ClientRequest) -> Result<ClientResponse, RequestError> {
        match self.0 {
            MultipartFormData::OnClient { wire_form, files } => {
                let json = serde_json::to_string(&wire_form)
                    .map_err(|e| RequestError::Serialization(e.to_string()))?;
                send_parts(req, json, files).await
            }
            MultipartFormData::OnServer(_) => Err(RequestError::Builder(
                "a `Multipart` read on the server can't be sent".into(),
            )),
        }
    }
}

impl<S: Send + Sync + 'static, T> FromRequest<S> for Multipart<T> {
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match AxumMultipart::from_request(req, state).await {
            Ok(mp) => Ok(Multipart(MultipartFormData::OnServer(mp))),
            Err(e) => Err(e.into_response()),
        }
    }
}

/// In a browser: the browser's own `FormData`, which streams each file from
/// disk itself, so wasm never holds the bytes.
#[cfg(target_arch = "wasm32")]
async fn send_parts(
    req: ClientRequest,
    json: String,
    files: Vec<(NodePath, FileData)>,
) -> Result<ClientResponse, RequestError> {
    let js_error = |e: wasm_bindgen::JsValue| RequestError::Builder(format!("{e:?}"));
    let form_data = web_sys::FormData::new().map_err(js_error)?;
    form_data
        .append_with_str(WIRE_PART, &json)
        .map_err(js_error)?;
    for (path, file) in &files {
        let Some(browser_file) = file.inner().downcast_ref::<web_sys::File>() else {
            return Err(RequestError::Builder(format!("{path}: not a browser file")));
        };
        form_data
            .append_with_blob_and_filename(&path.to_string(), browser_file, &browser_file.name())
            .map_err(js_error)?;
    }
    req.send_js_value(form_data.into()).await
}

/// On desktop and mobile: dioxus's own `send_multipart`, because
/// `ClientResponse`'s fields are private and only dioxus can build one.
#[cfg(not(target_arch = "wasm32"))]
async fn send_parts(
    req: ClientRequest,
    json: String,
    files: Vec<(NodePath, FileData)>,
) -> Result<ClientResponse, RequestError> {
    use dioxus_html::{FormData, SerializedFileData, SerializedFormData, SerializedFormObject};

    let mut parts = vec![SerializedFormObject {
        key: WIRE_PART.to_string(),
        text: Some(json),
        file: None,
    }];
    parts.extend(files.iter().map(|(path, file)| SerializedFormObject {
        key: path.to_string(),
        text: None,
        file: Some(SerializedFileData {
            path: file.path(),
            size: file.size(),
            last_modified: file.last_modified(),
            content_type: file.content_type(),
            contents: None,
        }),
    }));
    let form_data = FormData::new(SerializedFormData::new(String::new(), parts));
    req.send_multipart(&form_data).await
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use googletest::prelude::*;

    use super::{
        IfExists, TemporaryUploadedFile, clean_file_name, copy_new, format_size, staging_path,
        versioned,
    };

    #[gtest]
    fn a_size_is_shown_in_decimal_units_rounded_down() {
        for (bytes, shown) in [
            (0, "0 bytes"),
            (1, "1 byte"),
            (999, "999 bytes"),
            (1_000, "1 kB"),
            (1_999, "1.9 kB"),
            (500_000, "500 kB"),
            (999_999, "999.9 kB"),
            (1_500_000, "1.5 MB"),
            (2_000_000, "2 MB"),
            (2_050_000, "2 MB"),
            (2_099_999, "2 MB"),
            (10_250_000, "10.2 MB"),
            (5_000_000_000, "5 GB"),
            (2_500_000_000_000, "2500 GB"),
        ] {
            expect_that!(format_size(bytes), eq(shown), "{bytes}");
        }
    }

    /// The reason for dividing before multiplying: this would overflow
    /// computed as `bytes * 10 / unit`.
    #[gtest]
    fn the_largest_size_does_not_overflow() {
        expect_that!(format_size(u64::MAX), eq("18446744073.7 GB"));
    }

    /// A fresh directory for one test, removed when the test ends. Named for
    /// the test and this process, since tests run in parallel threads of one
    /// process.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(test: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("fx-file-{test}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).expect("the system temp dir is writable");
            Self(dir)
        }

        fn path(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }

        /// A temporary upload holding `body`, as the handler would leave one.
        fn upload(&self, name: &str, body: &str) -> TemporaryUploadedFile {
            let path = self.path(name);
            fs::write(&path, body).expect("the scratch dir is writable");
            TemporaryUploadedFile::new(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn read(path: &Path) -> String {
        fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    #[gtest]
    fn a_temporary_file_is_deleted_once_its_last_clone_is_dropped() {
        let scratch = Scratch::new("drop");
        let upload = scratch.upload("tmp", "body");
        let path = upload.path();
        let clone = upload.clone();

        drop(upload);
        expect_true!(path.exists(), "a clone still holds it");
        drop(clone);
        expect_false!(path.exists());
    }

    #[gtest]
    fn a_moved_file_survives_every_clone_being_dropped() {
        let scratch = Scratch::new("moved");
        let upload = scratch.upload("tmp", "body");
        let clone = upload.clone();

        let to = upload
            .move_to(scratch.path("kept"), IfExists::Fail)
            .expect("nothing is at the destination");
        drop(upload);
        drop(clone);

        expect_that!(read(&to), eq("body"));
        expect_false!(
            scratch.path("tmp").exists(),
            "the move removes the temporary file"
        );
    }

    #[gtest]
    fn a_move_is_seen_by_every_clone() {
        let scratch = Scratch::new("clones");
        let upload = scratch.upload("tmp", "body");
        let clone = upload.clone();

        upload
            .move_to(scratch.path("kept"), IfExists::Fail)
            .expect("nothing is at the destination");

        expect_that!(clone.path(), eq(&scratch.path("kept")));
    }

    #[gtest]
    fn fail_leaves_an_existing_destination_and_the_upload_where_they_were() {
        let scratch = Scratch::new("fail");
        fs::write(scratch.path("essay.pdf"), "old").expect("the scratch dir is writable");
        let upload = scratch.upload("tmp", "new");

        let refused = upload.move_to(scratch.path("essay.pdf"), IfExists::Fail);

        expect_that!(
            refused.map_err(|e| e.kind()),
            err(eq(&io::ErrorKind::AlreadyExists))
        );
        expect_that!(read(&scratch.path("essay.pdf")), eq("old"));
        expect_that!(upload.path(), eq(&scratch.path("tmp")));
        drop(upload);
        expect_false!(
            scratch.path("tmp").exists(),
            "a refused move leaves it temporary, so it is still deleted"
        );
    }

    #[gtest]
    fn replace_overwrites_an_existing_destination() {
        let scratch = Scratch::new("replace");
        fs::write(scratch.path("essay.pdf"), "old").expect("the scratch dir is writable");
        let upload = scratch.upload("tmp", "new");

        let to = upload
            .move_to(scratch.path("essay.pdf"), IfExists::Replace)
            .expect("replacing is allowed");

        expect_that!(to, eq(&scratch.path("essay.pdf")));
        expect_that!(read(&to), eq("new"));
        expect_that!(upload.path(), eq(&to));
    }

    #[gtest]
    fn version_counts_up_from_two_past_every_taken_name() {
        let scratch = Scratch::new("version");
        fs::write(scratch.path("essay.pdf"), "first").expect("the scratch dir is writable");

        let second = scratch
            .upload("tmp2", "second")
            .move_to(scratch.path("essay.pdf"), IfExists::Version)
            .expect("a free version exists");
        let third = scratch
            .upload("tmp3", "third")
            .move_to(scratch.path("essay.pdf"), IfExists::Version)
            .expect("a free version exists");

        expect_that!(second, eq(&scratch.path("essay-2.pdf")));
        expect_that!(third, eq(&scratch.path("essay-3.pdf")));
        expect_that!(read(&scratch.path("essay.pdf")), eq("first"));
        expect_that!(read(&second), eq("second"));
        expect_that!(read(&third), eq("third"));
    }

    #[gtest]
    fn version_takes_the_plain_name_while_it_is_free() {
        let scratch = Scratch::new("version-free");

        let to = scratch
            .upload("tmp", "body")
            .move_to(scratch.path("essay.pdf"), IfExists::Version)
            .expect("nothing is there");

        expect_that!(to, eq(&scratch.path("essay.pdf")));
    }

    /// `x == x` would deadlock if equality locked both guards' paths.
    #[gtest]
    fn clones_are_equal_and_two_uploads_are_not() {
        let scratch = Scratch::new("eq");
        let one = scratch.upload("one", "body");
        let other = scratch.upload("other", "body");

        expect_that!(one, eq(&one));
        expect_that!(one.clone(), eq(&one));
        expect_that!(one, not(eq(&other)));
    }

    /// The cross-filesystem fallback, called directly: a test can't count on a
    /// second filesystem being mounted.
    #[gtest]
    fn the_copy_fallback_refuses_an_existing_file_and_copies_to_a_new_one() {
        let scratch = Scratch::new("copy");
        fs::write(scratch.path("from"), "body").expect("the scratch dir is writable");
        fs::write(scratch.path("taken"), "old").expect("the scratch dir is writable");

        expect_that!(
            copy_new(&scratch.path("from"), &scratch.path("taken")).map_err(|e| e.kind()),
            err(eq(io::ErrorKind::AlreadyExists))
        );
        expect_that!(read(&scratch.path("taken")), eq("old"));

        copy_new(&scratch.path("from"), &scratch.path("fresh")).expect("nothing is there");
        expect_that!(read(&scratch.path("fresh")), eq("body"));
    }

    #[gtest]
    fn a_version_goes_before_the_last_extension() {
        expect_that!(
            versioned(Path::new("d/essay.pdf"), 2),
            eq(Path::new("d/essay-2.pdf"))
        );
        expect_that!(
            versioned(Path::new("d/a.tar.gz"), 2),
            eq(Path::new("d/a.tar-2.gz"))
        );
        expect_that!(
            versioned(Path::new("d/README"), 3),
            eq(Path::new("d/README-3"))
        );
        expect_that!(
            versioned(Path::new("d/.bashrc"), 3),
            eq(Path::new("d/.bashrc-3"))
        );
    }

    #[gtest]
    fn a_staging_name_is_hidden_beside_its_destination_and_never_repeats() {
        let first = staging_path(Path::new("d/essay.pdf"));
        let second = staging_path(Path::new("d/essay.pdf"));

        expect_that!(first.parent(), some(eq(Path::new("d"))));
        expect_that!(
            first.file_name().and_then(|n| n.to_str()),
            some(starts_with(".essay.pdf."))
        );
        expect_that!(first, not(eq(&second)));
    }

    // ── `clean_file_name` (WIDGETS_PLAN 11h, h.4) ───────────────────────────
    //
    // Django's `sanitize_file_name` without its `html.unescape`. The input is
    // the name in the wire part, never the file part's `filename=` (see "The
    // name comes from the wire part" in the plan).

    #[gtest]
    fn a_plain_name_is_kept() {
        expect_that!(clean_file_name("essay.pdf"), some(eq("essay.pdf")));
    }

    #[gtest]
    fn a_unix_path_keeps_its_last_segment() {
        expect_that!(
            clean_file_name("/home/todd/essay.pdf"),
            some(eq("essay.pdf"))
        );
    }

    /// What old browsers on Windows sent, and the reason for splitting on `\`
    /// on every server, not only on Windows ones.
    #[gtest]
    fn a_windows_path_keeps_its_last_segment() {
        expect_that!(
            clean_file_name(r"C:\Users\todd\essay.pdf"),
            some(eq("essay.pdf"))
        );
    }

    #[gtest]
    fn mixed_separators_keep_the_last_segment() {
        expect_that!(
            clean_file_name(r"C:\Users/todd\drafts/essay.pdf"),
            some(eq("essay.pdf"))
        );
    }

    /// Nothing is escaped or unescaped: the wire part is JSON, so the name
    /// arrives as the client read it.
    #[gtest]
    fn quotes_spaces_and_non_ascii_are_kept() {
        expect_that!(
            clean_file_name(r#"my "best" résumé ✓.pdf"#),
            some(eq(r#"my "best" résumé ✓.pdf"#))
        );
    }

    /// No `html.unescape`, unlike Django: a `FormData` is always UTF-8, so an
    /// `&amp;` is part of the real name.
    #[gtest]
    fn an_html_entity_is_left_alone() {
        expect_that!(
            clean_file_name("Tom &amp; Jerry.pdf"),
            some(eq("Tom &amp; Jerry.pdf"))
        );
    }

    #[gtest]
    fn control_characters_are_dropped() {
        expect_that!(
            clean_file_name("es\u{0}say\r\n\t.pdf\u{7f}"),
            some(eq("essay.pdf"))
        );
    }

    #[gtest]
    fn a_dot_file_and_three_dots_are_names() {
        expect_that!(clean_file_name(".hidden"), some(eq(".hidden")));
        expect_that!(clean_file_name("..."), some(eq("...")));
    }

    #[gtest]
    fn an_empty_name_is_refused() {
        expect_that!(clean_file_name(""), none());
    }

    #[gtest]
    fn dot_and_dot_dot_are_refused() {
        expect_that!(clean_file_name("."), none());
        expect_that!(clean_file_name(".."), none());
    }

    /// The LAST segment is what's judged, so a path ending in `..` is refused
    /// however harmless the rest looks.
    #[gtest]
    fn a_path_ending_in_dot_dot_is_refused() {
        expect_that!(clean_file_name("uploads/.."), none());
        expect_that!(clean_file_name(r"uploads\.."), none());
    }

    /// Nothing after the last separator: the last segment is empty.
    #[gtest]
    fn a_path_ending_in_a_separator_is_refused() {
        expect_that!(clean_file_name("uploads/"), none());
        expect_that!(clean_file_name(r"C:\Users\"), none());
    }

    #[gtest]
    fn a_name_of_only_control_characters_is_refused() {
        expect_that!(clean_file_name("\u{7}\n"), none());
    }

    /// `U+202E` (right-to-left override) would show this as `exe.pdf`.
    #[gtest]
    fn a_bidi_override_is_dropped() {
        expect_that!(
            clean_file_name("invoice\u{202E}fdp.exe"),
            some(eq("invoicefdp.exe"))
        );
        expect_that!(
            clean_file_name("a\u{202A}b\u{2066}c\u{2069}d"),
            some(eq("abcd"))
        );
    }

    #[gtest]
    fn zero_width_characters_are_dropped() {
        expect_that!(
            clean_file_name("\u{FEFF}ess\u{200B}ay\u{200D}\u{2060}.pdf"),
            some(eq("essay.pdf"))
        );
    }

    #[gtest]
    fn a_soft_hyphen_is_dropped() {
        expect_that!(clean_file_name("es\u{00AD}say.pdf"), some(eq("essay.pdf")));
    }

    /// Only the hand list goes: a no-break space, which Django would drop, is
    /// kept (see `hides_text`).
    #[gtest]
    fn a_no_break_space_is_kept() {
        expect_that!(
            clean_file_name("my\u{00A0}essay.pdf"),
            some(eq("my\u{00A0}essay.pdf"))
        );
    }

    #[gtest]
    fn a_name_of_only_invisible_characters_is_refused() {
        expect_that!(clean_file_name("\u{200B}\u{202E}"), none());
    }

    /// The check comes AFTER the control characters go, or `.\0.` would pass
    /// as a name and then be `..`.
    #[gtest]
    fn dot_dot_hidden_by_a_control_character_is_refused() {
        expect_that!(clean_file_name(".\u{0}."), none());
    }

    // ── `store_part` (WIDGETS_PLAN 11h, h.6) ────────────────────────────────
    //
    // A multipart request built in memory and read through axum's own
    // extractor, which needs no runtime, so `block_on` runs it. The body
    // arrives in 1,000-byte frames, so a large part comes as several chunks
    // and the counting and the abort happen mid-part.
    //
    // The handler records what was called. `TemporaryFileWriter` deletes its
    // file on drop with or without `abort`, so an empty directory could not
    // show that `abort` was called; the log can.

    use std::sync::{Arc, Mutex};

    use dioxus_fullstack::body::Body;
    use dioxus_fullstack::extract::{FromRequest, Multipart as AxumMultipart};
    use dioxus_fullstack::http::{Request, header::CONTENT_TYPE};
    use futures::executor::block_on;

    use super::{
        FileUploadHandler, FileUploadWriter, PartOutcome, TemporaryFileUploadHandler, store_part,
    };
    use crate::path::NodePath;

    const BOUNDARY: &str = "fx-test-boundary";

    /// A multipart body of file parts, closed with the final boundary unless
    /// `closed` is false, which leaves the last part unfinished.
    fn body(parts: &[(&str, &[u8])], closed: bool) -> Vec<u8> {
        let mut out = Vec::new();
        for (name, bytes) in parts {
            out.extend_from_slice(
                format!(
                    "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"; \
                     filename=\"{name}.bin\"\r\nContent-Type: application/pdf\r\n\r\n"
                )
                .as_bytes(),
            );
            out.extend_from_slice(bytes);
            out.extend_from_slice(b"\r\n");
        }
        if closed {
            out.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
        }
        out
    }

    /// `body` as a request, through axum's extractor (so with axum's default
    /// 2 MB body limit), in 1,000-byte frames with a `Pending` before each, as
    /// a socket delivers them.
    ///
    /// **The `Pending`s matter.** A stream whose frames are all ready at once
    /// lets multer's buffer swallow the whole body on its first poll, so the
    /// body limit trips inside `next_field`, before `store_part` sees the
    /// part. Over a network the 413 normally comes mid-part, from `chunk`, but
    /// it CAN come from `next_field`, so the driver (h.8) handles both.
    fn multipart(body: &[u8]) -> AxumMultipart {
        let mut frames: std::collections::VecDeque<Result<Vec<u8>, io::Error>> =
            body.chunks(1_000).map(|c| Ok(c.to_vec())).collect();
        let mut wait = true;
        let trickle = futures::stream::poll_fn(move |cx| {
            if wait {
                wait = false;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            } else {
                wait = true;
                std::task::Poll::Ready(frames.pop_front())
            }
        });
        let request = Request::builder()
            .header(
                CONTENT_TYPE,
                format!("multipart/form-data; boundary={BOUNDARY}"),
            )
            .body(Body::from_stream(trickle))
            .expect("a valid request");
        block_on(AxumMultipart::from_request(request, &())).expect("a multipart request")
    }

    /// `store_part` on the request's next part, as the driver would call it.
    fn store_next<H: FileUploadHandler>(
        multipart: &mut AxumMultipart,
        handler: &H,
        max_size: Option<u64>,
    ) -> PartOutcome<H::Stored> {
        block_on(async {
            let mut field = multipart
                .next_field()
                .await
                .expect("a readable request")
                .expect("one more part");
            store_part(
                &mut field,
                handler,
                &NodePath::new("essay"),
                "essay.pdf",
                Some("application/pdf"),
                max_size,
            )
            .await
        })
    }

    /// The name of the request's next part: whether the request can still be
    /// read after `store_part` left a part unfinished.
    fn next_name(multipart: &mut AxumMultipart) -> Option<String> {
        block_on(multipart.next_field())
            .expect("a readable request")
            .map(|field| field.name().unwrap_or_default().to_string())
    }

    /// A handler that keeps each file in memory and logs every call, and can
    /// be told to fail `new_file` or every `write_chunk`.
    #[derive(Default)]
    struct Recording {
        log: Arc<Mutex<Vec<String>>>,
        fail_new_file: bool,
        fail_writes: bool,
    }

    impl Recording {
        fn log(&self) -> Vec<String> {
            self.log.lock().expect("unpoisoned").clone()
        }
    }

    struct RecordingWriter {
        bytes: Vec<u8>,
        log: Arc<Mutex<Vec<String>>>,
        fail_writes: bool,
    }

    impl RecordingWriter {
        fn note(&self, entry: String) {
            self.log.lock().expect("unpoisoned").push(entry);
        }
    }

    impl FileUploadHandler for Recording {
        type Stored = Vec<u8>;
        type Writer = RecordingWriter;

        fn new_file(
            &self,
            path: &NodePath,
            name: &str,
            content_type: Option<&str>,
        ) -> impl Future<Output = io::Result<Self::Writer>> + Send {
            self.log.lock().expect("unpoisoned").push(format!(
                "new_file {path} {name} {}",
                content_type.unwrap_or("-")
            ));
            std::future::ready(if self.fail_new_file {
                Err(io::Error::other("no space"))
            } else {
                Ok(RecordingWriter {
                    bytes: Vec::new(),
                    log: Arc::clone(&self.log),
                    fail_writes: self.fail_writes,
                })
            })
        }

        fn delete(&self, _stored: Self::Stored) -> impl Future<Output = ()> + Send {
            std::future::ready(())
        }
    }

    impl FileUploadWriter for RecordingWriter {
        type Stored = Vec<u8>;

        fn write_chunk(&mut self, chunk: &[u8]) -> impl Future<Output = io::Result<()>> + Send {
            std::future::ready(if self.fail_writes {
                Err(io::Error::other("disk gone"))
            } else {
                self.bytes.extend_from_slice(chunk);
                Ok(())
            })
        }

        fn finish(self) -> impl Future<Output = io::Result<Self::Stored>> + Send {
            self.note(format!("finish {}", self.bytes.len()));
            std::future::ready(Ok(self.bytes))
        }

        fn abort(self) -> impl Future<Output = ()> + Send {
            self.note(format!("abort {}", self.bytes.len()));
            std::future::ready(())
        }
    }

    fn bytes(n: usize) -> Vec<u8> {
        // 251, a prime, so no chunk boundary lines up with the pattern.
        (0..=250_u8).cycle().take(n).collect()
    }

    #[gtest]
    fn a_whole_part_is_stored_with_its_counted_size() {
        let file = bytes(5_500);
        let mut request = multipart(&body(&[("essay", &file)], true));
        let handler = Recording::default();
        let outcome = store_next(&mut request, &handler, None);
        let PartOutcome::Stored { stored, size } = outcome else {
            panic!("expected Stored, got {outcome:?}");
        };
        expect_that!(size, eq(5_500));
        expect_that!(stored == file, eq(true));
        expect_that!(
            handler.log(),
            elements_are![
                eq("new_file essay essay.pdf application/pdf"),
                eq("finish 5500")
            ]
        );
        expect_that!(next_name(&mut request), none());
    }

    #[gtest]
    fn a_part_of_exactly_max_size_is_stored() {
        let mut request = multipart(&body(&[("essay", &bytes(3_000))], true));
        let outcome = store_next(&mut request, &Recording::default(), Some(3_000));
        expect_that!(
            matches!(outcome, PartOutcome::Stored { size: 3_000, .. }),
            eq(true),
            "{outcome:?}"
        );
    }

    /// Aborted at the first chunk past the limit, holding only what fit, and
    /// the request still readable afterwards: multer reads through the rest
    /// of the part on the next `next_field`.
    #[gtest]
    fn a_part_past_max_size_is_aborted_with_a_size_over_the_limit() {
        let mut request = multipart(&body(&[("essay", &bytes(5_500)), ("after", b"x")], true));
        let handler = Recording::default();
        let outcome = store_next(&mut request, &handler, Some(3_000));
        let PartOutcome::TooLarge { size } = outcome else {
            panic!("expected TooLarge, got {outcome:?}");
        };
        expect_that!(size, gt(3_000));
        // Aborted holding only what fit: fewer than 3,000 bytes written.
        let log = handler.log();
        expect_that!(log.len(), eq(2), "{log:?}");
        expect_that!(log[1], starts_with("abort "));
        let written: u64 = log[1]["abort ".len()..].parse().expect("a count");
        expect_that!(written, le(3_000));
        expect_that!(next_name(&mut request), some(eq("after")));
    }

    /// Past axum's default 2 MB body limit (trap 12): the request can't be
    /// read further, so there is nothing to drain.
    #[gtest]
    fn a_body_past_the_limit_is_the_body_limit() {
        let mut request = multipart(&body(&[("essay", &bytes(2_200_000))], true));
        let handler = Recording::default();
        let outcome = store_next(&mut request, &handler, None);
        expect_that!(
            matches!(outcome, PartOutcome::BodyLimit),
            eq(true),
            "{outcome:?}"
        );
        expect_that!(handler.log().last(), some(starts_with("abort ")));
    }

    /// A request that ends mid-part, as when the client goes away.
    #[gtest]
    fn a_request_cut_off_mid_part_is_broken() {
        let mut request = multipart(&body(&[("essay", &bytes(5_500))], false));
        let handler = Recording::default();
        let outcome = store_next(&mut request, &handler, None);
        expect_that!(
            matches!(outcome, PartOutcome::Broken(_)),
            eq(true),
            "{outcome:?}"
        );
        expect_that!(handler.log().last(), some(starts_with("abort ")));
    }

    /// Nothing was opened, so nothing to abort.
    #[gtest]
    fn after_a_failed_new_file_the_request_is_still_readable() {
        let mut request = multipart(&body(&[("essay", &bytes(5_500)), ("after", b"x")], true));
        let handler = Recording {
            fail_new_file: true,
            ..Recording::default()
        };
        let outcome = store_next(&mut request, &handler, None);
        expect_that!(
            matches!(outcome, PartOutcome::Failed(_)),
            eq(true),
            "{outcome:?}"
        );
        expect_that!(
            handler.log(),
            elements_are![eq("new_file essay essay.pdf application/pdf")]
        );
        expect_that!(next_name(&mut request), some(eq("after")));
    }

    #[gtest]
    fn a_failed_write_aborts_and_the_request_is_still_readable() {
        let mut request = multipart(&body(&[("essay", &bytes(5_500)), ("after", b"x")], true));
        let handler = Recording {
            fail_writes: true,
            ..Recording::default()
        };
        let outcome = store_next(&mut request, &handler, None);
        expect_that!(
            matches!(outcome, PartOutcome::Failed(_)),
            eq(true),
            "{outcome:?}"
        );
        expect_that!(handler.log().last(), some(eq("abort 0")));
        expect_that!(next_name(&mut request), some(eq("after")));
    }

    /// The real handler: what it stored is on disk, byte for byte.
    #[gtest]
    fn the_temporary_handler_stores_the_part_on_disk() {
        let scratch = Scratch::new("store-part");
        let file = bytes(5_500);
        let mut request = multipart(&body(&[("essay", &file)], true));
        let handler = TemporaryFileUploadHandler::in_dir(&scratch.0);
        let outcome = store_next(&mut request, &handler, None);
        let PartOutcome::Stored { stored, size } = outcome else {
            panic!("expected Stored, got {outcome:?}");
        };
        expect_that!(size, eq(5_500));
        expect_that!(
            fs::read(stored.path()).expect("the stored file") == file,
            eq(true)
        );
    }
}
