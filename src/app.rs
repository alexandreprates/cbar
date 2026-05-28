use crate::catalog::{CatalogPlugin, fetch_catalog, install_catalog_plugin, remove_catalog_plugin};
use crate::config::{AppConfig, default_config_path, load_config, save_config};
use crate::fl;
use crate::parser::EmbeddedImage;
use crate::plugin::{PluginState, load_plugins, refresh_plugin_state, trigger_entry};
use cosmic::app::{Core, Task};
use cosmic::applet::menu_button;
use cosmic::iced::platform_specific::shell::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::time;
use cosmic::iced::widget::{Image, column, container, row, scrollable, svg};
use cosmic::iced::{Alignment, ContentFit, Length, Limits, Subscription, window};
use cosmic::widget::{
    Id, autosize, button, divider,
    icon::{self, Data as IconData, Handle as IconHandle},
    rectangle_tracker::{RectangleTracker, RectangleUpdate, rectangle_tracker_subscription},
    text,
};
use cosmic::{Element, widget};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;
use std::time::{Duration, Instant};
use tokio::process::Command;

const APP_ID: &str = "io.github.alexprates.CBar";
const REPOSITORY_URL: &str = "https://github.com/alexandreprates/cbar";
const PANEL_IMAGE_HEIGHT: u16 = 22;
const PLUGIN_MENU_POPUP_WIDTH: f32 = 280.0;
const PLUGIN_MENU_POPUP_MIN_WIDTH: f32 = 180.0;
const MENU_POPUP_WIDTH: f32 = 420.0;
const MENU_POPUP_MIN_WIDTH: f32 = 360.0;
const CATALOG_POPUP_WIDTH: f32 = 520.0;
const CATALOG_POPUP_MAX_WIDTH: f32 = 560.0;
const POPUP_MAX_HEIGHT: f32 = 640.0;
const POSITIONER_MAX_WIDTH: f32 = 760.0;
const POSITIONER_MAX_HEIGHT: f32 = 1080.0;
static AUTOSIZE_MAIN_ID: LazyLock<Id> = LazyLock::new(|| Id::new("autosize-main"));

pub fn run() -> cosmic::iced::Result {
    cosmic::applet::run::<CBarApplet>(())
}

