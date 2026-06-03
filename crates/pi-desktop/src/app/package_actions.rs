use super::*;

const PACKAGE_SEARCH_LIMIT: u32 = 6;

impl PiDesktop {
    pub(crate) fn search_packages(&mut self, cx: &mut Context<Self>) {
        let query = self.package_search_input.read(cx).value().trim().to_owned();
        let Some(session) = self.backend.clone() else {
            self.status = "Pi worker backend is not ready yet.".into();
            cx.notify();
            return;
        };

        self.package_pending = true;
        self.status = if query.is_empty() {
            "Searching Pi package catalog…".into()
        } else {
            format!("Searching Pi packages for {query}…").into()
        };
        cx.notify();

        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(
                    async move { session.search_packages(query, PACKAGE_SEARCH_LIMIT) },
                )
                .await;
            let _ = this.update(cx, |view, cx| {
                view.package_pending = false;
                match result {
                    Ok(response) => {
                        view.package_results = response.results;
                        view.status = format!(
                            "Found {} Pi package result{}.",
                            view.package_results.len(),
                            if view.package_results.len() == 1 {
                                ""
                            } else {
                                "s"
                            }
                        )
                        .into();
                    }
                    Err(error) => {
                        view.package_results.clear();
                        view.status = format!("Pi package search failed: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn install_package(
        &mut self,
        source: String,
        project: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.backend.clone() else {
            self.status = "Pi worker backend is not ready yet.".into();
            cx.notify();
            return;
        };
        let cwd = self.cwd.clone();
        self.installing_package = Some(source.clone());
        self.new_installed_package = None;
        self.package_pending = true;
        self.status = format!("Installing Pi package {source}…").into();
        cx.notify();

        let installed_source = source.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { session.install_package(source, project, cwd) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.package_pending = false;
                view.installing_package = None;
                match result {
                    Ok(data) => {
                        view.new_installed_package = Some(installed_source.clone());
                        view.apply_data(
                            data,
                            format!("Installed Pi package {installed_source}."),
                            cx,
                        );
                    }
                    Err(error) => {
                        view.status = format!("Pi package install failed: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn uninstall_package(
        &mut self,
        source: String,
        project: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(session) = self.backend.clone() else {
            self.status = "Pi worker backend is not ready yet.".into();
            cx.notify();
            return;
        };
        let cwd = self.cwd.clone();
        self.removing_package = Some(source.clone());
        self.package_pending = true;
        self.status = format!("Removing Pi package {source}…").into();
        cx.notify();

        let removed_source = source.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { session.remove_package(source, project, cwd) })
                .await;
            let _ = this.update(cx, |view, cx| {
                view.package_pending = false;
                view.removing_package = None;
                match result {
                    Ok(data) => {
                        view.new_installed_package = None;
                        view.apply_data(data, format!("Removed Pi package {removed_source}."), cx);
                    }
                    Err(error) => {
                        view.status = format!("Pi package remove failed: {error:#}").into();
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn installed_packages(&self) -> Vec<pi_bridge_types::InstalledPackage> {
        self.data
            .as_ref()
            .map(|data| data.packages.clone())
            .unwrap_or_default()
    }
}
