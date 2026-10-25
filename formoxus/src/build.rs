//! The SHAPE walk: turn a `&'static Shape` (plus an optional value to populate
//! from) into a tree of `Node`s. One `*_node` helper per kind of thing
//! a shape can be.

use facet::{
    EnumType, Field as FacetField, OptionDef, Peek, PeekEnum, ScalarType, Shape, StructKind,
    StructType, Type, UserType, Variant,
};

use crate::field_node::LeafArgs;
use crate::fields::{BoolField, NumberField, StringField};
use crate::form::FieldSpec;
use crate::nodes::{
    FieldSet, ListSet, Node, OptionNode, ROW_SIGIL, SpecsByPath, VariantChoice, VariantSet,
    qualify, row_segment, variant_segment,
};

/// Which mode the whole walk is in — fixed at the root by which constructor the
/// caller reached for, then threaded down unchanged.
///
/// **Never recompute this from a local peek.** It was a `seeding: bool` derived
/// from `peek.is_some()`, which was tautological the moment `option_node`
/// started peeling one `Option` layer per recursion: the peek it asked about was
/// the very one it was meant to disambiguate.
///
/// Note that [`Populated`](Self::Populated) with no peek at some node is not a
/// contradiction — it is the whole point. It means "there is a value, and at
/// this node that value is absent," which is an answer. That is also why the
/// tempting tightening (`Populated(Peek<'_, 'static>)`) is wrong: it would
/// outlaw a legal state and re-conflate "nothing here" with "nothing anywhere."
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FormMode {
    /// `empty_form*`: no value anywhere. A missing peek means "nothing to populate
    /// from," and variant choices come from the caller's map.
    Blank,
    /// `form_for(&value)`: a missing peek means the value's own `Option` really
    /// was `None` — absent, and no choice needed. This is what makes edit mode
    /// infallible even for optional enums.
    Populated,
}

/// What the whole walk carries, fixed at the root.
///
/// One value rather than another parameter on every function below: the mode
/// and the spec are both decided once, by the caller, and threaded unchanged.
///
/// **The walk applies the spec as it builds.** Each node looks itself up by
/// its path when it is constructed, so it is right the first time: there is
/// no second pass that visits the tree and mutates it, and a structural edit
/// (`AddRow`, `ChooseVariant`) builds its new nodes through this same walk, so
/// they come out the same way as the rest.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Walk<'s> {
    pub(crate) mode: FormMode,
    pub(crate) specs: &'s SpecsByPath,
}

impl<'s> Walk<'s> {
    /// The spec that names the node at `path`, if any.
    fn spec_for(&self, path: &str) -> Option<&'s FieldSpec> {
        self.specs.get(&spec_key(path))
    }
}

/// The spec key that names the node at `path`.
///
/// Rows are keyed by identity (`venues.#3`), but a spec names every row at
/// once (`venues[]`), so each `.#n` segment folds into a `[]` on the segment
/// before it: `venues.#3.city` is `venues[].city`, and `answers.#0` is
/// `answers[]`. Variant segments (`$Circle`) are spelled the same in both.
pub(crate) fn spec_key(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for segment in path.split('.') {
        if segment.starts_with(ROW_SIGIL) {
            out.push_str("[]");
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(segment);
        }
    }
    out
}

/// One node per declared field of `shape`. `peek` is the value being populated
/// from, if any — it must describe the same shape.
pub(crate) fn nodes_for(
    shape: &'static Shape,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Vec<Box<dyn Node>> {
    match &shape.ty {
        Type::User(UserType::Struct(struct_type)) => {
            fields_from_struct(struct_type, peek, walk, prefix, optional)
        }
        Type::User(UserType::Enum(enum_type)) => {
            fields_from_enum(enum_type, peek, walk, prefix, optional)
        }
        _ => panic!("form_for only handles structs and enums, got {shape}"),
    }
}