struct CBarApplet {
    core: Core,
    popup: Option<window::Id>,
    popup_plugin_index: Option<usize>,
    popup_view: PopupView,
    config: AppConfig,
    config_path: PathBuf,
    plugin_dir: PathBuf,
    plugins: Vec<PluginState>,
    status: String,
    rectangle_tracker: Option<RectangleTracker<u32>>,
    refresh_in_flight: Vec<bool>,
    pending_refresh: Vec<bool>,
    pending_force_refresh: Vec<bool>,
    catalog_plugins: Vec<CatalogPlugin>,
    catalog_loading: bool,
    catalog_installing: Option<String>,
    catalog_removing: Option<String>,
    catalog_status: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopupView {
    PluginMenu,
    Settings,
    Catalog,
}

#[derive(Debug, Clone)]
enum Message {
    TogglePopup {
        plugin_index: Option<usize>,
        popup_view: PopupView,
    },
    OpenBarSettings,
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
    OpenPluginCatalog,
    ReloadPluginCatalog,
    CatalogLoaded(Result<Vec<CatalogPlugin>, String>),
    InstallCatalogPlugin(String),
    CatalogPluginInstalled(Result<String, String>),
    RemoveCatalogPlugin(String),
    CatalogPluginRemoved(Result<String, String>),
    OpenPluginDirectory,
    PluginDirectoryOpened(Result<(), String>),
    OpenAbout,
    AboutOpened(Result<(), String>),
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
                popup_plugin_index: None,
                popup_view: PopupView::Settings,
                config,
                config_path,
                plugin_dir: plugin_dir.clone(),
                plugins: Vec::new(),
                status: "loading".to_owned(),
                rectangle_tracker: None,
                refresh_in_flight: Vec::new(),
                pending_refresh: Vec::new(),
                pending_force_refresh: Vec::new(),
                catalog_plugins: Vec::new(),
                catalog_loading: false,
                catalog_installing: None,
                catalog_removing: None,
                catalog_status: None,
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
            Message::TogglePopup {
                plugin_index,
                popup_view,
            } => {
                if self.popup.is_some()
                    && self.popup_plugin_index == plugin_index
                    && self.popup_view == popup_view
                {
                    self.popup_plugin_index = None;
                    self.popup_view = PopupView::Settings;
                    if let Some(popup) = self.popup.take() {
                        return destroy_popup(popup);
                    }
                }

                return self.open_popup_with_view(plugin_index, popup_view);
            }
            Message::OpenBarSettings => {
                self.popup_view = PopupView::Settings;
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.popup_plugin_index = None;
                    self.popup_view = PopupView::Settings;
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
                if self
                    .popup_plugin_index
                    .is_some_and(|index| index >= self.plugins.len())
                {
                    self.popup_plugin_index = None;
                }
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
            Message::OpenPluginCatalog => {
                self.popup_view = PopupView::Catalog;
                self.popup_plugin_index = None;

                if self.catalog_plugins.is_empty() && !self.catalog_loading {
                    self.catalog_loading = true;
                    self.catalog_status = Some(fl!("catalog-loading"));
                    return Task::perform(fetch_catalog(), app_message(Message::CatalogLoaded));
                }
            }
            Message::ReloadPluginCatalog => {
                if self.catalog_action_in_progress() {
                    self.catalog_status = Some(fl!("catalog-action-in-progress").to_string());
                    return Task::none();
                }

                self.catalog_loading = true;
                self.catalog_status = Some(fl!("catalog-loading"));
                return Task::perform(fetch_catalog(), app_message(Message::CatalogLoaded));
            }
            Message::CatalogLoaded(result) => {
                self.catalog_loading = false;
                match result {
                    Ok(plugins) => {
                        let count = plugins.len();
                        self.catalog_plugins = plugins;
                        self.catalog_status =
                            Some(fl!("catalog-loaded", count = count).to_string());
                    }
                    Err(err) => {
                        self.catalog_status = Some(err);
                    }
                }
            }
            Message::InstallCatalogPlugin(plugin_id) => {
                if self.catalog_action_in_progress() {
                    self.catalog_status = Some(fl!("catalog-action-in-progress").to_string());
                    return Task::none();
                }

                let Some(plugin) = self
                    .catalog_plugins
                    .iter()
                    .find(|plugin| plugin.id == plugin_id)
                    .cloned()
                else {
                    self.catalog_status = Some(fl!("catalog-plugin-not-found").to_string());
                    return Task::none();
                };

                self.catalog_installing = Some(plugin.id.clone());
                self.catalog_status =
                    Some(fl!("catalog-installing", name = plugin.name.clone()).to_string());
                return Task::perform(
                    install_catalog_plugin(self.plugin_dir.clone(), plugin),
                    app_message(Message::CatalogPluginInstalled),
                );
            }
            Message::CatalogPluginInstalled(result) => {
                self.catalog_installing = None;

                match result {
                    Ok(plugin_name) => {
                        if let Some(enabled_plugins) = self.config.enabled_plugins.as_mut() {
                            enabled_plugins.insert(plugin_name.clone());
                        }

                        self.catalog_status =
                            Some(fl!("catalog-installed", name = plugin_name).to_string());
                        let reload_task = Task::perform(
                            load_plugins(self.plugin_dir.clone()),
                            app_message(Message::PluginsLoaded),
                        );
                        return Task::batch(vec![reload_task, self.persist_config()]);
                    }
                    Err(err) => {
                        self.catalog_status = Some(err);
                    }
                }
            }
            Message::RemoveCatalogPlugin(plugin_id) => {
                if self.catalog_action_in_progress() {
                    self.catalog_status = Some(fl!("catalog-action-in-progress").to_string());
                    return Task::none();
                }

                let Some(plugin) = self
                    .catalog_plugins
                    .iter()
                    .find(|plugin| plugin.id == plugin_id)
                    .cloned()
                else {
                    self.catalog_status = Some(fl!("catalog-plugin-not-found").to_string());
                    return Task::none();
                };

                self.catalog_removing = Some(plugin.id.clone());
                self.catalog_status =
                    Some(fl!("catalog-removing", name = plugin.name.clone()).to_string());
                return Task::perform(
                    remove_catalog_plugin(self.plugin_dir.clone(), plugin),
                    app_message(Message::CatalogPluginRemoved),
                );
            }
            Message::CatalogPluginRemoved(result) => {
                self.catalog_removing = None;

                match result {
                    Ok(plugin_name) => {
                        let mut config_changed = false;
                        if let Some(enabled_plugins) = self.config.enabled_plugins.as_mut() {
                            config_changed = enabled_plugins.remove(&plugin_name);
                        }

                        self.catalog_status =
                            Some(fl!("catalog-removed", name = plugin_name).to_string());
                        let reload_task = Task::perform(
                            load_plugins(self.plugin_dir.clone()),
                            app_message(Message::PluginsLoaded),
                        );
                        let save_task = if config_changed {
                            self.persist_config()
                        } else {
                            Task::none()
                        };
                        return Task::batch(vec![reload_task, save_task]);
                    }
                    Err(err) => {
                        self.catalog_status = Some(err);
                    }
                }
            }
            Message::OpenPluginDirectory => {
                return Task::perform(
                    open_path(self.plugin_dir.clone()),
                    app_message(Message::PluginDirectoryOpened),
                );
            }
            Message::PluginDirectoryOpened(result) => {
                if let Err(err) = result {
                    self.status = err;
                }
            }
            Message::OpenAbout => {
                return Task::perform(open_repository(), app_message(Message::AboutOpened));
            }
            Message::AboutOpened(result) => {
                if let Err(err) = result {
                    self.status = err;
                }
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
        let button_height = Length::Fixed(f32::from(suggested.1 + 2 * vertical_padding));
        let enabled_plugins = self
            .plugins
            .iter()
            .enumerate()
            .filter(|(_, plugin)| self.config.is_enabled(&plugin.name))
            .collect::<Vec<_>>();

        let content = if enabled_plugins.is_empty() {
            let button =
                button::custom(container(cbar_panel_icon_content()).center_y(button_height))
                    .on_press_down(Message::TogglePopup {
                        plugin_index: None,
                        popup_view: PopupView::Settings,
                    })
                    .padding([0, horizontal_padding])
                    .class(cosmic::theme::Button::AppletIcon);

            Element::from(
                widget::mouse_area(button).on_right_press(Message::TogglePopup {
                    plugin_index: None,
                    popup_view: PopupView::Settings,
                }),
            )
        } else {
            let plugin_padding = if enabled_plugins.len() > 1 {
                horizontal_padding / 2
            } else {
                horizontal_padding
            };
            let mut buttons = row![]
                .spacing(0)
                .align_y(Alignment::Center)
                .width(Length::Shrink);

            for (plugin_index, plugin) in enabled_plugins {
                let button =
                    button::custom(container(plugin_panel_content(plugin)).center_y(button_height))
                        .on_press_down(Message::TogglePopup {
                            plugin_index: Some(plugin_index),
                            popup_view: PopupView::PluginMenu,
                        })
                        .padding([0, plugin_padding])
                        .class(cosmic::theme::Button::AppletIcon);

                buttons = buttons.push(widget::mouse_area(button).on_right_press(
                    Message::TogglePopup {
                        plugin_index: Some(plugin_index),
                        popup_view: PopupView::Settings,
                    },
                ));
            }

            buttons.into()
        };

        autosize::autosize(
            if let Some(tracker) = self.rectangle_tracker.as_ref() {
                Element::from(tracker.container(0, content).ignore_bounds(true))
            } else {
                content
            },
            AUTOSIZE_MAIN_ID.clone(),
        )
        .into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        if !matches!(self.popup, Some(popup) if popup == id) {
            return widget::text("").into();
        }

        let mut content = column![]
            .spacing(2)
            .padding([4, 0])
            .align_x(Alignment::Start);

        match self.popup_view {
            PopupView::PluginMenu => {
                if let Some(plugin_index) = self.popup_plugin_index {
                    if let Some(plugin) = self.plugins.get(plugin_index) {
                        content = push_plugin_menu(content, plugin_index, plugin);
                    } else {
                        content =
                            content.push(popup_label(fl!("no-selected-plugins").to_string(), 14));
                    }
                } else if self.plugins.is_empty() {
                    content = content.push(popup_label(self.status.clone(), 14));
                } else {
                    let mut has_visible_plugins = false;
                    for (plugin_index, plugin) in self.plugins.iter().enumerate() {
                        if !self.config.is_enabled(&plugin.name) {
                            continue;
                        }

                        has_visible_plugins = true;
                        content = push_plugin_menu(content, plugin_index, plugin)
                            .push(divider::horizontal::default());
                    }

                    if !self.plugins.is_empty() && !has_visible_plugins {
                        content =
                            content.push(popup_label(fl!("no-selected-plugins").to_string(), 14));
                    }
                }
            }
            PopupView::Settings => {
                content = build_bar_settings_view(content, &self.plugins, &self.config);
            }
            PopupView::Catalog => {
                content = build_catalog_view(
                    content,
                    &self.catalog_plugins,
                    &self.plugin_dir,
                    self.catalog_loading,
                    self.catalog_installing.as_deref(),
                    self.catalog_removing.as_deref(),
                    self.catalog_status.as_deref(),
                );
                let limits = Limits::NONE
                    .min_width(MENU_POPUP_MIN_WIDTH)
                    .max_width(CATALOG_POPUP_MAX_WIDTH)
                    .min_height(200.0)
                    .max_height(POPUP_MAX_HEIGHT);

                return self
                    .core
                    .applet
                    .popup_container(
                        container(scrollable(content)).width(Length::Fixed(CATALOG_POPUP_WIDTH)),
                    )
                    .limits(limits)
                    .into();
            }
        }

        let (popup_width, popup_min_width) = match self.popup_view {
            PopupView::PluginMenu => (PLUGIN_MENU_POPUP_WIDTH, PLUGIN_MENU_POPUP_MIN_WIDTH),
            PopupView::Settings | PopupView::Catalog => (MENU_POPUP_WIDTH, MENU_POPUP_MIN_WIDTH),
        };

        let limits = Limits::NONE
            .min_width(popup_min_width)
            .max_width(popup_width)
            .min_height(1.0)
            .max_height(POPUP_MAX_HEIGHT);

        self.core
            .applet
            .popup_container(container(scrollable(content)).width(Length::Fixed(popup_width)))
            .limits(limits)
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
    fn open_popup_with_view(
        &mut self,
        plugin_index: Option<usize>,
        popup_view: PopupView,
    ) -> Task<Message> {
        let previous_popup = self.popup.take();
        let new_id = window::Id::unique();
        self.popup = Some(new_id);
        self.popup_plugin_index = plugin_index;
        self.popup_view = popup_view;

        let popup_settings = self.core.applet.get_popup_settings(
            self.core.main_window_id().unwrap(),
            new_id,
            None,
            None,
            None,
        );
        let mut popup_settings = popup_settings;
        popup_settings.positioner.size_limits = Limits::NONE
            .min_height(1.0)
            .min_width(MENU_POPUP_MIN_WIDTH)
            .max_width(POSITIONER_MAX_WIDTH)
            .max_height(POSITIONER_MAX_HEIGHT);

        let mut tasks = Vec::new();
        if let Some(popup) = previous_popup {
            tasks.push(destroy_popup(popup));
        }
        tasks.push(get_popup(popup_settings));

        Task::batch(tasks)
    }

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

    fn catalog_action_in_progress(&self) -> bool {
        self.catalog_loading || self.catalog_installing.is_some() || self.catalog_removing.is_some()
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

    let left_padding = 20 + (entry.level as u16 * 16) + if is_alternate { 12 } else { 0 };
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
        .padding([2, 0])
        .on_press(Message::RunEntry {
            plugin_index,
            entry_index,
            alternate: is_alternate,
        })
        .into()
}

fn push_plugin_menu<'a>(
    mut content: cosmic::widget::Column<'a, Message, cosmic::Theme>,
    plugin_index: usize,
    plugin: &'a PluginState,
) -> cosmic::widget::Column<'a, Message, cosmic::Theme> {
    let mut has_intro = false;

    if plugin.cycle_items().len() > 1 {
        content = content.push(popup_label(
            fl!("cycle-items", items = plugin.cycle_items().join(" | ")).to_string(),
            14,
        ));
        has_intro = true;
    }

    if let Some(error) = &plugin.last_error {
        content = content.push(popup_label(
            fl!("plugin-error", error = error).to_string(),
            14,
        ));
        has_intro = true;
    }

    if has_intro {
        content = content.push(divider::horizontal::default());
    }

    for (entry_index, entry) in plugin.menu_entries().iter().enumerate() {
        if entry.separator {
            content = content.push(divider::horizontal::default());
            continue;
        }

        content = content.push(render_entry(plugin_index, entry_index, entry, false));

        if let Some(alternate) = entry.alternate.as_ref() {
            content = content.push(render_entry(plugin_index, entry_index, alternate, true));
        }
    }

    content
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

async fn open_repository() -> Result<(), String> {
    open_path(PathBuf::from(REPOSITORY_URL)).await
}

async fn open_path(path: PathBuf) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(&path)
        .spawn()
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    Ok(())
}

fn plugin_panel_label(plugin: &PluginState) -> String {
    let title = plugin.panel_title().trim();
    if !title.is_empty() {
        title.to_owned()
    } else if plugin.title_image().is_some() {
        String::new()
    } else {
        plugin.name.clone()
    }
}

fn plugin_panel_content(plugin: &PluginState) -> Element<'_, Message> {
    let label = plugin_panel_label(plugin);
    let mut content = row![]
        .spacing(6)
        .align_y(Alignment::Center)
        .width(Length::Shrink);

