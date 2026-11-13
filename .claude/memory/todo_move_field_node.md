---
name: todo-move-field-node
description: "REMINDER (Todd, 2026-11-13): move src/field_node.rs into src/nodes/ AFTER the WIDGETS_PLAN work is done; raise it when the plan's last box is ticked"
metadata:
  type: project
---

Todd asked on 2026-11-13 to be reminded to move `formoxus/src/field_node.rs` into
`formoxus/src/nodes/`, next to every other `Node` impl (`struct_node.rs`,
`list_node.rs`, `set_node.rs`, `option_node.rs`, `variant_node.rs`, and
`file_node.rs`, which was born there during WIDGETS_PLAN box 11e).

**Why:** `FieldNode` is the one `Node` impl outside `nodes/`, a leftover of its
history, not a design choice.

**How to apply:** do NOT do it mid-plan, since `file` and later branches would
conflict. Bring it up once WIDGETS_PLAN is finished (all boxes, `file` merged), as
its own small commit. It's a move plus path fixes (`crate::field_node::` is used
about 6 times in `src/`); `formoxus-macros` did not name `field_node` as of 2026-11-13, so check that again, and check the intra-doc links.