fn fields_from_struct(
    struct_type: &StructType,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Vec<Box<dyn Node>> {
    let peek_struct = peek.map(|p| {
        p.into_struct()
            .expect("shape said struct, so the value peeks as one")
    });

    struct_type
        .fields
        .iter()
        .map(|field| {
            let field_peek = peek_struct.map(|ps| {
                ps.field_by_name(field.name)
                    .expect("field came from this shape, so it exists on the value")
            });
            node_for(
                field,
                field_peek,
                walk,
                &qualify(prefix, field.name),
                optional,
            )
        })
        .collect()
}

/// The variant to build for an enum shape. Edit mode reads it off the value
/// (the value pins it); create mode takes it from the caller's chosen-variant
/// map, keyed by this enum's qualified path. Shared by the top-level
/// `fields_from_enum` and `node_for`'s enum-field arm (which also needs the
/// variant's *name*), so the selection logic lives in exactly one place.
/// `None` means [`VariantChoice::Unchosen`] — an `Option<Enum>` with no value.
///
/// [`FormMode`] is what disambiguates a `peek_enum` of `None`: under
/// `Populated` it means the value's `Option` really was `None` (absent, no
/// choice needed); under `Blank` it just means there is no value at all and the
/// answer comes from the caller's map.
fn chosen_variant(
    enum_type: &EnumType,
    peek_enum: Option<PeekEnum<'_, 'static>>,
    walk: Walk<'_>,
    _prefix: &str,
) -> Option<&'static Variant> {
    if walk.mode == FormMode::Populated {
        // `?` is the `None` case: populating from a value whose optional enum
        // is itself `None`, so there is no active variant to read.
        let variant_name = peek_enum?
            .variant_name_active()
            .expect("an enum value always has an active variant")
            .to_string();
        Some(
            enum_type
                .variants
            .iter()
            .find(|v| v.name == variant_name)
            .unwrap_or_else(|| {
                panic!("{variant_name:?} is not a variant of this enum — missing_variants should have caught this")
            }
            )
        )
    } else {
        None
    }
}