    if plugin.last_error.is_some() {
        content = content.push(warning_panel_icon());
    } else if let Some(image) = plugin
        .title_image()
        .and_then(|image| panel_image_element(image, PANEL_IMAGE_HEIGHT))
    {
        content = content.push(image);
    }

    if plugin.last_error.is_none() && !label.is_empty() {
        content = content.push(text::body(label));
    }

    content.into()
}

fn cbar_panel_icon_content() -> Element<'static, Message> {
    row![
        icon::icon(
            icon::from_svg_bytes(
                &include_bytes!(
                    "../data/icons/scalable/apps/io.github.alexprates.CBar-symbolic.svg"
                )[..]
            )
            .symbolic(true)
        )
        .size(PANEL_IMAGE_HEIGHT)
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .width(Length::Shrink)
    .into()
}

fn warning_panel_icon() -> Element<'static, Message> {
    icon::icon(icon::from_svg_bytes(
        &include_bytes!("../data/icons/scalable/apps/io.github.alexprates.CBar-warning.svg")[..],
    ))
    .size(PANEL_IMAGE_HEIGHT)
    .into()
}

fn popup_label(label: impl Into<String>, size: u16) -> Element<'static, Message> {
    widget::container(text::body(label.into()).size(size))
        .padding([0, 0, 0, 16])
        .width(Length::Fill)
        .into()
}

