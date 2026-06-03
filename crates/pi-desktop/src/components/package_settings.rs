use std::time::Duration;

use gpui::prelude::FluentBuilder as _;
use gpui::{
    Animation, AnimationExt as _, AnyElement, App, Context, Entity, InteractiveElement as _,
    IntoElement, ParentElement as _, SharedString, Styled as _, Window, div, px,
};
use gpui_component::animation::cubic_bezier;
use gpui_component::table::{Column, Table, TableDelegate, TableState};
use gpui_component::{Sizable as _, Size, StyledExt as _};
use pi_bridge_types::{InstalledPackage, PackageScope, PackageSearchResult};

use crate::app::PiDesktop;
use crate::components::button::{PiButtonKind, pi_button};
use crate::components::input::pi_input;
use crate::design::theme;

const PACKAGE_RESULT_LIMIT: usize = 6;

pub(crate) struct PackageSettingsState {
    pub(crate) results: Vec<PackageSearchResult>,
    pub(crate) installed: Vec<InstalledPackage>,
    pub(crate) installing_source: Option<String>,
    pub(crate) removing_source: Option<String>,
    pub(crate) new_installed_source: Option<String>,
    pub(crate) pending: bool,
    pub(crate) canvas_node_count: usize,
}

pub(crate) fn package_settings_content(
    state: PackageSettingsState,
    search_input: Entity<gpui_component::input::InputState>,
    table: Entity<TableState<InstalledPackagesTableDelegate>>,
    view: Entity<PiDesktop>,
    cx: &mut App,
) -> AnyElement {
    sync_table(
        &table,
        state.installed,
        state.removing_source.clone(),
        state.new_installed_source.clone(),
        cx,
    );

    let query = search_input.read(cx).value().to_string();
    let search_view = view.clone();

    div()
        .w_full()
        .h_full()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .w_full()
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::app_bg())
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(section_header(
                    "Package search",
                    "Search npm for packages tagged pi-package, matching pi.dev/packages.",
                ))
                .child(
                    div()
                        .w_full()
                        .flex()
                        .items_end()
                        .gap_2()
                        .child(div().flex_1().child(pi_input(&search_input).h(px(38.0))))
                        .child(
                            pi_button("package-search", "Search", PiButtonKind::Primary, cx)
                                .loading(state.pending)
                                .on_click(move |_, _, cx| {
                                    search_view.update(cx, |view, cx| view.search_packages(cx));
                                }),
                        ),
                )
                .when(state.results.is_empty(), |this| {
                    this.child(div().text_xs().text_color(theme::text_muted()).child(
                        if query.trim().is_empty() {
                            "Try “web”, “memory”, “subagents”, or leave blank for top packages."
                        } else {
                            "No package results yet. Press Search to query the catalog."
                        },
                    ))
                })
                .when(!state.results.is_empty(), |this| {
                    this.child(
                        div().w_full().flex().flex_col().gap_2().children(
                            state
                                .results
                                .into_iter()
                                .take(PACKAGE_RESULT_LIMIT)
                                .map(|result| {
                                    package_result_card(
                                        result,
                                        state.installing_source.clone(),
                                        view.clone(),
                                        cx,
                                    )
                                    .into_any_element()
                                }),
                        ),
                    )
                }),
        )
        .child(
            div()
                .w_full()
                .flex_1()
                .min_h(px(190.0))
                .border_1()
                .border_color(theme::hairline())
                .bg(theme::app_bg())
                .p_3()
                .flex()
                .flex_col()
                .gap_3()
                .child(section_header(
                    "Installed packages",
                    if state.canvas_node_count == 0 {
                        "User and project packages currently configured for Pi."
                    } else {
                        "User and project packages currently configured for Pi, including canvas node manifests."
                    },
                ))
                .child(
                    div()
                        .w_full()
                        .flex_1()
                        .min_h(px(132.0))
                        .child(Table::new(&table).with_size(Size::Small).bordered(false)),
                ),
        )
        .into_any_element()
}

fn sync_table(
    table: &Entity<TableState<InstalledPackagesTableDelegate>>,
    packages: Vec<InstalledPackage>,
    removing_source: Option<String>,
    new_source: Option<String>,
    cx: &mut App,
) {
    table.update(cx, |table, cx| {
        table
            .delegate_mut()
            .set_packages(packages, removing_source, new_source);
        cx.notify();
    });
}

fn section_header(title: &'static str, description: &'static str) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_sm().font_semibold().child(title))
        .child(
            div()
                .text_xs()
                .text_color(theme::text_muted())
                .child(description),
        )
}

