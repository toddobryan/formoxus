//! A form that arrived from a client, rebuilt and validated on the server.

use std::fmt::Debug;

use dioxus_fullstack::extract::multipart::{Field, MultipartError};
use facet::Facet;
use indexmap::IndexMap;

use crate::build::{peel, spec_key_shape};
use crate::error::ValidationMessage;
use crate::file::{
    FileMeta, FileType, FileUploadHandler, Multipart, MultipartFormData, PartOutcome, StoredFile,
    WIRE_PART, clean_file_name, drain, is_body_limit, store_part,
};
use crate::form::{FormErrors, FormSpec, FormState, empty_form, form_for};
use crate::nodes::{is_file_upload, stored_shapes};
use crate::path::{ITEM_SIGIL, ModelPath, NodePath};
use crate::stores::{FilesByPath, ValuesByPath};
use crate::wire::WireForm;

/// A submitted form that passed its own validation, holding both the model the
/// caller wanted and the tree a server verdict can still be attached to.
///
/// **Why both halves.** Rebuilding a form server-side is four mechanical steps
/// — `empty_form`, `apply`, `validate`, and `collect_errors` on the way out —
/// and every `#[post]` that takes a form repeats them identically. Packaging
/// them as a plain `fn(spec, values) -> Result<T, FormErrors>` doesn't work,
/// though: a handler that finds something wrong *after* validation ("that
/// password is incorrect", "that username is taken") needs the `FormState` to
/// push it into, and a function returning only `T` has already dropped it. So
/// this keeps the pair, and hands out `&T` alone.
///
/// **Why `reject_*` consume `self`.** Once a server verdict is recorded the
/// submission is finished, and consuming makes that structural rather than a
/// warning in a comment: `validate` clears every field's errors before it
/// re-runs, so calling it after a push would silently erase the very thing the
/// caller just recorded. Take the value away and there is nothing left to call
/// it on.
///
/// Nothing here is server-only — `FormState` is plain data with no Dioxus
/// runtime behind it, which is what makes the whole approach possible. (A
/// `FormState` still cannot CROSS a server fn boundary, since it holds
/// `Box<dyn Node>`; building one on either side is a different thing.)
// `Debug` for the same reason the rest of this path requires it: a failed
// `expect`/matcher assertion has to be able to print what it got.
#[derive(Debug)]
pub struct Submission<T: Clone + Debug + Facet<'static>> {
    state: FormState<T>,
    model: T,
}