fn build_bar_settings_view<'a>(
    mut content: cosmic::widget::Column<'a, Message, cosmic::Theme>,
    plugins: &'a [PluginState],
    config: &'a AppConfig,
) -> cosmic::widget::Column<'a, Message, cosmic::Theme> {
    let enabled_count = plugins
        .iter()
        .filter(|plugin| config.is_enabled(&plugin.name))
        .count();

    content = content
        .push(featured_menu_button(
            fl!("explore-plugin-catalog"),
            Message::OpenPluginCatalog,
        ))
        .push(divider::horizontal::default())
        .push(
            widget::container(text::body(fl!("bar-settings")).size(24))
                .padding([4, 16, 0, 16])
                .width(Length::Fill),
        )
        .push(
            widget::container(
                row![
                    text::body(fl!("installed-plugins"))
                        .size(13)
                        .width(Length::Fill),
                    settings_summary_pill(fl!(
                        "visible-plugins-summary",
                        enabled = enabled_count,
                        total = plugins.len()
                    ))
                ]
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
            .padding([2, 16, 8, 16])
            .width(Length::Fill),
        );

    if plugins.is_empty() {
        content = content
            .push(widget::container(text::body(fl!("no-executable-plugins"))).padding([8, 16]));
    } else {
        for plugin in plugins {
            content = content.push(settings_plugin_card(
                plugin,
                config.is_enabled(&plugin.name),
            ));
        }
    }

    content = content
        .push(divider::horizontal::default())
        .push(indented_menu_button(
            fl!("open-plugin-directory"),
            Message::OpenPluginDirectory,
        ))
        .push(indented_menu_button(
            fl!("reload-plugin-directory"),
            Message::ReloadPlugins,
        ));

    content
        .push(divider::horizontal::default())
        .push(indented_menu_button(
            fl!("refresh-now"),
            Message::RefreshAll,
        ))
        .push(indented_menu_button(fl!("about"), Message::OpenAbout))
}