fn package_result_card(
    result: PackageSearchResult,
    installing_source: Option<String>,
    view: Entity<PiDesktop>,
    cx: &App,
) -> impl IntoElement {
    let name = result.name.clone();
    let source = format!("npm:{}", result.name);
    let install_source = source.clone();
    let installing = installing_source.as_deref() == Some(source.as_str())
        || installing_source.as_deref() == Some(result.name.as_str());
    let publisher = result.publisher.as_deref().unwrap_or("unknown");
    let downloads = result
        .monthly_downloads
        .map(format_downloads)
        .unwrap_or_else(|| "—/mo".to_owned());
    let updated = result.updated.as_deref().map(short_date).unwrap_or("—");
    let tags = if result.resource_types.is_empty() {
        "package".to_owned()
    } else {
        result.resource_types.join(" · ")
    };

    div()
        .w_full()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::surface())
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .line_clamp(1)
                                .child(format!("{}@{}", result.name, result.version)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme::text_muted())
                                .line_clamp(2)
                                .child(result.description),
                        ),
                )
                .child(
                    pi_button("install-package", "Install", PiButtonKind::Secondary, cx)
                        .loading(installing)
                        .on_click(move |_, _, cx| {
                            view.update(cx, |view, cx| {
                                view.install_package(install_source.clone(), false, cx);
                            });
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_2()
                .text_xs()
                .text_color(theme::text_muted())
                .child(meta_pill(publisher.to_owned()))
                .child(meta_pill(downloads))
                .child(meta_pill(updated.to_owned()))
                .child(meta_pill(tags)),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::text_muted())
                .child(format!("$ pi install {source}")),
        )
        .child(
            div()
                .text_xs()
                .text_color(theme::text_muted())
                .line_clamp(1)
                .child(result.repository_url.unwrap_or(result.npm_url)),
        )
        .id(SharedString::from(format!("package-result-{name}")))
}

fn meta_pill(label: String) -> impl IntoElement {
    div()
        .border_1()
        .border_color(theme::hairline())
        .bg(theme::app_bg())
        .px_1p5()
        .py_0p5()
        .child(label)
}

#[derive(Clone)]
pub(crate) struct InstalledPackagesTableDelegate {
    columns: Vec<Column>,
    packages: Vec<InstalledPackage>,
    removing_source: Option<String>,
    new_source: Option<String>,
    view: Entity<PiDesktop>,
}

impl InstalledPackagesTableDelegate {
    pub(crate) fn new(view: Entity<PiDesktop>) -> Self {
        Self {
            columns: vec![
                Column::new("package", "Package")
                    .width(px(150.0))
                    .resizable(false),
                Column::new("scope", "Scope")
                    .width(px(70.0))
                    .resizable(false),
                Column::new("version", "Version")
                    .width(px(82.0))
                    .resizable(false),
                Column::new("nodes", "Nodes")
                    .width(px(62.0))
                    .resizable(false),
                Column::new("path", "Path")
                    .width(px(150.0))
                    .resizable(false),
                Column::new("remove", "").width(px(44.0)).resizable(false),
            ],
            packages: Vec::new(),
            removing_source: None,
            new_source: None,
            view,
        }
    }

    fn set_packages(
        &mut self,
        packages: Vec<InstalledPackage>,
        removing_source: Option<String>,
        new_source: Option<String>,
    ) {
        self.packages = packages;
        self.removing_source = removing_source;
        self.new_source = new_source;
    }
}

impl TableDelegate for InstalledPackagesTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.packages.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> &Column {
        &self.columns[col_ix]
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(package) = self.packages.get(row_ix).cloned() else {
            return div().into_any_element();
        };
        let source = package.source.clone();
        let project = package.scope == PackageScope::Project;
        let removing = self.removing_source.as_deref() == Some(source.as_str());
        let is_new = self.new_source.as_deref() == Some(source.as_str());
        let view = self.view.clone();
        let remove_source = source.clone();

        match col_ix {
            0 => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .child(package.display_name),
                is_new,
                &source,
                col_ix,
            ),
            1 => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(match package.scope {
                        PackageScope::User => "user",
                        PackageScope::Project => "project",
                    }),
                is_new,
                &source,
                col_ix,
            ),
            2 => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(package.version.unwrap_or_else(|| "—".to_owned())),
                is_new,
                &source,
                col_ix,
            ),
            3 => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(match package.canvas_nodes.len() {
                        0 => "—".to_owned(),
                        count => count.to_string(),
                    }),
                is_new,
                &source,
                col_ix,
            ),
            4 => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .line_clamp(1)
                    .child(
                        package
                            .installed_path
                            .unwrap_or_else(|| package.source.clone()),
                    ),
                is_new,
                &source,
                col_ix,
            ),
            _ => fade_cell(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        pi_button("remove-package", "×", PiButtonKind::Danger, cx)
                            .loading(removing)
                            .on_click(move |_, _, cx| {
                                view.update(cx, |view, cx| {
                                    view.uninstall_package(remove_source.clone(), project, cx);
                                });
                            }),
                    ),
                is_new,
                &source,
                col_ix,
            ),
        }
    }

    fn render_empty(
        &mut self,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .text_xs()
            .text_color(theme::text_muted())
            .child("No packages installed yet.")
    }
}

fn fade_cell(cell: gpui::Div, is_new: bool, source: &str, col_ix: usize) -> AnyElement {
    if is_new {
        cell.with_animation(
            SharedString::from(format!("installed-package-cell-{source}-{col_ix}")),
            Animation::new(Duration::from_millis(720))
                .with_easing(cubic_bezier(0.32, 0.72, 0.0, 1.0)),
            |cell, delta| cell.opacity(delta),
        )
        .into_any_element()
    } else {
        cell.into_any_element()
    }
}

fn format_downloads(value: u32) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M/mo", value as f32 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K/mo", value as f32 / 1_000.0)
    } else {
        format!("{value}/mo")
    }
}

fn short_date(value: &str) -> &str {
    value.split('T').next().unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_download_counts_like_catalog_meta() {
        assert_eq!(format_downloads(999), "999/mo");
        assert_eq!(format_downloads(12_300), "12.3K/mo");
        assert_eq!(format_downloads(1_200_000), "1.2M/mo");
    }
}
