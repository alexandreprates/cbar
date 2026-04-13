use crate::config::{AppConfig, default_config_path, load_config, save_config};
use crate::fl;
use crate::parser::EmbeddedImage;
use crate::plugin::{PluginState, load_plugins, refresh_plugin_state, trigger_entry};
use cosmic::app::{Core, Task};
use cosmic::applet::menu_button;
use cosmic::iced::platform_specific::shell::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::time;
use cosmic::iced::widget::{Image, column, container, row, scrollable, svg};
use cosmic::iced::{Alignment, Length, Subscription, window};
use cosmic::theme;
use cosmic::widget::{
    Id, autosize, button, divider,
    icon::{self, Data as IconData, Handle as IconHandle},
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
    config: AppConfig,
    config_path: PathBuf,
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
    ReloadPlugins,
    PluginsLoaded(Vec<PluginState>),
    PluginRefreshed {
        index: usize,
        plugin: Box<PluginState>,
    },
    RefreshAll,
    TogglePluginSelection(String),
    ConfigSaved(Result<(), String>),
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
        let config_path = default_config_path();
        let config = load_config(&config_path).unwrap_or_default();

        (
            Self {
                core,
                popup: None,
                config,
                config_path,
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
            Message::ReloadPlugins => {
                self.status = fl!("reloading-plugins");
                return Task::perform(
                    load_plugins(self.plugin_dir.clone()),
                    app_message(Message::PluginsLoaded),
                );
            }
            Message::PluginsLoaded(plugins) => {
                self.plugins = plugins;
                self.sync_refresh_tracking();
                self.update_status();
            }
            Message::PluginRefreshed { index, plugin } => {
                if let Some(slot) = self.plugins.get_mut(index) {
                    *slot = *plugin;
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
                    return self.request_refresh_for_index(index, force);
                }
            }
            Message::RefreshAll => {
                return self.spawn_due_refreshes(true);
            }
            Message::TogglePluginSelection(plugin_name) => {
                let enabled = self.toggle_plugin_selection(&plugin_name);
                let refresh_task = self
                    .plugins
                    .iter()
                    .position(|plugin| plugin.name == plugin_name)
                    .map_or_else(Task::none, |index| {
                        if enabled {
                            self.request_refresh_for_index(index, true)
                        } else {
                            Task::none()
                        }
                    });
                let save_task = self.persist_config();
                return Task::batch(vec![refresh_task, save_task]);
            }
            Message::ConfigSaved(result) => {
                if let Err(err) = result {
                    self.status = err;
                } else {
                    self.update_status();
                }
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
        let suggested = self.core.applet.suggested_size(true);
        let (applet_padding_major_axis, applet_padding_minor_axis) =
            self.core.applet.suggested_padding(true);
        let (horizontal_padding, vertical_padding) = if self.core.applet.is_horizontal() {
            (applet_padding_major_axis, applet_padding_minor_axis)
        } else {
            (applet_padding_minor_axis, applet_padding_major_axis)
        };
        let button = button::custom(
            container(panel_content(&self.plugins, &self.config, &self.status))
                .center_y(Length::Fixed(f32::from(suggested.1 + 2 * vertical_padding))),
        )
        .on_press_down(Message::TogglePopup)
        .padding([0, horizontal_padding])
        .class(cosmic::theme::Button::AppletIcon);

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
        let enabled_count = self.enabled_plugin_count();

        let mut content = column![
            text::body(fl!(
                "plugin-dir",
                path = self.plugin_dir.display().to_string()
            )),
            menu_button(text::body(fl!("refresh-now"))).on_press(Message::RefreshAll),
            divider::horizontal::default(),
            text::body(
                fl!(
                    "visible-plugins",
                    enabled = enabled_count,
                    total = self.plugins.len()
                )
                .to_string()
            )
            .size(14),
            menu_button(text::body(fl!("reload-plugin-directory")))
                .on_press(Message::ReloadPlugins),
            divider::horizontal::default(),
        ]
        .spacing(spacing.space_xxs)
        .padding([8, 0])
        .align_x(Alignment::Start);

        if self.plugins.is_empty() {
            content = content.push(text::body(fl!("no-executable-plugins")));
        }

        for plugin in &self.plugins {
            let marker = if self.config.is_enabled(&plugin.name) {
                "✓"
            } else {
                "○"
            };
            content = content.push(
                menu_button(text::body(format!("{marker} {}", plugin.name)))
                    .on_press(Message::TogglePluginSelection(plugin.name.clone())),
            );
        }

        if !self.plugins.is_empty() {
            content = content.push(divider::horizontal::default());
        }

        let mut has_visible_plugins = false;
        for (plugin_index, plugin) in self.plugins.iter().enumerate() {
            if !self.config.is_enabled(&plugin.name) {
                continue;
            }

            has_visible_plugins = true;
            content = content.push(text::body(plugin.title().to_owned()).size(14));

            if plugin.cycle_items().len() > 1 {
                content = content.push(text::body(
                    fl!("cycle-items", items = plugin.cycle_items().join(" | ")).to_string(),
                ));
            }

            if let Some(error) = &plugin.last_error {
                content = content.push(text::body(fl!("plugin-error", error = error)));
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

        if !self.plugins.is_empty() && !has_visible_plugins {
            content = content.push(text::body(fl!("no-selected-plugins")));
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
            if !self.config.is_enabled(&self.plugins[index].name) {
                continue;
            }

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
            app_message(move |plugin| Message::PluginRefreshed {
                index,
                plugin: Box::new(plugin),
            }),
        )
    }

    fn request_refresh_for_index(&mut self, index: usize, force: bool) -> Task<Message> {
        if self.refresh_in_flight.get(index).copied().unwrap_or(false) {
            if force {
                if let Some(flag) = self.pending_force_refresh.get_mut(index) {
                    *flag = true;
                }
            } else if let Some(flag) = self.pending_refresh.get_mut(index) {
                *flag = true;
            }
            return Task::none();
        }

        self.spawn_refresh_for_index(index, force)
    }

    fn sync_refresh_tracking(&mut self) {
        let count = self.plugins.len();
        self.refresh_in_flight = vec![false; count];
        self.pending_refresh = vec![false; count];
        self.pending_force_refresh = vec![false; count];
    }

    fn update_status(&mut self) {
        self.status = if self.plugins.is_empty() {
            fl!(
                "status-no-plugins",
                path = self.plugin_dir.display().to_string()
            )
        } else if self.enabled_plugin_count() == 0 {
            fl!("status-no-selected-plugins")
        } else if self.enabled_plugin_count() == self.plugins.len() {
            fl!("status-plugin-count", count = self.plugins.len())
        } else {
            fl!(
                "status-plugin-count-filtered",
                enabled = self.enabled_plugin_count(),
                total = self.plugins.len()
            )
        };
    }

    fn enabled_plugin_count(&self) -> usize {
        self.plugins
            .iter()
            .filter(|plugin| self.config.is_enabled(&plugin.name))
            .count()
    }

    fn persist_config(&self) -> Task<Message> {
        let config_path = self.config_path.clone();
        let config = self.config.clone();
        Task::perform(
            async move { save_config(config_path, config) },
            app_message(Message::ConfigSaved),
        )
    }

    fn toggle_plugin_selection(&mut self, plugin_name: &str) -> bool {
        let mut enabled_plugins = self.config.enabled_plugins.clone().unwrap_or_else(|| {
            self.plugins
                .iter()
                .map(|plugin| plugin.name.clone())
                .collect()
        });

        if !enabled_plugins.remove(plugin_name) {
            enabled_plugins.insert(plugin_name.to_owned());
        }

        self.config.enabled_plugins = Some(enabled_plugins);
        let enabled = self.config.is_enabled(plugin_name);
        self.update_status();
        enabled
    }
}

fn render_entry(
    plugin_index: usize,
    entry_index: usize,
    entry: &crate::parser::MenuEntry,
    is_alternate: bool,
) -> Element<'static, Message> {
    let prefix = if is_alternate {
        &fl!("alternate-prefix")
    } else {
        ""
    };
    let mut label = format!("{prefix}{}", entry.text);

    if entry.params.href.is_some() {
        label.push_str(" ↗");
    }

    let left_padding = 12 + (entry.level as u16 * 16) + if is_alternate { 12 } else { 0 };
    let mut entry_row = row![]
        .spacing(8)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    if let Some(image) = entry
        .params
        .image
        .as_ref()
        .and_then(|image| image_element(image, 18))
    {
        entry_row = entry_row.push(image);
    }

    entry_row = entry_row.push(text::body(label).width(Length::Fill));

    let content = widget::container(entry_row)
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

fn panel_label(plugins: &[PluginState], config: &AppConfig, status: &str) -> String {
    let labels = plugins
        .iter()
        .filter(|plugin| config.is_enabled(&plugin.name))
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

fn panel_content<'a>(
    plugins: &'a [PluginState],
    config: &'a AppConfig,
    status: &'a str,
) -> Element<'a, Message> {
    let label = panel_label(plugins, config, status);
    let mut content = row![]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Shrink);

    if let Some(image) = plugins
        .iter()
        .find(|plugin| config.is_enabled(&plugin.name) && plugin.title_image().is_some())
        .and_then(|plugin| plugin.title_image())
        .and_then(|image| image_element(image, 18))
    {
        content = content.push(image);
    }

    if !label.is_empty() {
        content = content.push(text::body(label));
    }

    content.into()
}

fn image_element(image: &EmbeddedImage, size: u16) -> Option<Element<'static, Message>> {
    let handle = if image.is_svg {
        IconHandle {
            symbolic: image.is_template,
            data: IconData::Svg(svg::Handle::from_memory(image.bytes.clone())),
        }
    } else {
        let mut handle = icon::from_raster_bytes(image.bytes.clone());
        handle.symbolic = image.is_template;
        handle
    };

    Some(match handle.data {
        IconData::Svg(svg_handle) => icon::icon(IconHandle {
            symbolic: handle.symbolic,
            data: IconData::Svg(svg_handle),
        })
        .size(size)
        .into(),
        IconData::Image(image_handle) => Image::new(image_handle)
            .width(Length::Fixed(f32::from(size)))
            .height(Length::Fixed(f32::from(size)))
            .into(),
    })
}