fn settings_plugin_card<'a>(plugin: &'a PluginState, enabled: bool) -> Element<'a, Message> {
    let mut plugin_badges = row![catalog_badge(settings_plugin_category(&plugin.name))]
        .spacing(4)
        .align_y(Alignment::Center);

    if let Some(interval) = plugin_interval_label(&plugin.name) {
        plugin_badges = plugin_badges.push(catalog_badge(interval));
    }

    let plugin_details = column![text::body(plugin.name.clone()).size(14), plugin_badges]
        .spacing(6)
        .width(Length::Fill);

    let plugin_name = plugin.name.clone();
    let toggle: Element<'a, Message> = widget::toggler(enabled)
        .size(24)
        .on_toggle(move |_| Message::TogglePluginSelection(plugin_name.clone()))
        .into();

    let state_label = if enabled {
        fl!("plugin-toggle-visible")
    } else {
        fl!("plugin-toggle-hidden")
    };

    let state = column![text::body(state_label).size(12), toggle]
        .spacing(6)
        .align_x(Alignment::End)
        .width(Length::Shrink);

    let card = row![settings_monogram(&plugin.name), plugin_details, state]
        .spacing(14)
        .align_y(Alignment::Center)
        .width(Length::Fill);

    widget::container(
        widget::container(card)
            .class(catalog_card_container())
            .padding(12)
            .width(Length::Fill),
    )
    .padding([5, 14])
    .width(Length::Fill)
    .into()
}