impl<T: Clone + Debug + PartialEq + Facet<'static>> Submission<T> {
    /// Rebuild the form from the raw `(path, value)` pairs a client sent, and
    /// run the form's own validation over it.
    ///
    /// The form starts EMPTY, with no variant chosen and no items, so the pairs
    /// have to carry the structure as well as the leaves: each enum's variant
    /// name and each list's item keys at its own path, as
    /// [`Form::to_wire`](crate::form::Form::to_wire) sends them (the
    /// [`wire`](crate::wire) module docs show the spelling). An unknown variant
    /// name or a malformed item list is a field error at that path. An entry
    /// that is simply MISSING leaves that enum unchosen and that list empty,
    /// which a required enum then rejects, but an empty list does not.
    ///
    /// The spec is the same one the client rendered from, so every per-field
    /// check AND the cross-field validator run again here — a request that
    /// skipped the browser entirely gets identical treatment, with no check
    /// restated by hand at the call site.
    ///
    /// `Err` carries everything the form objected to. Note it can arrive with
    /// an EMPTY `fields`: a cross-field validator's verdict has no path and
    /// lands in [`FormErrors::form`], so "did it pass?" is this `Result`, never
    /// `fields.is_empty()`.
    pub fn accept(spec: FormSpec<T>, values: &ValuesByPath) -> Result<Self, FormErrors> {
        let mut state = empty_form(spec);
        state.distribute_values(values);
        match state.validate() {
            Some(model) => Ok(Self { state, model }),
            None => Err(state.collect_errors()),
        }
    }

    /// The validated model. Borrowed rather than moved out, because a caller
    /// that reads a field and then rejects still needs the submission intact.
    pub fn model(&self) -> &T {
        &self.model
    }

    /// Take the validated model, giving up the ability to reject.
    ///
    /// For a handler that is done checking and wants to hand the model onward
    /// by value. Rejecting afterwards is then a compile error rather than a
    /// judgement call, which is the same trade `reject_field` makes.
    pub fn into_model(self) -> T {
        self.model
    }

    /// Record a server-discovered verdict about one field and finish.
    ///
    /// For what only the server can know — nothing local is wrong with
    /// "incorrect password", so no validator could have caught it. The message
    /// lands at the path the widget that must display it already renders at.
    ///
    /// **Panics if `path` isn't a field of this form.** That is a caller bug (a
    /// typo, or a path assuming a shape the model doesn't have), not anything a
    /// user's input can provoke — and unlike the client, where a panic aborts
    /// instead of reaching an `ErrorBoundary`, a server panic unwinds into a
    /// 500. Failing loudly beats returning a `Result` that every call site has
    /// to `?` for a bug that should never ship, and beats dropping the message
    /// on the floor.
    /// **Takes a [`ModelPath`], not a `&str`.** A typo is a compile error at the
    /// call site, so the panic below is now reachable only by a path
    /// `path!` cannot spell — a specific item, whose index only the runtime
    /// knows. Those go through [`FormState::push_field_error`] with a string.
    ///
    /// **Still panics on a path this form does not have.** A `ModelPath<T>` proves
    /// the field exists on `T`; it does not prove this form renders it, since a
    /// form may cover part of a model, and an unchosen variant's fields are not
    /// in the tree. That remains a caller bug rather than anything a user's
    /// input can provoke, and on a server a panic unwinds into a 500 — which
    /// beats dropping the message on the floor.
    ///
    /// **After [`accept_multipart`](Self::accept_multipart),** rejecting drops
    /// the model's `Received` files without deleting them. Pass each one's
    /// `stored` to `handler.delete` first, unless the handler's files delete
    /// themselves when dropped, as
    /// [`TemporaryFileUploadHandler`](crate::file::TemporaryFileUploadHandler)'s
    /// do.
    ///
    /// [`FormState::push_field_error`]: crate::form::FormState::push_field_error
    pub fn reject_field(mut self, path: &ModelPath<T>, message: &str) -> WireForm<T> {
        self.state
            .push_field_error(path, message)
            .unwrap_or_else(|e| panic!("cannot reject `{path}`: {e}"));
        self.into_wire_with_errors()
    }

    /// Record a server-discovered verdict about the form as a whole and finish.
    ///
    /// The counterpart to [`reject_field`](Self::reject_field) for a verdict
    /// that names no field — "those credentials don't match" can't say whether
    /// it was the username or the password.
    ///
    /// After [`accept_multipart`](Self::accept_multipart), delete the received
    /// files first, as [`reject_field`](Self::reject_field) says.
    pub fn reject(mut self, message: &str) -> WireForm<T> {
        self.state.errors.push(message.into());
        self.into_wire_with_errors()
    }

    /// Accepted as submitted: the form's own values, leaves and structure, no
    /// errors.
    ///
    /// For a handler that is finished and wants the client to absorb the
    /// canonical values. To send back values the server *changed*, normalize
    /// the model instead and use [`WireForm::from_model`] — that regenerates
    /// every value from the model, so the two cannot drift.
    pub fn into_wire(self) -> WireForm<T> {
        WireForm::new(self.state.collect_values(), FormErrors::default())
    }

    /// Values plus whatever has been objected to — what both `reject_*` return.
    fn into_wire_with_errors(self) -> WireForm<T> {
        let errors = self.state.collect_errors();
        WireForm::new(self.state.collect_values(), errors)
    }

    /// [`accept`](Self::accept) for a form with files: read a [`Multipart`]
    /// request, store each file through `handler` as it arrives, then rebuild
    /// the form and run its validation.
    ///
    /// - **`spec`** is the one the client rendered from, as for `accept`. Each
    ///   file field's `max_size` and `accept` are checked again against what
    ///   arrives, with the messages the browser showed. A file whose stated
    ///   size is over the limit, or whose type the field won't take, is never
    ///   stored; one that turns out larger than it said is stopped where it
    ///   passes the limit.
    /// - **`handler`** stores each file as it streams in, never whole in
    ///   memory. Its `Stored` must be the `S` of every
    ///   [`FileUpload<S>`](crate::file::FileUpload) in `T`. The signature can't
    ///   say so, so it is checked first, and a mismatch panics: that is a bug
    ///   in the server function, not anything a user can cause.
    /// - **`existing`** is the record being edited, or `None` on a create. A
    ///   file the user kept arrives as a name with no file, and is believed
    ///   only if `existing` has a file at the same path. It then takes the
    ///   record's name, whatever the client sent; otherwise it counts as
    ///   missing (a required error, or `None` for an `Option<FileUpload>`). A
    ///   kept file inside a list or set item always counts as missing, because
    ///   an item's key is the form's own count and may name a different file on
    ///   the record.
    ///
    /// On `Ok`, each new file is `FileUpload::Received`, holding what the
    /// handler stored, and each kept one is `FileUpload::Existing`. On `Err`,
    /// every file stored for this request has already been passed to
    /// `handler.delete`. The `Err` holds field errors, or one form-level error
    /// when the request was too large, broke off, didn't come from
    /// [`Form::to_multipart`](crate::form::Form::to_multipart), or a file
    /// couldn't be stored.
    ///
    /// **The request is read once, in order, and to its end.** The values come
    /// first, then each file, stored before the form exists. This returns only
    /// after the whole request has been read, even when it has already failed,
    /// because a server that answers before the browser has finished sending
    /// loses its reply. The exceptions are a request past the body limit and
    /// one that broke off, neither of which can be read any further.
    ///
    /// **The body limit.** axum refuses a request body past 2 MB by default, so
    /// a server function whose files can be larger needs a limit of its own:
    /// `#[middleware(DefaultBodyLimit::max(…))]` on it. Past the limit this
    /// returns the form-level error "These files are too large to send
    /// together." The limit can't be read from the request, so the message
    /// can't name it. A `max_size` above the body limit can never be reached.
    ///
    /// **Rejecting after success.** A check that fails after this returned
    /// `Ok` ([`reject_field`](Self::reject_field), [`reject`](Self::reject))
    /// drops the model's `Received` files without deleting them.
    /// [`TemporaryFileUploadHandler`](crate::file::TemporaryFileUploadHandler)'s
    /// files delete themselves when dropped; a handler of your own (an S3 key,
    /// say) may leave them stored. Pass each `Received` file's `stored` to
    /// `handler.delete` before rejecting.
    ///
    /// ```ignore
    /// // server
    /// #[post("/api/essay")]
    /// #[middleware(DefaultBodyLimit::max(25 * 1024 * 1024))]
    /// async fn submit(form: Multipart<Essay>) -> Result<WireForm<Essay>, ServerFnError> {
    ///     let handler = TemporaryFileUploadHandler::new();
    ///     let sub = match Submission::accept_multipart(spec(), form, &handler, None).await {
    ///         Ok(sub) => sub,
    ///         Err(errors) => return Ok(WireForm::from_errors(errors)),
    ///     };
    ///     let model = sub.into_model();
    ///     if let FileUpload::Received { name, stored, .. } = &model.essay {
    ///         stored.move_to(essay_dir().join(name), IfExists::Version)?;
    ///     }
    ///     insert(&model).await?;
    ///     Ok(WireForm::from_model(&model, spec()))
    /// }
    ///
    /// // client
    /// form.absorb(submit(form.to_multipart()).await?);
    /// ```
    pub async fn accept_multipart<H: FileUploadHandler>(
        spec: FormSpec<T>,
        multipart: Multipart<T>,
        handler: &H,
        existing: Option<&T>,
    ) -> Result<Self, FormErrors> {
        for s in stored_shapes(T::SHAPE) {
            assert!(
                s == <H::Stored as Facet>::SHAPE,
                "Expected file handler of type {}, but found {}",
                s,
                <H::Stored as Facet>::SHAPE,
            );
        }
        let Multipart(MultipartFormData::OnServer(mut axum_mp)) = multipart else {
            panic!("Expected a Multipart(OnServer(_), but found {multipart:?}");
        };

        // A read error (from `next_field` or `text`) means the request can't be
        // read further, so it returns at once; anything else drains first.
        let text = match axum_mp.next_field().await {
            Ok(Some(field)) if field.name() == Some(WIRE_PART) => match field.text().await {
                Ok(text) => Some(text),
                Err(e) => return Err(unreadable(&e)),
            },
            Ok(_) => None,
            Err(e) => return Err(unreadable(&e)),
        };
        let Some(wire_form) = text.and_then(|t| serde_json::from_str::<WireForm<T>>(&t).ok())
        else {
            drain(&mut axum_mp).await;
            return Err(form_error(NOT_FROM_FORMOXUS));
        };

        let mut received = Received {
            values: wire_form.values().clone(),
            files: FilesByPath::new(),
            stored: Vec::new(),
        };
        // Every failure breaks out to the one cleanup below, so a file stored
        // before it is always deleted.
        let abort = loop {
            match axum_mp.next_field().await {
                Ok(Some(field)) => {
                    let sizes = wire_form.file_sizes();
                    if let Err(abort) =
                        Self::handle_file(field, &spec, handler, sizes, &mut received).await
                    {
                        break Some(abort);
                    }
                }
                Ok(None) => break None,
                Err(e) => break Some(Abort::Unreadable(unreadable(&e))),
            }
        };
        if let Some(abort) = abort {
            let errors = match abort {
                Abort::Unreadable(errors) => errors,
                Abort::Drain(errors) => {
                    drain(&mut axum_mp).await;
                    errors
                }
            };
            // Each handler owns what a failed delete means (PP).
            for stored in received.stored {
                handler.delete(stored).await;
            }
            return Err(errors);
        }

        Self::check_existing_claims(&spec, existing, &mut received);

        // h.10: as `accept` does, with the files distributed after the values,
        // since the values build the structure (variants, items) every file
        // leaf's path needs.
        let mut state = empty_form(spec);
        state.distribute_values(&received.values);
        state.distribute_files(&received.files);
        let Some(model) = state.validate() else {
            // Each handler owns what a failed delete means (PP).
            for stored in received.stored {
                handler.delete(stored).await;
            }
            return Err(state.collect_errors());
        };
        Ok(Self { state, model })
    }

    /// h.8 for one part: check it names a file leaf the wire part named,
    /// refuse a stated size or a type the field won't take, otherwise store
    /// it. Every file part it accepts ends as a `FileMeta` in
    /// `received.files`, stored or not; `FileNode::validate` reports a refused
    /// one later, with the message the browser showed.
    ///
    /// A part that isn't a file leaf, or whose name the wire part left empty,
    /// is ignored: only a hand-made request sends one, and the next
    /// `next_field` drains it.
    async fn handle_file<H: FileUploadHandler>(
        mut field: Field<'_>,
        spec: &FormSpec<T>,
        handler: &H,
        stated_sizes: &IndexMap<NodePath, u64>,
        received: &mut Received<H::Stored>,
    ) -> Result<(), Abort> {
        let Some(path) = field.name().map(NodePath::new) else {
            return Ok(());
        };
        if !is_file_leaf::<T>(&path) {
            return Ok(());
        }
        let key = path.spec_key();
        // The file's name is the wire's, never the part's `filename=`.
        let Some(raw_name) = received.values.get(&path).filter(|name| !name.is_empty()) else {
            return Ok(());
        };
        let Some(name) = clean_file_name(raw_name) else {
            return Err(Abort::Drain(form_error(UNUSABLE_FILE_NAME)));
        };
        received.values.insert(path.clone(), name.clone());

        let content_type = field.content_type().map(str::to_owned);
        let field_spec = spec.fields.get(&key);
        let max_size = field_spec.and_then(|s| s.max_size);
        let accept = field_spec.map_or(&[][..], |s| &s.accept[..]);
        let stated = stated_sizes.get(&path).copied();

        // Refused before `new_file`, so nothing is stored.
        if stated
            .zip(max_size)
            .is_some_and(|(stated, max)| stated > max)
            || !FileType::accepts(accept, &name, content_type.as_deref())
        {
            let meta = FileMeta {
                content_type,
                size: stated.unwrap_or(0),
                stored: None,
            };
            received.files.insert(path, meta);
            return Ok(());
        }

        let outcome = store_part(
            &mut field,
            handler,
            &path,
            &name,
            content_type.as_deref(),
            max_size,
        )
        .await;
        let meta = match outcome {
            PartOutcome::Stored { stored, size } => {
                received.stored.push(stored.clone());
                FileMeta {
                    content_type,
                    size,
                    stored: Some(StoredFile::new(stored)),
                }
            }
            PartOutcome::TooLarge { size } => FileMeta {
                content_type,
                size,
                stored: None,
            },
            PartOutcome::BodyLimit => {
                return Err(Abort::Unreadable(form_error(TOO_LARGE_TOGETHER)));
            }
            PartOutcome::Broken(_) => return Err(Abort::Unreadable(form_error(BROKEN_OFF))),
            PartOutcome::Failed(_) => return Err(Abort::Drain(form_error(NOT_SAVED))),
        };
        received.files.insert(path, meta);
        Ok(())
    }

    /// h.9, decision V with NN: a file leaf the wire part names, with no part
    /// in `received.files`, claims the file already on the record. Check each
    /// claim against `existing`, rewriting its value in `received.values`.
    ///
    /// A claim stands only where the record has a non-empty name at the same
    /// path, and the value becomes the RECORD's name, whatever the client sent
    /// (NN). Otherwise it becomes `""`, so it counts as missing: a required
    /// error, or `None` for an `Option<FileUpload>`. That covers a create
    /// (`existing` is `None`), a path the record lacks, and any path inside a
    /// list or set item: an item key (`#3`) is the form's own count, not an
    /// identity the record shares, so the same key may name another file there
    /// (decision V's trap).
    fn check_existing_claims<S>(
        spec: &FormSpec<T>,
        existing: Option<&T>,
        received: &mut Received<S>,
    ) {
        let claims: Vec<NodePath> = received
            .values
            .iter()
            .filter(|(path, name)| {
                !name.is_empty() && !received.files.contains_key(*path) && is_file_leaf::<T>(path)
            })
            .map(|(path, _)| path.clone())
            .collect();
        if claims.is_empty() {
            return;
        }
        // Built once, and only when something claims a file.
        let on_record = existing.map(|record| form_for(record, spec.clone()).collect_values());
        for path in claims {
            let in_item = path.as_str().split('.').any(|s| s.starts_with(ITEM_SIGIL));
            let name = on_record
                .as_ref()
                .filter(|_| !in_item)
                .and_then(|values| values.get(&path))
                .filter(|name| !name.is_empty())
                .cloned()
                .unwrap_or_default();
            received.values.insert(path, name);
        }
    }
}