/// One node per field of the chosen `variant`, populated from that variant's
/// fields on the value in edit mode. Paths accumulate under this enum's path
/// just like a struct's fields — the variant name is NOT part of the path
/// (it's locked; `write_into` replays it via `select_variant_named`).
pub(crate) fn variant_nodes(
    variant: &'static Variant,
    peek_enum: Option<PeekEnum<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Vec<Box<dyn Node>> {
    variant
        .data
        .fields
        .iter()
        .map(|field| {
            let field_peek = peek_enum.and_then(|pe| {
                pe.field_by_name(field.name)
                    .expect("field belongs to the active variant, so access can't error")
            });
            node_for(
                field,
                field_peek,
                walk,
                &qualify(prefix, field.name),
                optional,
            )
        })
        .collect()
}

/// Nodes for a top-level enum model (the whole `FormState<T>` is an enum). Enum
/// *fields* don't come through here — `node_for` builds a `VariantSet` for
/// those; this is only the `nodes_for` dispatch for a bare-enum `T`.
fn fields_from_enum(
    enum_type: &EnumType,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Vec<Box<dyn Node>> {
    let peek_enum = peek.map(|p| {
        p.into_enum()
            .expect("shape said enum, so the value peeks as one")
    });
    let variant = chosen_variant(enum_type, peek_enum, walk, prefix)
        .expect("Variant should not be Unchosen here.");
    variant_nodes(variant, peek_enum, walk, prefix, optional)
}

/// The node for one declared *field* — the common case, where the name and
/// shape both come from the field itself.
fn node_for(
    field: &'static FacetField,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Box<dyn Node> {
    node_for_shape(field.shape(), field.name, peek, walk, prefix, optional)
}

/// The node for a shape that is *named separately* from where it came from.
///
/// Split out of [`node_for`] because a list element has no [`Field`] and so no
/// `field.name` — its name is its index (`"0"`, `"1"`, …), supplied by the
/// enclosing `ListSet`. Everything below is shape-driven and never looked at the
/// `Field` for anything but those two things, so the split is pure motion.
pub(crate) fn node_for_shape(
    shape: &'static Shape,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Box<dyn Node> {
    if let Ok(option_def) = shape.def.into_option() {
        return option_node(option_def, name, peek, walk, prefix);
    } else if let Some(scalar) = shape.scalar_type() {
        return scalar_node(
            scalar, name, peek, walk, prefix, optional, /* wrapper */ None,
        )
        .unwrap_or_else(|| {
            panic!("scalar type {scalar:?} is not supported in Field (field {name})")
        });
    } else if let Some(inner) = newtype_inner(shape) {
        // A newtype is ONE input, not a fieldset around a field called "0".
        // The peek descends with it: the field holds the inner scalar, so
        // `populate` must see the inner value, not the wrapper.
        let scalar = inner
            .scalar_type()
            .expect("newtype_inner only returns scalars");
        let inner_peek = peek
            .and_then(|p| p.into_struct().ok())
            .and_then(|s| s.field(0).ok());
        return scalar_node(
            scalar,
            name,
            inner_peek,
            walk,
            prefix,
            optional,
            Some(shape),
        )
        .unwrap_or_else(|| {
            panic!(
                "newtype {shape} wraps {scalar:?}, which is not supported in Field (field {name})"
            )
        });
    } else if let Ok(list_def) = shape.def.into_list() {
        // SEAM: `list_node(_list_def.t, name, inner_peek, variants, prefix)`,
        // building one row per element via `node_for_shape` with the index as
        // the row's name. See VEC_PLAN.md.
        return list_node(list_def.t, name, peek, walk, prefix, optional);
    }

    match &shape.ty {
        Type::User(UserType::Struct(_)) => struct_node(shape, name, peek, walk, prefix, optional),
        Type::User(UserType::Enum(enum_type)) => {
            enum_node(enum_type, name, peek, walk, prefix, optional)
        }
        other => panic!("field {name} has unsupported type {other:?}"),
    }
}

fn option_node(
    option_def: OptionDef,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
) -> Box<dyn Node> {
    let inner_peek = peek.and_then(|p| {
        p.into_option()
            .expect("shape said Option, so the value should peek as one")
            .value()
    });
    let inner = node_for_shape(
        option_def.t,
        name,
        inner_peek,
        walk,
        prefix,
        /* optional */ true,
    );
    Box::new(OptionNode { inner })
}

/// The inner shape of a newtype wrapper — `String` for `struct Markdown(String)`.
///
/// `None` for anything else, which leaves the walk exactly as it was.
///
/// **A ONE-FIELD TUPLE STRUCT, not any one-field struct.** `Markdown(String)` is
/// Rust's newtype idiom and means "this IS a string, with a type attached";
/// `struct Config { name: String }` is an ordinary struct that happens to have
/// one field today. Flattening the latter would rename its leaf from
/// `config.name` to `config` — and leaf paths ARE the wire format now, the
/// thing a server-side rebuild keys on. The day someone adds a second field the
/// paths would silently change back, and every stored or in-flight payload
/// would mean something different. A tuple struct cannot grow that way without
/// the author rewriting it as a named struct, which is a visible act.
///
/// The inner must itself be a scalar: `struct Wrapper(SomeStruct)` stays a
/// struct, because there is no single input that could carry it.
///
/// Note this needs NO `#[facet(transparent)]`. That attribute would also work
/// (it populates `shape.inner`), but requiring it would mean a newtype in a
/// crate we don't widget could never be a form field.
pub(crate) fn newtype_inner(shape: &'static Shape) -> Option<&'static Shape> {
    let Type::User(UserType::Struct(st)) = &shape.ty else {
        return None;
    };
    if st.kind != StructKind::TupleStruct || st.fields.len() != 1 {
        return None;
    }
    let inner = st.fields[0].shape();
    inner.scalar_type().map(|_| inner)
}

/// The closed set of scalar types with a built-in widget. Anything else needs
/// a custom widget and returns `None` here, which `node_for_shape` turns into
/// a panic naming the type.
///
/// **`usize` and `isize` are left out on purpose, not by oversight.** Their
/// width is target-dependent — 32 bits on the wasm client, 64 on the server —
/// so the same field would carry different bounds on each side of the wire, and
/// a value the server accepts could be unparseable in the browser. Use a fixed
/// width (`u32`, `u64`) in a form model and convert at the edges. Adding the
/// arms back would compile and look fine; the disagreement only shows up at
/// runtime, on one target.
///
/// **This walk no longer decides anything about presentation.** It used to
/// assign an `InputKind` per arm; the widget is now resolved once, in
/// `LeafArgs::node`: the spec's `with_widget`, else the field's own
/// `default_widget`. So the
/// only thing the shape contributes here is *which concrete `T`* the field
/// carries, and the only thing the walk contributes is `optional` — a
/// structural fact (`Option` peeling) that no later reader can recover,
/// because `bool` and `Option<bool>` both arrive as `Field<bool>`.
fn scalar_node(
    scalar: ScalarType,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    // This leaf's own full path: `node_for_shape`'s `prefix` already has the
    // name qualified on.
    path: &str,
    optional: bool,
    // The newtype this scalar is wrapped in, if any. The field carries the
    // INNER type (`Field<String>` for a `Markdown`), because a concrete `T`
    // cannot be recovered from a runtime `Shape` — so the wrapper is remembered
    // here and re-applied when the value is written back.
    wrapper: Option<&'static Shape>,
) -> Option<Box<dyn Node>> {
    let args = LeafArgs {
        name,
        path,
        peek,
        optional,
        wrapper,
        spec: walk.spec_for(path),
    };
    // A spec that names this leaf with a constructor builds it, in place of the
    // default for its type. That is how `min: 3` reaches a typed field, and how
    // `field: X` changes the node's type, with nothing mutated afterwards.
    if let Some(ctor) = args.spec.and_then(|s| s.node.as_ref()) {
        return Some(ctor.build(args));
    }

    // The variant and the type can't be collapsed into one token: `ScalarType`
    // spells them `I8`/`U32` and Rust spells them `i8`/`u32`, and `macro_rules!`
    // has no way to change an identifier's case. So each arm names both, and the
    // pairing is the one thing to get right when adding a type.
    //
    // The numeric arms are exactly `Number`'s impls (`fields/number_fields.rs`);
    // keep the two lists in step.
    macro_rules! dispatch {
        ($( $variant:ident => $ty:ty ),* $(,)?) => {
            match scalar {
                ScalarType::String => Some(Box::new(args.node(StringField::default())) as Box<dyn Node>),
                ScalarType::Bool => Some(Box::new(args.node(BoolField::default())) as Box<dyn Node>),
                $(
                    ScalarType::$variant => Some(
                        Box::new(args.node(NumberField::<$ty>::default())) as Box<dyn Node>
                    ),
                )*
                _ => None
            }
        };
    }

    dispatch! {
        I8 => i8,
        I16 => i16,
        I32 => i32,
        I64 => i64,
        U8 => u8,
        U16 => u16,
        U32 => u32,
        U64 => u64,
        F32 => f32,
        F64 => f64,
    }
}

/// A list-typed field: a [`ListSet`] whose rows are named by a `#`-prefixed key,
/// so the leaf paths (`answer_choices.#0.text`) fall out of the same `qualify`
/// nesting a struct's fields use — no special casing anywhere downstream.
///
/// `shape` is the *element* shape (`ListDef::t`), not the `Vec`'s, and the
/// `ListSet` keeps it: `Edit::AddRow` has to build a node for a row that has
/// no value to peek at, and this is the only thing that says what kind.
///
/// Edit mode takes the row count from the value. Create mode has no value to
/// count and so starts empty — which is now an answer rather than a gap, since
/// `AddRow` is how a blank list gets its rows.
fn list_node(
    shape: &'static Shape,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Box<dyn Node> {
    let rows: Vec<Box<dyn Node>> = peek
        .map(|p| {
            let list = p
                .into_list()
                .expect("shape said list, so the value peeks as one");
            list.iter()
                .enumerate()
                .map(|(i, element)| {
                    // A row's KEY is its name — not its index. At construction
                    // the two coincide, but they part company the moment a row
                    // is inserted or removed; see `row_segment`. And unlike
                    // `struct_node`, which passes `prefix` straight through,
                    // nothing upstream has qualified this on yet, so do it here.
                    // Keeping this in step with `collect_values` is what keeps
                    // the leaf paths and the store keys the same strings.
                    let row = row_segment(i);
                    node_for_shape(
                        shape,
                        &row,
                        Some(element),
                        walk,
                        &qualify(prefix, &row),
                        /* optional */ false,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let spec = walk.spec_for(prefix);
    if let Some(spec) = spec {
        assert!(
            spec.widget.is_none(),
            "{prefix} is a list, which has no single widget to override — \
             write `{name}[]` to give every ROW a widget, or did you mean `label`?",
        );
    }
    Box::new(ListSet {
        name: name.to_string(),
        label: spec.and_then(|s| s.label.clone()),
        shape,
        optional,
        // The next key to hand out. Starts past the rows built here so a key is
        // never reused, which is the whole point of keying by identity.
        next_key: rows.len(),
        rows,
        errors: Vec::new(),
        rejected_keys: None,
    })
}

/// A struct-typed field: a [`FieldSet`] over that struct's own fields, whose
/// paths accumulate under this field's name.
fn struct_node(
    shape: &'static Shape,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Box<dyn Node> {
    let spec = walk.spec_for(prefix);
    if let Some(spec) = spec {
        assert!(
            spec.widget.is_none(),
            "{prefix} is a field set, which has no single widget to override \
             (a composite widget for one isn't supported yet) — did you mean `label`?"
        );
    }
    Box::new(FieldSet {
        name: name.to_string(),
        optional,
        label: spec.and_then(|s| s.label.clone()),
        nodes: nodes_for(shape, peek, walk, prefix, /* optional */ false),
        errors: Vec::new(),
    })
}

/// An enum-typed field: a [`VariantSet`] locked to one already-decided variant.
///
/// [`FormMode`] comes from the root, not from `peek` — see its docs for why
/// deriving it here was the bug that made absent optional enums panic.
fn enum_node(
    enum_type: &'static EnumType,
    name: &str,
    peek: Option<Peek<'_, 'static>>,
    walk: Walk<'_>,
    prefix: &str,
    optional: bool,
) -> Box<dyn Node> {
    let peek_enum = peek.map(|p| {
        p.into_enum()
            .expect("shape said enum, so the value peeks as one")
    });
    let variant = chosen_variant(enum_type, peek_enum, walk, prefix);
    let spec = walk.spec_for(prefix);
    Box::new(VariantSet {
        field_attrs: spec.map(|s| s.attrs.clone()).unwrap_or_default(),
        name: name.to_string(),
        label: spec.and_then(|s| s.label.clone()),
        enum_type,
        optional,
        // `None` from `chosen_variant` is exactly `Unchosen` — the caller chose it
        choice: match variant {
            Some(v) => VariantChoice::Named(v.name.to_string()),
            None => VariantChoice::Unchosen,
        },
        // No variant means no fields to build.
        nodes: variant
            // Under the variant's own segment, the path these fields really live
            // at (`shape.$Circle.radius`), as `set_variant` and `child_prefix`
            // have it. The spec is looked up by that path.
            .map(|v| {
                variant_nodes(
                    v,
                    peek_enum,
                    walk,
                    &qualify(prefix, &variant_segment(v.name)),
                    /* optional */ false,
                )
            })
            .unwrap_or_default(),
        errors: Vec::new(),
    })
}