fn settings_summary_pill<'a>(label: impl Into<String>) -> Element<'a, Message> {
    widget::container(text::body(label.into()).size(12))
        .padding([2, 8])
        .class(catalog_badge_container())
        .into()
}

fn settings_monogram<'a>(name: &str) -> Element<'a, Message> {
    widget::container(text::body(plugin_monogram(name)).size(15))
        .class(catalog_monogram_container())
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(44.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

fn settings_plugin_category(name: &str) -> String {
    let lower_name = name.to_ascii_lowercase();
    if lower_name.starts_with("showcase-") {
        "showcase".to_owned()
    } else if lower_name.contains("github")
        || lower_name.contains("openai")
        || lower_name.contains("docker")
    {
        "dev".to_owned()
    } else {
        "local".to_owned()
    }
}

fn build_catalog_view<'a>(
    mut content: cosmic::widget::Column<'a, Message, cosmic::Theme>,
    catalog_plugins: &'a [CatalogPlugin],
    plugin_dir: &'a Path,
    loading: bool,
    installing: Option<&'a str>,
    removing: Option<&'a str>,
    status: Option<&'a str>,
) -> cosmic::widget::Column<'a, Message, cosmic::Theme> {
    let catalog_busy = loading || installing.is_some() || removing.is_some();
    let reload_label = if loading {
        fl!("catalog-loading")
    } else if catalog_busy {
        fl!("catalog-action-in-progress")
    } else {
        fl!("reload-plugin-catalog")
    };

    content = content
        .push(indented_menu_button(fl!("back"), Message::OpenBarSettings))
        .push(
            widget::container(text::body(fl!("plugin-catalog")).size(24))
                .padding([2, 16, 0, 16])
                .width(Length::Fill),
        )
        .push(
            widget::container(text::body(fl!("catalog-security-note")).size(13))
                .padding([4, 16, 12, 16])
                .width(Length::Fill),
        )
        .push(divider::horizontal::default());

    let catalog_status = status
        .map(str::to_owned)
        .unwrap_or_else(|| fl!("catalog-loaded", count = catalog_plugins.len()).to_string());
    let catalog_toolbar = if catalog_busy {
        row![
            text::body(catalog_status).size(13).width(Length::Fill),
            text::body(reload_label).size(13)
        ]
    } else {
        let reload_action: Element<'_, Message> = button::suggested(reload_label)
            .on_press(Message::ReloadPluginCatalog)
            .into();
        row![
            text::body(catalog_status).size(13).width(Length::Fill),
            reload_action
        ]
    }
    .spacing(12)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    content = content.push(
        widget::container(catalog_toolbar)
            .padding([12, 16])
            .width(Length::Fill),
    );

    if catalog_plugins.is_empty() && !loading {
        return content.push(
            widget::container(text::body(fl!("catalog-empty")))
                .padding([0, 16])
                .width(Length::Fill),
        );
    }

    for plugin in catalog_plugins {
        let installed = plugin
            .installed_path(plugin_dir)
            .is_ok_and(|path| path.exists());
        let is_installing = installing.is_some_and(|plugin_id| plugin_id == plugin.id);
        let is_removing = removing.is_some_and(|plugin_id| plugin_id == plugin.id);

        content = content.push(catalog_plugin_card(
            plugin,
            installed,
            is_installing,
            is_removing,
            catalog_busy,
        ));
    }

    content
}

