use crate::plugin::{PluginState, load_plugins, refresh_plugin_state, trigger_entry};
use cosmic::app::{Core, Task};
use cosmic::applet::menu_button;
use cosmic::iced::platform_specific::shell::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::time;
use cosmic::iced::widget::{column, container, scrollable};
use cosmic::iced::{Alignment, Length, Subscription, window};
use cosmic::theme;
use cosmic::widget::{
    Id, autosize, divider,
    rectangle_tracker::{RectangleTracker, RectangleUpdate, rectangle_tracker_subscription},
    text,
};
use cosmic::{Element, widget};
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

const APP_ID: &str = "io.github.alexprates.CBar";
static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

pub fn run() -> cosmic::iced::Result {
    cosmic::applet::run::<CBarApplet>(())
}

struct CBarApplet {
    core: Core,
    popup: Option<window::Id>,
    plugin_dir: PathBuf,
    plugins: Vec<PluginState>,
    status: String,
    rectangle_tracker: Option<RectangleTracker<u32>>,
    refresh_in_flight: Vec<bool>,
    pending_refresh: Vec<bool>,
    pending_force_refresh: Vec<bool>,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    Tick,
    PluginsLoaded(Vec<PluginState>),
    PluginRefreshed {
        index: usize,
        plugin: PluginState,
    },
    RefreshAll,
    Rectangle(RectangleUpdate<u32>),
    RunEntry {
        plugin_index: usize,
        entry_index: usize,
        alternate: bool,
    },
    EntryFinished(Result<bool, String>),
}

impl cosmic::Application for CBarApplet {
    type Executor = cosmic::SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let plugin_dir = default_plugin_dir();

        (
            Self {
                core,
                popup: None,
                plugin_dir: plugin_dir.clone(),
                plugins: Vec::new(),
                status: "loading".to_owned(),
                rectangle_tracker: None,
                refresh_in_flight: Vec::new(),
                pending_refresh: Vec::new(),
                pending_force_refresh: Vec::new(),
            },
            Task::perform(
                load_plugins(plugin_dir),
                app_message(Message::PluginsLoaded),
            ),
        )
    }

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::TogglePopup => {
                if let Some(popup) = self.popup.take() {
                    return destroy_popup(popup);
                }

                let new_id = window::Id::unique();
                self.popup = Some(new_id);
                let popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().unwrap(),
                    new_id,
                    Some((420, 520)),
                    None,
                    None,
                );
                return get_popup(popup_settings);
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                }
            }
            Message::Tick => {
                return self.spawn_due_refreshes(false);
            }
            Message::PluginsLoaded(plugins) => {
                self.plugins = plugins;
                self.sync_refresh_tracking();
                self.update_status();
            }
            Message::PluginRefreshed { index, plugin } => {
                if let Some(slot) = self.plugins.get_mut(index) {
                    *slot = plugin;
                }

                if let Some(in_flight) = self.refresh_in_flight.get_mut(index) {
                    *in_flight = false;
                }

                self.update_status();

                let force = self
                    .pending_force_refresh
                    .get(index)
                    .copied()
                    .unwrap_or(false);
                let pending = self.pending_refresh.get(index).copied().unwrap_or(false) || force;

                if let Some(flag) = self.pending_refresh.get_mut(index) {
                    *flag = false;
                }
                if let Some(flag) = self.pending_force_refresh.get_mut(index) {
                    *flag = false;
                }

                if pending {
                    return self.spawn_refresh_for_index(index, force);
                }
            }
            Message::RefreshAll => {
                return self.spawn_due_refreshes(true);
            }
            Message::Rectangle(update) => match update {
                RectangleUpdate::Init(tracker) => {
                    self.rectangle_tracker = Some(tracker);
                }
                RectangleUpdate::Rectangle(_) => {}
            },
            Message::RunEntry {
                plugin_index,
                entry_index,
                alternate,
            } => {
                if let Some(plugin) = self.plugins.get(plugin_index).cloned()
                    && let Some(entry) = plugin.menu_entries().get(entry_index).cloned()
                {
                    let chosen_entry = if alternate {
                        entry.alternate.as_deref().cloned().unwrap_or(entry)
                    } else {
                        entry
                    };
                    return Task::perform(
                        async move { trigger_entry(&plugin, &chosen_entry).await },
                        app_message(Message::EntryFinished),
                    );
                }
            }
            Message::EntryFinished(result) => match result {
                Ok(true) => return self.update(Message::RefreshAll),
                Ok(false) => {}
                Err(err) => {
                    self.status = err;
                }
            },
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let label = panel_label(&self.plugins, &self.status);
        let button = self
            .core
            .applet
            .text_button(self.core.applet.text(label), Message::TogglePopup);

        autosize::autosize(
            if let Some(tracker) = self.rectangle_tracker.as_ref() {
                Element::from(tracker.container(0, button).ignore_bounds(true))
            } else {
                button.into()
            },
            AUTOSIZE_MAIN_ID.clone(),
        )
        .into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if !matches!(self.popup, Some(popup) if popup == id) {
            return widget::text("").into();
        }

        let spacing = theme::active().cosmic().spacing;

        let mut content = column![
            text::body(format!("Plugin dir: {}", self.plugin_dir.display())),
            menu_button(text::body("Refresh now")).on_press(Message::RefreshAll),
            divider::horizontal::default(),
        ]
        .spacing(spacing.space_xxs)
        .padding([8, 0])
        .align_x(Alignment::Start);

        if self.plugins.is_empty() {
            content = content.push(text::body("Nenhum plugin executável encontrado."));
        }

        for (plugin_index, plugin) in self.plugins.iter().enumerate() {
            content = content.push(text::body(plugin.title().to_owned()).size(14));

            if plugin.cycle_items().len() > 1 {
                content = content.push(text::body(format!(
                    "cycle: {}",
                    plugin.cycle_items().join(" | ")
                )));
            }

            if let Some(error) = &plugin.last_error {
                content = content.push(text::body(format!("erro: {error}")));
            }

            for (entry_index, entry) in plugin.menu_entries().iter().enumerate() {
                if entry.separator {
                    content = content.push(divider::horizontal::default());
                    continue;
                }

                content = content.push(render_entry(plugin_index, entry_index, entry, false));

                if let Some(alternate) = entry.alternate.as_ref() {
                    content =
                        content.push(render_entry(plugin_index, entry_index, alternate, true));
                }
            }

            content = content.push(divider::horizontal::default());
        }

        self.core
            .applet
            .popup_container(container(scrollable(content)).width(Length::Fixed(420.0)))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(1)).map(|_| Message::Tick),
            rectangle_tracker_subscription(0).map(|event| Message::Rectangle(event.1)),
        ])
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }
}

