use yew::prelude::*;
use web_sys::{HtmlTextAreaElement, KeyboardEvent};
use crate::types::{ActiveTab, Message};

#[derive(Properties, PartialEq)]
pub struct ChatPanelProps {
    pub active_tab: ActiveTab,
    pub messages: Vec<Message>,
    pub on_send_message: Callback<String>,
}

#[function_component(ChatPanel)]
pub fn chat_panel(props: &ChatPanelProps) -> Html {
    if props.active_tab != ActiveTab::Chat {
        return html! {};
    }

    let input_value = use_state(|| String::new());

    let on_input = {
        let input_value = input_value.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(textarea) = e.target_dyn_into::<HtmlTextAreaElement>() {
                input_value.set(textarea.value());
            }
        })
    };

    let on_keydown = {
        let input_value = input_value.clone();
        let on_send = props.on_send_message.clone();
        Callback::from(move |e: KeyboardEvent| {
            if e.key() == "Enter" && !e.shift_key() {
                e.prevent_default();
                let value = (*input_value).clone();
                if !value.trim().is_empty() {
                    on_send.emit(value);
                    input_value.set(String::new());
                }
            }
        })
    };

    let on_submit = {
        let input_value = input_value.clone();
        let on_send = props.on_send_message.clone();
        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();
            let value = (*input_value).clone();
            if !value.trim().is_empty() {
                on_send.emit(value);
                input_value.set(String::new());
            }
        })
    };

    html! {
        <div class="flex flex-col flex-1">
            <div class="p-4 border-b border-gray-300">
                <h2 class="text-lg font-semibold">{"Chat"}</h2>
            </div>

            <div class="flex-1 overflow-y-auto p-4 space-y-3">
                {
                    props.messages.iter().map(|msg| {
                        let is_user = msg.role == "user";
                        html! {
                            <div
                                class={classes!(
                                    "p-3",
                                    "rounded-lg",
                                    if is_user { "bg-blue-100 ml-4" } else { "bg-gray-100 mr-4" }
                                )}
                            >
                                <div class="text-xs font-semibold text-gray-600 mb-1">
                                    {if is_user { "You" } else { "Assistant" }}
                                </div>
                                <div class="text-sm whitespace-pre-wrap">
                                    {&msg.content}
                                </div>
                            </div>
                        }
                    }).collect::<Html>()
                }
            </div>

            <form onsubmit={on_submit} class="p-4 border-t border-gray-300">
                <textarea
                    value={(*input_value).clone()}
                    oninput={on_input}
                    onkeydown={on_keydown}
                    placeholder="Type a message... (Shift+Enter for new line)"
                    class="w-full px-3 py-2 border border-gray-300 rounded-lg text-sm resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
                    rows="3"
                />
                <button
                    type="submit"
                    class="mt-2 w-full px-4 py-2 bg-blue-500 text-white rounded-lg text-sm font-medium hover:bg-blue-600 transition-colors"
                >
                    {"Send"}
                </button>
            </form>
        </div>
    }
}