fn catalog_plugin_card<'a>(
    plugin: &'a CatalogPlugin,
    installed: bool,
    installing: bool,
    removing: bool,
    catalog_busy: bool,
) -> Element<'a, Message> {
    let mut plugin_details = column![text::body(plugin.name.clone()).size(14)]
        .spacing(4)
        .width(Length::Fill);

    if let Some(publisher) = &plugin.publisher {
        plugin_details = plugin_details
            .push(text::body(fl!("catalog-published-by", publisher = publisher.clone())).size(12));
    }

    plugin_details = plugin_details
        .push(text::body(plugin.description.clone()).size(13))
        .push(catalog_metadata_badges(plugin));

    let card = row![
        catalog_monogram(&plugin.name),
        plugin_details,
        catalog_install_toggle(plugin, installed, installing, removing, catalog_busy)
    ]
    .spacing(14)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    widget::container(
        widget::container(card)
            .class(catalog_card_container())
            .padding(16)
            .width(Length::Fill),
    )
    .padding([7, 14])
    .width(Length::Fill)
    .into()
}

fn catalog_monogram<'a>(name: &str) -> Element<'a, Message> {
    widget::container(text::body(plugin_monogram(name)).size(18))
        .class(catalog_monogram_container())
        .width(Length::Fixed(52.0))
        .height(Length::Fixed(52.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .into()
}

fn plugin_monogram(name: &str) -> String {
    let words = plugin_name_parts(name);

    if words.len() == 1 {
        return words[0]
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(3)
            .collect::<String>()
            .to_ascii_uppercase();
    }

    let acronym = words
        .iter()
        .filter_map(|word| word.chars().find(|ch| ch.is_ascii_alphanumeric()))
        .take(3)
        .collect::<String>()
        .to_ascii_uppercase();

    if acronym.is_empty() {
        "?".to_owned()
    } else {
        acronym
    }
}

fn plugin_name_parts(name: &str) -> Vec<String> {
    let normalized = name.strip_suffix(".sh").unwrap_or(name);

    normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .filter(|part| !looks_like_interval(part))
        .map(ToOwned::to_owned)
        .collect()
}

fn plugin_interval_label(name: &str) -> Option<String> {
    let normalized = name.strip_suffix(".sh").unwrap_or(name);

    normalized
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .find(|part| looks_like_interval(part))
        .map(ToOwned::to_owned)
}

fn looks_like_interval(value: &str) -> bool {
    let Some(unit) = value.chars().last() else {
        return false;
    };

    let number = &value[..value.len().saturating_sub(1)];
    matches!(unit, 's' | 'm' | 'h' | 'd')
        && !number.is_empty()
        && number.chars().all(|ch| ch.is_ascii_digit())
}

fn catalog_metadata_badges<'a>(plugin: &'a CatalogPlugin) -> Element<'a, Message> {
    let mut badges = row![catalog_badge(plugin.category.clone())]
        .spacing(4)
        .align_y(Alignment::Center);

    for language in &plugin.languages {
        badges = badges.push(catalog_badge(language.clone()));
    }

    badges = badges
        .push(catalog_badge(plugin.interval.clone()))
        .push(catalog_badge(plugin.language.clone()));

    badges.into()
}

fn catalog_badge<'a>(label: impl Into<String>) -> Element<'a, Message> {
    widget::container(text::body(label.into()).size(11))
        .padding([1, 7])
        .class(catalog_badge_container())
        .into()
}