impl CBarApplet {
    fn spawn_due_refreshes(&mut self, force: bool) -> Task<Message> {
        let now = Instant::now();
        let mut tasks = Vec::new();

        for index in 0..self.plugins.len() {
            let should_refresh = force || self.plugins[index].next_refresh_at <= now;
            if !should_refresh {
                continue;
            }

            if self.refresh_in_flight.get(index).copied().unwrap_or(false) {
                if force {
                    if let Some(flag) = self.pending_force_refresh.get_mut(index) {
                        *flag = true;
                    }
                } else if let Some(flag) = self.pending_refresh.get_mut(index) {
                    *flag = true;
                }
                continue;
            }

            tasks.push(self.spawn_refresh_for_index(index, force));
        }

        Task::batch(tasks)
    }

    fn spawn_refresh_for_index(&mut self, index: usize, force: bool) -> Task<Message> {
        let Some(plugin) = self.plugins.get(index).cloned() else {
            return Task::none();
        };

        if let Some(in_flight) = self.refresh_in_flight.get_mut(index) {
            *in_flight = true;
        }

        if force {
            if let Some(flag) = self.pending_force_refresh.get_mut(index) {
                *flag = false;
            }
        } else if let Some(flag) = self.pending_refresh.get_mut(index) {
            *flag = false;
        }

        Task::perform(
            refresh_plugin_state(plugin),
            app_message(move |plugin| Message::PluginRefreshed { index, plugin }),
        )
    }

    fn sync_refresh_tracking(&mut self) {
        let count = self.plugins.len();
        self.refresh_in_flight = vec![false; count];
        self.pending_refresh = vec![false; count];
        self.pending_force_refresh = vec![false; count];
    }

    fn update_status(&mut self) {
        self.status = if self.plugins.is_empty() {
            format!("no plugins in {}", self.plugin_dir.display())
        } else {
            format!("{} plugin(s)", self.plugins.len())
        };
    }
}

fn render_entry(
    plugin_index: usize,
    entry_index: usize,
    entry: &crate::parser::MenuEntry,
    is_alternate: bool,
) -> Element<'static, Message> {
    let prefix = if is_alternate { "[alt] " } else { "" };
    let mut label = format!("{prefix}{}", entry.text);

    if entry.params.href.is_some() {
        label.push_str(" ↗");
    }

    let left_padding = 12 + (entry.level as u16 * 16) + if is_alternate { 12 } else { 0 };
    let content = widget::container(text::body(label).width(Length::Fill))
        .padding([0, 0, 0, left_padding])
        .width(Length::Fill);

    if entry.params.disabled {
        return content.into();
    }

    menu_button(content)
        .on_press(Message::RunEntry {
            plugin_index,
            entry_index,
            alternate: is_alternate,
        })
        .into()
}

fn app_message<T>(
    f: impl FnOnce(T) -> Message + Send + 'static,
) -> impl FnOnce(T) -> cosmic::Action<Message> + Send + 'static {
    move |value| cosmic::action::app(f(value))
}

fn default_plugin_dir() -> PathBuf {
    if let Ok(value) = std::env::var("CBAR_PLUGIN_DIR") {
        return PathBuf::from(value);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        let local_plugins = current_dir.join("plugins");
        if local_plugins.is_dir() {
            return local_plugins;
        }
    }

    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/cbar/plugins");
    }

    PathBuf::from("plugins")
}

fn panel_label(plugins: &[PluginState], status: &str) -> String {
    let labels = plugins
        .iter()
        .filter_map(|plugin| {
            let title = plugin.title().trim();
            if title.is_empty() {
                None
            } else {
                Some(title.to_owned())
            }
        })
        .collect::<Vec<_>>();

    if labels.is_empty() {
        status.to_owned()
    } else {
        labels.join(" | ")
    }
}
