use crate::compositor::{Component, Compositor, Context, EventResult};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tui::{
    buffer::Buffer as Surface,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders},
};

use std::borrow::Cow;

use helix_core::{Position, Transaction};
use helix_view::Editor;

use crate::commands;
use crate::ui::{Menu, Popup, PromptEvent};

use helix_lsp::lsp;
use lsp::CompletionItem;

/// Wraps a Menu.
pub struct Completion {
    popup: Popup<Menu<CompletionItem>>, // TODO: Popup<Menu> need to be able to access contents.
    trigger_offset: usize,
    // TODO: maintain a completioncontext with trigger kind & trigger char
}

impl Completion {
    pub fn new(items: Vec<CompletionItem>, trigger_offset: usize) -> Self {
        // let items: Vec<CompletionItem> = Vec::new();
        let mut menu = Menu::new(
            items,
            |item| {
                // format_fn
                item.label.as_str().into()

                // TODO: use item.filter_text for filtering
            },
            move |editor: &mut Editor, item, event| {
                match event {
                    PromptEvent::Abort => {
                        // revert state
                        // let id = editor.view().doc;
                        // let doc = &mut editor.documents[id];
                        // doc.state = snapshot.clone();
                    }
                    PromptEvent::Validate => {
                        let (view, doc) = editor.current();

                        // revert state to what it was before the last update
                        // doc.state = snapshot.clone();

                        // extract as fn(doc, item):

                        // TODO: need to apply without composing state...
                        // TODO: need to update lsp on accept/cancel by diffing the snapshot with
                        // the final state?
                        // -> on update simply update the snapshot, then on accept redo the call,
                        // finally updating doc.changes + notifying lsp.
                        //
                        // or we could simply use doc.undo + apply when changing between options

                        // always present here
                        let item = item.unwrap();

                        use helix_lsp::{lsp, util};
                        // determine what to insert: text_edit | insert_text | label
                        let edit = if let Some(edit) = &item.text_edit {
                            match edit {
                                lsp::CompletionTextEdit::Edit(edit) => edit.clone(),
                                lsp::CompletionTextEdit::InsertAndReplace(item) => {
                                    unimplemented!("completion: insert_and_replace {:?}", item)
                                }
                            }
                        } else {
                            item.insert_text.as_ref().unwrap_or(&item.label);
                            unimplemented!();
                            // lsp::TextEdit::new(); TODO: calculate a TextEdit from insert_text
                            // and we insert at position.
                        };

                        // TODO: merge edit with additional_text_edits
                        if let Some(additional_edits) = &item.additional_text_edits {
                            if !additional_edits.is_empty() {
                                unimplemented!(
                                    "completion: additional_text_edits: {:?}",
                                    additional_edits
                                );
                            }
                        }

                        // if more text was entered, remove it
                        let cursor = doc.selection(view.id).cursor();
                        if trigger_offset < cursor {
                            let remove = Transaction::change(
                                doc.text(),
                                vec![(trigger_offset, cursor, None)].into_iter(),
                            );
                            doc.apply(&remove, view.id);
                        }

                        let transaction =
                            util::generate_transaction_from_edits(doc.text(), vec![edit]);
                        doc.apply(&transaction, view.id);
                    }
                    _ => (),
                };
            },
        );
        let popup = Popup::new(menu);
        Self {
            popup,
            trigger_offset,
        }
    }

    pub fn update(&mut self, cx: &mut commands::Context) {
        // recompute menu based on matches
        let menu = self.popup.contents_mut();
        let (view, doc) = cx.editor.current();

        // cx.hooks()
        // cx.add_hook(enum type,  ||)
        // cx.trigger_hook(enum type, &str, ...) <-- there has to be enough to identify doc/view
        // callback with editor & compositor
        //
        // trigger_hook sends event into channel, that's consumed in the global loop and
        // triggers all registered callbacks
        // TODO: hooks should get processed immediately so maybe do it after select!(), before
        // looping?

        let cursor = doc.selection(view.id).cursor();
        if self.trigger_offset <= cursor {
            let fragment = doc.text().slice(self.trigger_offset..cursor);
            let text = Cow::from(fragment);
            // TODO: logic is same as ui/picker
            menu.score(&text);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.popup.contents().is_empty()
    }
}

// need to:
// - trigger on the right trigger char
//   - detect previous open instance and recycle
// - update after input, but AFTER the document has changed
// - if no more matches, need to auto close
//
// missing bits:
// - a more robust hook system: emit to a channel, process in main loop
// - a way to find specific layers in compositor
// - components register for hooks, then unregister when terminated
// ... since completion is a special case, maybe just build it into doc/render?

impl Component for Completion {
    fn handle_event(&mut self, event: Event, cx: &mut Context) -> EventResult {
        self.popup.handle_event(event, cx)
    }

    fn required_size(&mut self, viewport: (u16, u16)) -> Option<(u16, u16)> {
        self.popup.required_size(viewport)
    }

    fn render(&self, area: Rect, surface: &mut Surface, cx: &mut Context) {
        self.popup.render(area, surface, cx)
    }
}
