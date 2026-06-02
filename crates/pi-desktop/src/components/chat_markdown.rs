use gpui::{
    AnyElement, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render,
    SharedString, Styled as _, Window, div,
};
use gpui_component::text::TextView;

use crate::design::theme;

pub struct MarkdownBlockView {
    id: SharedString,
    text: String,
    scale: f32,
}

impl MarkdownBlockView {
    pub fn new(id: SharedString, text: String, scale: f32) -> Self {
        Self { id, text, scale }
    }

    pub fn sync(&mut self, text: String, scale: f32, cx: &mut Context<Self>) -> bool {
        if self.text == text && (self.scale - scale).abs() <= f32::EPSILON {
            return false;
        }
        self.text = text;
        self.scale = scale;
        cx.notify();
        true
    }
}

impl Render for MarkdownBlockView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TextView::markdown(self.id.clone(), self.text.clone(), window, &mut *cx)
            .selectable(true)
            .text_size(gpui::px(14.0 * self.scale))
            .text_color(theme::text())
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sync_markdown_blocks<C>(
    views: &mut Vec<Entity<MarkdownBlockView>>,
    workspace_id: usize,
    node_id: usize,
    message_index: usize,
    text: &str,
    scale: f32,
    cx: &mut Context<C>,
) {
    let blocks = split_markdown_blocks(text);
    let shared_len = views.len().min(blocks.len());
    for (block_index, block) in blocks.iter().take(shared_len).cloned().enumerate() {
        let view = views[block_index].clone();
        view.update(cx, |view, cx| {
            view.sync(block, scale, cx);
        });
    }
    if blocks.len() < views.len() {
        views.truncate(blocks.len());
        return;
    }
    for (block_index, block) in blocks.into_iter().enumerate().skip(shared_len) {
        let id = SharedString::from(format!(
            "chat-node-{workspace_id}-{node_id}-assistant-{message_index}-block-{block_index}"
        ));
        views.push(cx.new(|_| MarkdownBlockView::new(id, block, scale)));
    }
}

pub fn render_markdown_blocks(views: &[Entity<MarkdownBlockView>]) -> AnyElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(gpui::px(8.0))
        .children(views.iter().cloned())
        .into_any_element()
}

fn split_markdown_blocks(text: &str) -> Vec<String> {
    let content = if text.trim().is_empty() { "" } else { text };
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    let mut in_fence = false;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence && line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
            continue;
        }
        current.push(line.to_owned());
    }
    if !current.is_empty() || blocks.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::split_markdown_blocks;

    #[test]
    fn splits_blank_line_markdown_blocks_without_splitting_code_fences() {
        let blocks = split_markdown_blocks("one\n\n```\na\n\n b\n```\n\ntwo");
        assert_eq!(blocks, vec!["one", "```\na\n\n b\n```", "two"]);
    }
}
