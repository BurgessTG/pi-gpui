use std::path::{Component, Path, PathBuf};
use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement, StatefulInteractiveElement as _, Styled as _, div, px, svg,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::input::InputState;
use gpui_component::list::ListItem;
use gpui_component::menu::{ContextMenuExt as _, PopupMenu, PopupMenuItem};
use gpui_component::tree::{TreeState, tree};
use gpui_component::{Icon, IconName, Sizable as _, Size, h_flex};

use crate::app::PiDesktop;
use crate::components::button::{PiButtonKind, pi_button};
use crate::components::input::pi_input;
use crate::design::theme;
use crate::ui;

const NEW_FOLDER_ROW_ANIMATION: Duration = Duration::from_millis(180);

pub fn workspace_dialog_backdrop(cx: &mut Context<PiDesktop>) -> impl IntoElement {
    div()
        .id("workspace-dialog-backdrop")
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .bg(gpui::black())
        .opacity(0.56)
        .on_click(cx.listener(|view, _, _, cx| {
            view.cancel_workspace_dialog(cx);
        }))
}

#[allow(clippy::too_many_arguments)]
pub fn open_workspace_dialog(
    tree_state: &Entity<TreeState>,
    _root_path: &Path,
    selected_path: &Path,
    new_folder_name_input: &Entity<InputState>,
    new_folder_input_visible: bool,
    showing_new_folder_input: bool,
    pending_delete_folder: Option<&Path>,
    showing_delete_folder_confirmation: bool,
    cx: &mut Context<PiDesktop>,
) -> AnyElement {
    dialog_container()
        .child(
            dialog_card("open-workspace-dialog")
                .w(px(640.0))
                .child(picker_navigation(selected_path, cx))
                .child(
                    div()
                        .border_1()
                        .border_color(theme::hairline())
                        .bg(theme::app_bg())
                        .h(px(320.0))
                        .overflow_hidden()
                        .child(directory_tree(tree_state, cx)),
                )
                .when_some(pending_delete_folder, |this, path| {
                    this.child(delete_confirmation_row(
                        path,
                        showing_delete_folder_confirmation,
                        cx,
                    ))
                })
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .items_center()
                        .gap_2()
                        .child(new_folder_button(cx))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .when(new_folder_input_visible, |this| {
                                    this.child(new_folder_row(
                                        new_folder_name_input,
                                        showing_new_folder_input,
                                    ))
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .justify_end()
                                .gap_2()
                                .child(
                                    pi_button(
                                        "cancel-open-workspace",
                                        "Cancel",
                                        PiButtonKind::Ghost,
                                        cx,
                                    )
                                    .on_click(cx.listener(
                                        |view, _, _, cx| {
                                            view.cancel_workspace_dialog(cx);
                                        },
                                    )),
                                )
                                .child(open_workspace_button(cx)),
                        ),
                ),
        )
        .into_any_element()
}

fn dialog_container() -> impl ParentElement + IntoElement {
    div()
        .absolute()
        .top_0()
        .right_0()
        .bottom_0()
        .left_0()
        .p_6()
        .flex()
        .items_center()
        .justify_center()
}

fn dialog_card(_id: &'static str) -> gpui::Div {
    div()
        .w(px(480.0))
        .occlude()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
}

fn picker_navigation(root_path: &Path, cx: &mut Context<PiDesktop>) -> impl IntoElement {
    div().px_1().child(workspace_breadcrumb(root_path, cx))
}

fn workspace_breadcrumb(root_path: &Path, cx: &mut Context<PiDesktop>) -> AnyElement {
    let items = breadcrumb_items(root_path);
    let last_index = items.len().saturating_sub(1);
    let mut children = Vec::new();

    for (index, (label, path, is_current)) in items.into_iter().enumerate() {
        children.push(
            div()
                .id(("breadcrumb-item", index))
                .text_sm()
                .text_color(if is_current {
                    theme::text()
                } else {
                    theme::text_muted()
                })
                .when(!is_current, |this| {
                    this.cursor_pointer()
                        .hover(|style| style.text_color(theme::text()))
                        .on_click(cx.listener(move |view, _, _, cx| {
                            view.navigate_workspace_picker_root(path.clone(), cx);
                        }))
                })
                .child(label)
                .into_any_element(),
        );

        if index != last_index {
            children.push(
                div()
                    .text_sm()
                    .text_color(theme::text())
                    .child("›")
                    .into_any_element(),
            );
        }
    }

    h_flex()
        .gap_1p5()
        .items_center()
        .children(children)
        .into_any_element()
}

fn breadcrumb_items(path: &Path) -> Vec<(String, PathBuf, bool)> {
    let mut items = Vec::new();
    let mut current = PathBuf::new();
    let components = path.components().collect::<Vec<_>>();

    for component in &components {
        match component {
            Component::RootDir => {
                current.push(component.as_os_str());
                items.push(("/".to_owned(), current.clone(), false));
            }
            Component::Normal(label) => {
                current.push(label);
                items.push((label.to_string_lossy().to_string(), current.clone(), false));
            }
            Component::Prefix(prefix) => {
                current.push(prefix.as_os_str());
                items.push((
                    prefix.as_os_str().to_string_lossy().to_string(),
                    current.clone(),
                    false,
                ));
            }
            Component::CurDir | Component::ParentDir => {}
        }
    }

    if items.is_empty() {
        items.push((path.display().to_string(), path.to_path_buf(), false));
    }
    if let Some(last) = items.last_mut() {
        last.2 = true;
    }

    items
}

