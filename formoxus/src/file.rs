use std::fmt::Debug;
use std::fs::{self, File, OpenOptions};
use std::future::ready;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::{env, io};

use facet::Facet;

use crate::path::NodePath;

static UNIQUE: AtomicU64 = AtomicU64::new(0);

/// `prefix`, this process's id and a counter: never the same twice in one
/// process, and different processes on one host have different ids.
fn unique_name(prefix: &str) -> String {
    let n = UNIQUE.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{}-{n}", std::process::id())
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
    let mut out = OpenOptions::new().write(true).create_new(true).open(to)?;
    let copied = File::open(from)
        .and_then(|mut input| io::copy(&mut input, &mut out))
        .and_then(|_| out.sync_all());
    if copied.is_err() {
        drop(out);
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use googletest::prelude::*;

    use super::{IfExists, TemporaryUploadedFile, copy_new, staging_path, versioned};

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