fn catalog_card_container<'a>() -> cosmic::theme::Container<'a> {
    cosmic::theme::Container::custom(|_theme| cosmic::iced::widget::container::Style {
        background: Some(cosmic::iced::Background::Color(
            cosmic::iced::Color::from_rgb8(43, 43, 43),
        )),
        border: cosmic::iced::Border {
            color: cosmic::iced::Color::from_rgba8(255, 255, 255, 0.08),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
}

fn catalog_monogram_container<'a>() -> cosmic::theme::Container<'a> {
    cosmic::theme::Container::custom(|_theme| cosmic::iced::widget::container::Style {
        text_color: Some(cosmic::iced::Color::from_rgb8(174, 208, 255)),
        background: Some(cosmic::iced::Background::Color(
            cosmic::iced::Color::from_rgb8(32, 40, 50),
        )),
        border: cosmic::iced::Border {
            color: cosmic::iced::Color::from_rgb8(89, 103, 122),
            width: 2.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    })
}

fn catalog_badge_container<'a>() -> cosmic::theme::Container<'a> {
    cosmic::theme::Container::custom(|_theme| cosmic::iced::widget::container::Style {
        text_color: Some(cosmic::iced::Color::from_rgb8(219, 234, 254)),
        background: Some(cosmic::iced::Background::Color(
            cosmic::iced::Color::from_rgb8(58, 69, 83),
        )),
        border: cosmic::iced::Border {
            radius: 999.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn catalog_install_toggle<'a>(
    plugin: &'a CatalogPlugin,
    installed: bool,
    installing: bool,
    removing: bool,
    catalog_busy: bool,
) -> Element<'a, Message> {
    let toggled = if installing {
        true
    } else if removing {
        false
    } else {
        installed
    };

    let label = if installing {
        fl!("catalog-toggle-installing")
    } else if removing {
        fl!("catalog-toggle-removing")
    } else if installed {
        fl!("catalog-toggle-installed")
    } else {
        fl!("catalog-toggle-available")
    };

    let toggler: Element<'a, Message> = if catalog_busy {
        widget::toggler(toggled).size(24).into()
    } else if installed {
        let plugin_id = plugin.id.clone();
        widget::toggler(true)
            .size(24)
            .on_toggle(move |_| Message::RemoveCatalogPlugin(plugin_id.clone()))
            .into()
    } else {
        let plugin_id = plugin.id.clone();
        widget::toggler(false)
            .size(24)
            .on_toggle(move |_| Message::InstallCatalogPlugin(plugin_id.clone()))
            .into()
    };

    column![text::body(label).size(12), toggler]
        .spacing(6)
        .align_x(Alignment::End)
        .width(Length::Shrink)
        .into()
}

fn indented_menu_button<'a>(label: impl Into<String>, message: Message) -> Element<'a, Message> {
    menu_button(
        widget::container(text::body(label.into()))
            .padding([0, 0, 0, 16])
            .width(Length::Fill),
    )
    .padding([2, 0])
    .on_press(message)
    .into()
}

fn featured_menu_button<'a>(label: impl Into<String>, message: Message) -> Element<'a, Message> {
    menu_button(
        widget::container(text::body(label.into()))
            .class(featured_action_container())
            .padding([9, 18])
            .width(Length::Fill),
    )
    .padding([14, 18])
    .on_press(message)
    .into()
}

fn featured_action_container<'a>() -> cosmic::theme::Container<'a> {
    cosmic::theme::Container::custom(|_theme| cosmic::iced::widget::container::Style {
        text_color: Some(cosmic::iced::Color::from_rgb8(17, 21, 28)),
        background: Some(cosmic::iced::Background::Color(
            cosmic::iced::Color::from_rgb8(159, 197, 255),
        )),
        border: cosmic::iced::Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn panel_image_element(image: &EmbeddedImage, height: u16) -> Option<Element<'static, Message>> {
    Some(if image.is_svg {
        svg::Svg::new(svg::Handle::from_memory(image.bytes.clone()))
            .symbolic(image.is_template)
            .height(Length::Fixed(f32::from(height)))
            .width(Length::Shrink)
            .content_fit(ContentFit::Contain)
            .into()
    } else {
        let image_handle = match icon::from_raster_bytes(image.bytes.clone()).data {
            IconData::Image(handle) => handle,
            IconData::Svg(_) => return None,
        };
        Image::new(image_handle)
            .height(Length::Fixed(f32::from(height)))
            .width(Length::Shrink)
            .content_fit(ContentFit::Contain)
            .into()
    })
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