fn new_folder_row(new_folder_name_input: &Entity<InputState>, opening: bool) -> AnyElement {
    let target_opacity = if opening { 1.0 } else { 0.0 };

    div()
        .id("new-folder-name-row")
        .w_full()
        .h(px(28.0))
        .overflow_hidden()
        .opacity(target_opacity)
        .child(pi_input(new_folder_name_input).h(px(28.0)).w_full())
        .with_animation(
            ("new-folder-name-row", usize::from(opening)),
            Animation::new(NEW_FOLDER_ROW_ANIMATION)
                .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
            move |this, delta| {
                let progress = if opening { delta } else { 1.0 - delta };
                this.opacity(progress)
            },
        )
        .into_any_element()
}

fn new_folder_button(cx: &mut Context<PiDesktop>) -> AnyElement {
    div()
        .id("new-folder")
        .size(px(28.0))
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .border_1()
        .border_color(theme::hairline())
        .text_color(theme::text())
        .hover(|style| style.bg(theme::surface_hover()))
        .on_click(cx.listener(|view, _, window, cx| {
            view.start_new_folder_flow(window, cx);
        }))
        .child(
            svg()
                .path(ui::folder_plus_icon_path())
                .size(px(16.0))
                .text_color(theme::text()),
        )
        .into_any_element()
}

fn open_workspace_button(cx: &mut Context<PiDesktop>) -> AnyElement {
    div()
        .id("confirm-open-workspace")
        .h(px(28.0))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .gap_1()
        .cursor_pointer()
        .border_1()
        .border_color(theme::accent())
        .bg(theme::accent())
        .text_color(theme::app_bg())
        .hover(|style| {
            style
                .bg(theme::text_muted())
                .border_color(theme::text_muted())
        })
        .on_click(cx.listener(|view, _, _, cx| {
            view.open_selected_workspace(cx);
        }))
        .child(
            Icon::new(IconName::FolderOpen)
                .with_size(Size::XSmall)
                .text_color(theme::app_bg()),
        )
        .child(div().text_sm().child("Open"))
        .into_any_element()
}

fn delete_confirmation_row(path: &Path, showing: bool, cx: &mut Context<PiDesktop>) -> AnyElement {
    let target_opacity = if showing { 1.0 } else { 0.0 };
    let label = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_else(|| path.to_str().unwrap_or("folder"));

    h_flex()
        .id("delete-folder-confirmation")
        .w_full()
        .min_h(px(32.0))
        .px_2()
        .py_1()
        .gap_2()
        .items_center()
        .border_1()
        .border_color(theme::danger())
        .bg(theme::danger_soft())
        .text_color(theme::text())
        .opacity(target_opacity)
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_sm()
                .child(format!("Delete '{label}' and all contents?")),
        )
        .child(
            pi_button("cancel-delete-folder", "Cancel", PiButtonKind::Ghost, cx).on_click(
                cx.listener(|view, _, _, cx| {
                    view.cancel_delete_workspace_folder(cx);
                }),
            ),
        )
        .child(delete_folder_button(cx))
        .with_animation(
            ("delete-folder-confirmation", usize::from(showing)),
            Animation::new(NEW_FOLDER_ROW_ANIMATION)
                .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
            move |this, delta| {
                let progress = if showing { delta } else { 1.0 - delta };
                this.opacity(progress)
            },
        )
        .into_any_element()
}

fn delete_folder_button(cx: &mut Context<PiDesktop>) -> AnyElement {
    div()
        .id("confirm-delete-folder")
        .h(px(28.0))
        .px_2()
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .border_1()
        .border_color(theme::danger())
        .bg(theme::danger())
        .text_color(theme::app_bg())
        .text_sm()
        .hover(|style| {
            style
                .bg(theme::complement())
                .border_color(theme::complement())
        })
        .on_click(cx.listener(|view, _, _, cx| {
            view.confirm_delete_workspace_folder(cx);
        }))
        .child("Delete")
        .into_any_element()
}

fn directory_tree(tree_state: &Entity<TreeState>, cx: &mut Context<PiDesktop>) -> impl IntoElement {
    let view = cx.entity().downgrade();
    tree(tree_state, move |index, entry, selected, _, _| {
        let icon = if entry.is_folder() && entry.is_expanded() {
            IconName::FolderOpen
        } else {
            IconName::Folder
        };
        let indent = px(10.0 + entry.depth() as f32 * 16.0);
        let path = PathBuf::from(entry.item().id.to_string());
        let can_delete = entry.depth() > 0;
        let menu_view = view.clone();
        let menu_path = path.clone();

        ListItem::new(index)
            .selected(selected)
            .pl(indent)
            .mr(px(18.0))
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(icon)
                            .with_size(Size::Small)
                            .text_color(theme::text_muted()),
                    )
                    .child(entry.item().label.clone())
                    .context_menu(move |menu, _window, _cx| {
                        folder_context_menu(menu, menu_view.clone(), menu_path.clone(), can_delete)
                    }),
            )
    })
}

fn folder_context_menu(
    menu: PopupMenu,
    view: gpui::WeakEntity<PiDesktop>,
    path: PathBuf,
    can_delete: bool,
) -> PopupMenu {
    menu.label(path.display().to_string()).separator().item(
        PopupMenuItem::new("Delete Folder…")
            .icon(IconName::Delete)
            .disabled(!can_delete)
            .on_click(move |_, _, cx| {
                let _ = view.update(cx, |view, cx| {
                    view.request_delete_workspace_folder(path.clone(), cx);
                });
            }),
    )
}
