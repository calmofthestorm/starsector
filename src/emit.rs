use indextree::NodeId;

use ropey::Rope;

use crate::{Arena, RopeExt};

pub(crate) fn section_tree_to_rope(
    root_id: NodeId,
    arena: &Arena,
    terminal_newline: bool,
    empty_root_section: bool,
) -> Rope {
    let mut text = Rope::default();

    let mut owe_newline = false;

    let root = &arena.arena[root_id].get();

    if root.level > 0 {
        if owe_newline {
            text.push('\n');
        }
        text.append(root.text.clone());
        owe_newline = true;
    } else if root.text.is_empty() {
        if !empty_root_section {
            if owe_newline {
                text.push('\n');
            }
            owe_newline = true;
        }
    } else {
        if owe_newline {
            text.push('\n');
        }
        text.append(root.text.clone());
        owe_newline = true;
    }

    fn emitter(node: NodeId, arena: &Arena, text: &mut Rope, owe_newline: &mut bool) {
        for child in node.children(&arena.arena) {
            if *owe_newline {
                text.push('\n');
            }
            text.append(arena.arena[child].get().text.clone());
            *owe_newline = true;
            emitter(child, arena, text, owe_newline);
        }
    }

    emitter(root_id, arena, &mut text, &mut owe_newline);

    if terminal_newline && owe_newline {
        text.push('\n');
    }

    text
}
