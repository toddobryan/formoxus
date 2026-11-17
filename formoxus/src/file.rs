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
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "accept_multipart (11h) is the caller")
    )]
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
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "accept_multipart (11h) is the caller, through `new`"
    )
)]
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
    type Stored: Clone + Debug + PartialEq + Facet<'static> + Send + Sync + 'static;
    type Writer: FileUploadWriter<Stored = Self::Stored> + Send;

    fn new_file(
        &self,
        path: &NodePath,
        name: &str,
        content_type: Option<&str>,
    ) -> impl Future<Output = io::Result<Self::Writer>> + Send;

    fn delete(&self, stored: Self::Stored) -> impl Future<Output = io::Result<()>> + Send;
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

    fn delete(&self, stored: Self::Stored) -> impl Future<Output = io::Result<()>> + Send {
        // The guard deletes the file once the last clone is dropped, which is
        // when `accept_multipart` drops the rejected form's state.
        drop(stored);
        ready(Ok(()))
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

pub(crate) const WIRE_PART: &str = "__formoxus_wire_form";

pub struct Multipart<T>(pub(crate) MultipartFormData<T>);

impl<T> Debug for Multipart<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            MultipartFormData::OnClient { wire_form, files } => f
                .debug_struct("Multipart")
                .field("wire_form", wire_form)
                .field("files", files)
                .finish(),
        }
    }
}

pub(crate) enum MultipartFormData<T> {
    OnClient {
        wire_form: WireForm<T>,
        files: Vec<(NodePath, FileData)>,
    },
}

impl<T: 'static> IntoRequest for Multipart<T> {
    async fn into_request(self, req: ClientRequest) -> Result<ClientResponse, RequestError> {
        let Multipart(MultipartFormData::OnClient { wire_form, files }) = self;
        let json = serde_json::to_string(&wire_form)
            .map_err(|e| RequestError::Serialization(e.to_string()))?;
        send_parts(req, json, files).await
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

    use super::{IfExists, TemporaryUploadedFile, copy_new, format_size, staging_path, versioned};

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
}