/// Whether `path` names a `FileUpload` leaf of `T`, optional or not. Peeled:
/// `spec_key_shape` returns an `Option<FileUpload>` field's shape as the
/// `Option`.
fn is_file_leaf<T: Facet<'static>>(path: &NodePath) -> bool {
    spec_key_shape(T::SHAPE, &path.spec_key()).is_ok_and(|shape| is_file_upload(peel(shape)))
}

/// What h.8 gathers from the file parts, for h.9 and h.10.
struct Received<S> {
    /// A copy of the wire's values with each received file's name cleaned,
    /// which h.9 edits further and h.10 distributes.
    values: ValuesByPath,
    /// Every file part h.8 accepted, stored or refused, for `distribute_files`.
    files: FilesByPath,
    /// What the handler stored, to `delete` if the submission fails.
    stored: Vec<S>,
}

/// Why the driver stopped reading parts. Either way, everything stored so far
/// is deleted.
enum Abort {
    /// The body limit or a broken request: it can't be read further, so no
    /// drain.
    Unreadable(FormErrors),
    /// It can still be read (the handler failed, or a file name was refused),
    /// so drain it before answering.
    Drain(FormErrors),
}

/// The first part wasn't a [`WireForm`] named `WIRE_PART`: the request didn't
/// come from `to_multipart`, so only a hand-made one gets this.
const NOT_FROM_FORMOXUS: &str = "Expected a Multipart value with wire_form as first field";
/// Decision OO. The limit itself can't be named: axum-core keeps it private.
const TOO_LARGE_TOGETHER: &str = "These files are too large to send together.";
/// The request broke off or was malformed partway (`PartOutcome::Broken`).
const BROKEN_OFF: &str = "The upload was interrupted. Please try again.";
/// Decision QQ: the handler failed (the disk is full, S3 is down).
const NOT_SAVED: &str = "This upload couldn't be saved. Please try again.";
/// `clean_file_name` refused the wire's name: nothing usable was left.
const UNUSABLE_FILE_NAME: &str =
    "That file name can't be used. Please rename the file and try again.";

fn form_error(message: &str) -> FormErrors {
    FormErrors {
        form: vec![ValidationMessage(message.to_string())],
        fields: IndexMap::new(),
    }
}

/// A read error that ends the request: the body limit (OO) or broken off.
/// Neither drains, since neither request can be read any further.
fn unreadable(e: &MultipartError) -> FormErrors {
    form_error(if is_body_limit(e) {
        TOO_LARGE_TOGETHER
    } else {
        BROKEN_OFF
    })
}
