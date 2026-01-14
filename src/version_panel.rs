use yew::prelude::*;
use web_sys::window;
use crate::types::ActiveTab;
use crate::version::VersionHistory;

#[derive(Properties, PartialEq)]
pub struct VersionHistoryPanelProps {
    pub active_tab: ActiveTab,
    pub history: VersionHistory,
    pub has_unsaved_changes: bool,
    pub on_save_version: Callback<()>,
    pub on_restore_version: Callback<usize>,
}

#[function_component(VersionHistoryPanel)]
pub fn version_history_panel(props: &VersionHistoryPanelProps) -> Html {
    if props.active_tab != ActiveTab::Versions {
        return html! {};
    }

    let on_save = {
        let on_save_version = props.on_save_version.clone();
        Callback::from(move |_: MouseEvent| {
            on_save_version.emit(());
        })
    };

    html! {
        <div class="flex flex-col flex-1">
            // Header
            <div class="p-4 border-b border-gray-300">
                <h2 class="text-lg font-semibold">{"Version History"}</h2>
                <p class="text-xs text-gray-500 mt-1">
                    {format!("{} version(s) saved", props.history.len())}
                </p>
            </div>

            // Save Button
            <div class="p-4 border-b border-gray-300">
                <button
                    onclick={on_save}
                    class="w-full px-4 py-2 bg-blue-500 text-white rounded-lg text-sm font-medium hover:bg-blue-600 transition-colors"
                >
                    {"Save Version"}
                </button>
                if props.has_unsaved_changes {
                    <p class="text-xs text-amber-600 mt-2 text-center">
                        {"Unsaved changes"}
                    </p>
                }
            </div>

            // Version List
            <div class="flex-1 overflow-y-auto p-4 space-y-2">
                {
                    props.history.versions.iter().enumerate().rev().map(|(idx, version)| {
                        let is_current = props.history.current_version_idx == Some(idx);
                        let on_restore = props.on_restore_version.clone();
                        let version_label = version.label.clone();
                        let onclick = Callback::from(move |_: MouseEvent| {
                            if let Some(win) = window() {
                                let msg = format!("Are you sure you want to restore to '{}'? Any unsaved changes will be lost.", version_label);
                                if let Ok(true) = win.confirm_with_message(&msg) {
                                    on_restore.emit(idx);
                                }
                            }
                        });

                        html! {
                            <div
                                key={version.id}
                                {onclick}
                                class={classes!(
                                    "p-3",
                                    "rounded-lg",
                                    "cursor-pointer",
                                    "border",
                                    "transition-colors",
                                    if is_current {
                                        "bg-blue-50 border-blue-300"
                                    } else {
                                        "bg-gray-50 border-gray-200 hover:bg-gray-100 hover:border-gray-300"
                                    }
                                )}
                            >
                                <div class="flex items-center justify-between gap-2">
                                    <span class="font-medium text-sm">{&version.label}</span>
                                    if is_current {
                                        <span class="text-xs bg-blue-500 text-white px-2 py-0.5 rounded">
                                            {"Current"}
                                        </span>
                                    }
                                </div>
                                <div class="text-xs text-gray-500 mt-1">
                                    {format_timestamp(version.created_at)}
                                </div>
                                <div class="text-xs text-gray-400 mt-1">
                                    {format!("{} shape(s)", version.shapes.len())}
                                </div>
                            </div>
                        }
                    }).collect::<Html>()
                }

                if props.history.is_empty() {
                    <p class="text-sm text-gray-500 text-center py-4">
                        {"No versions saved yet. Click 'Save Version' to create your first snapshot."}
                    </p>
                }
            </div>
        </div>
    }
}

fn format_timestamp(ts: f64) -> String {
    // Convert milliseconds to seconds for display
    // In a real app, use a date formatting library
    let total_seconds = (ts / 1000.0) as u64;
    let hours = (total_seconds / 3600) % 24;
    let minutes = (total_seconds / 60) % 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
