// Copyright 2019-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! The Tauri plugin extension to expand Tauri functionality.

use crate::{
  utils::config::PluginConfig, AppHandle, Invoke, InvokeHandler, PageLoadPayload, RunEvent,
  Runtime, Window,
};
use serde::de::DeserializeOwned;
use serde_json::Value as JsonValue;
use tauri_macros::default_runtime;
use url::Url;

use std::{fmt, result::Result as StdResult, sync::Arc};

/// Mobile APIs.
#[cfg(mobile)]
pub mod mobile;

/// The result type of Tauri plugin module.
pub type Result<T> = StdResult<T, Box<dyn std::error::Error>>;

/// The plugin interface.
pub trait Plugin<R: Runtime>: Send {
  /// The plugin name. Used as key on the plugin config object.
  fn name(&self) -> &'static str;

  /// Initializes the plugin.
  #[allow(unused_variables)]
  fn initialize(&mut self, app: &AppHandle<R>, config: JsonValue) -> Result<()> {
    Ok(())
  }

  /// Add the provided JavaScript to a list of scripts that should be run after the global object has been created,
  /// but before the HTML document has been parsed and before any other script included by the HTML document is run.
  ///
  /// Since it runs on all top-level document and child frame page navigations,
  /// it's recommended to check the `window.location` to guard your script from running on unexpected origins.
  ///
  /// The script is wrapped into its own context with `(function () { /* your script here */ })();`,
  /// so global variables must be assigned to `window` instead of implicitly declared.
  fn initialization_script(&self) -> Option<String> {
    None
  }

  /// Callback invoked when the webview is created.
  #[allow(unused_variables)]
  fn created(&mut self, window: Window<R>) {}

  /// Callback invoked when webview tries to navigate to the given Url. Returning falses cancels navigation.
  #[allow(unused_variables)]
  fn on_navigation(&mut self, window: &Window<R>, url: &Url) -> bool {
    true
  }

  /// Callback invoked when the webview performs a navigation to a page.
  #[allow(unused_variables)]
  fn on_page_load(&mut self, window: Window<R>, payload: PageLoadPayload) {}

  /// Callback invoked when the event loop receives a new event.
  #[allow(unused_variables)]
  fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {}

  /// Extend commands to [`crate::Builder::invoke_handler`].
  #[allow(unused_variables)]
  fn extend_api(&mut self, invoke: Invoke<R>) -> bool {
    false
  }
}

type SetupHook<R, C> = dyn FnOnce(&AppHandle<R>, PluginApi<R, C>) -> Result<()> + Send;
type OnWebviewReady<R> = dyn FnMut(Window<R>) + Send;
type OnEvent<R> = dyn FnMut(&AppHandle<R>, &RunEvent) + Send;
type OnNavigation<R> = dyn Fn(&Window<R>, &Url) -> bool + Send;
type OnPageLoad<R> = dyn FnMut(Window<R>, PageLoadPayload) + Send;
type OnDrop<R> = dyn FnOnce(AppHandle<R>) + Send;

/// A handle to a plugin.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PluginHandle<R: Runtime> {
  name: &'static str,
  handle: AppHandle<R>,
}

impl<R: Runtime> Clone for PluginHandle<R> {
  fn clone(&self) -> Self {
    Self {
      name: self.name,
      handle: self.handle.clone(),
    }
  }
}

impl<R: Runtime> PluginHandle<R> {
  /// Returns the application handle.
  pub fn app(&self) -> &AppHandle<R> {
    &self.handle
  }
}

/// Api exposed to the plugin setup hook.
#[derive(Clone)]
#[allow(dead_code)]
pub struct PluginApi<R: Runtime, C: DeserializeOwned> {
  handle: AppHandle<R>,
  name: &'static str,
  raw_config: Arc<JsonValue>,
  config: C,
}

impl<R: Runtime, C: DeserializeOwned> PluginApi<R, C> {
  /// Returns the plugin configuration.
  pub fn config(&self) -> &C {
    &self.config
  }

  /// Returns the application handle.
  pub fn app(&self) -> &AppHandle<R> {
    &self.handle
  }
}

/// Builds a [`TauriPlugin`].
///
/// This Builder offers a more concise way to construct Tauri plugins than implementing the Plugin trait directly.
///
/// # Conventions
///
/// When using the Builder Pattern it is encouraged to export a function called `init` that constructs and returns the plugin.
/// While plugin authors can provide every possible way to construct a plugin,
/// sticking to the `init` function convention helps users to quickly identify the correct function to call.
///
/// ```rust
/// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
///
/// pub fn init<R: Runtime>() -> TauriPlugin<R> {
///   Builder::new("example")
///     .build()
/// }
/// ```
///
/// When plugins expose more complex configuration options, it can be helpful to provide a Builder instead:
///
/// ```rust
/// use tauri::{plugin::{Builder as PluginBuilder, TauriPlugin}, Runtime};
///
/// pub struct Builder {
///   option_a: String,
///   option_b: String,
///   option_c: bool
/// }
///
/// impl Default for Builder {
///   fn default() -> Self {
///     Self {
///       option_a: "foo".to_string(),
///       option_b: "bar".to_string(),
///       option_c: false
///     }
///   }
/// }
///
/// impl Builder {
///   pub fn new() -> Self {
///     Default::default()
///   }
///
///   pub fn option_a(mut self, option_a: String) -> Self {
///     self.option_a = option_a;
///     self
///   }
///
///   pub fn option_b(mut self, option_b: String) -> Self {
///     self.option_b = option_b;
///     self
///   }
///
///   pub fn option_c(mut self, option_c: bool) -> Self {
///     self.option_c = option_c;
///     self
///   }
///
///   pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
///     PluginBuilder::new("example")
///       .setup(move |app_handle, api| {
///         // use the options here to do stuff
///         println!("a: {}, b: {}, c: {}", self.option_a, self.option_b, self.option_c);
///
///         Ok(())
///       })
///       .build()
///   }
/// }
/// ```
pub struct Builder<R: Runtime, C: DeserializeOwned = ()> {
  name: &'static str,
  invoke_handler: Box<InvokeHandler<R>>,
  setup: Option<Box<SetupHook<R, C>>>,
  js_init_script: Option<String>,
  on_navigation: Box<OnNavigation<R>>,
  on_page_load: Box<OnPageLoad<R>>,
  on_webview_ready: Box<OnWebviewReady<R>>,
  on_event: Box<OnEvent<R>>,
  on_drop: Option<Box<OnDrop<R>>>,
}

impl<R: Runtime, C: DeserializeOwned> Builder<R, C> {
  /// Creates a new Plugin builder.
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      setup: None,
      js_init_script: None,
      invoke_handler: Box::new(|_| false),
      on_navigation: Box::new(|_, _| true),
      on_page_load: Box::new(|_, _| ()),
      on_webview_ready: Box::new(|_| ()),
      on_event: Box::new(|_, _| ()),
      on_drop: None,
    }
  }

  /// Defines the JS message handler callback.
  /// It is recommended you use the [tauri::generate_handler] to generate the input to this method, as the input type is not considered stable yet.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// #[tauri::command]
  /// async fn foobar<R: Runtime>(app: tauri::AppHandle<R>, window: tauri::Window<R>) -> Result<(), String> {
  ///   println!("foobar");
  ///
  ///   Ok(())
  /// }
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .invoke_handler(tauri::generate_handler![foobar])
  ///     .build()
  /// }
  ///
  /// ```
  /// [tauri::generate_handler]: ../macro.generate_handler.html
  #[must_use]
  pub fn invoke_handler<F>(mut self, invoke_handler: F) -> Self
  where
    F: Fn(Invoke<R>) -> bool + Send + Sync + 'static,
  {
    self.invoke_handler = Box::new(invoke_handler);
    self
  }

  /// Sets the provided JavaScript to be run after the global object has been created,
  /// but before the HTML document has been parsed and before any other script included by the HTML document is run.
  ///
  /// Since it runs on all top-level document and child frame page navigations,
  /// it's recommended to check the `window.location` to guard your script from running on unexpected origins.
  ///
  /// The script is wrapped into its own context with `(function () { /* your script here */ })();`,
  /// so global variables must be assigned to `window` instead of implicitly declared.
  ///
  /// Note that calling this function multiple times overrides previous values.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// const INIT_SCRIPT: &str = r#"
  ///   if (window.location.origin === 'https://tauri.app') {
  ///     console.log("hello world from js init script");
  ///
  ///     window.__MY_CUSTOM_PROPERTY__ = { foo: 'bar' };
  ///   }
  /// "#;
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .js_init_script(INIT_SCRIPT.to_string())
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn js_init_script(mut self, js_init_script: String) -> Self {
    self.js_init_script = Some(js_init_script);
    self
  }

  /// Define a closure that runs when the plugin is registered.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime, Manager};
  /// use std::path::PathBuf;
  ///
  /// #[derive(Debug, Default)]
  /// struct PluginState {
  ///    dir: Option<PathBuf>
  /// }
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  /// Builder::new("example")
  ///   .setup(|app, api| {
  ///     app.manage(PluginState::default());
  ///
  ///     Ok(())
  ///   })
  ///   .build()
  /// }
  /// ```
  #[must_use]
  pub fn setup<F>(mut self, setup: F) -> Self
  where
    F: FnOnce(&AppHandle<R>, PluginApi<R, C>) -> Result<()> + Send + 'static,
  {
    self.setup.replace(Box::new(setup));
    self
  }

  /// Callback invoked when the webview tries to navigate to a URL. Returning false cancels the navigation.
  ///
  /// #Example
  ///
  /// ```
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .on_navigation(|window, url| {
  ///       // allow the production URL or localhost on dev
  ///       url.scheme() == "tauri" || (cfg!(dev) && url.host_str() == Some("localhost"))
  ///     })
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn on_navigation<F>(mut self, on_navigation: F) -> Self
  where
    F: Fn(&Window<R>, &Url) -> bool + Send + 'static,
  {
    self.on_navigation = Box::new(on_navigation);
    self
  }

  /// Callback invoked when the webview performs a navigation to a page.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .on_page_load(|window, payload| {
  ///       println!("Loaded URL {} in window {}", payload.url(), window.label());
  ///     })
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn on_page_load<F>(mut self, on_page_load: F) -> Self
  where
    F: FnMut(Window<R>, PageLoadPayload) + Send + 'static,
  {
    self.on_page_load = Box::new(on_page_load);
    self
  }

  /// Callback invoked when the webview is created.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .on_webview_ready(|window| {
  ///       println!("created window {}", window.label());
  ///     })
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn on_webview_ready<F>(mut self, on_webview_ready: F) -> Self
  where
    F: FnMut(Window<R>) + Send + 'static,
  {
    self.on_webview_ready = Box::new(on_webview_ready);
    self
  }

  /// Callback invoked when the event loop receives a new event.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, RunEvent, Runtime};
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .on_event(|app_handle, event| {
  ///       match event {
  ///         RunEvent::ExitRequested { api, .. } => {
  ///           // Prevents the app from exiting.
  ///           // This will cause the core thread to continue running in the background even without any open windows.
  ///           api.prevent_exit();
  ///         }
  ///         // Ignore all other cases.
  ///         _ => {}
  ///       }
  ///     })
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn on_event<F>(mut self, on_event: F) -> Self
  where
    F: FnMut(&AppHandle<R>, &RunEvent) + Send + 'static,
  {
    self.on_event = Box::new(on_event);
    self
  }

  /// Callback invoked when the plugin is dropped.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use tauri::{plugin::{Builder, TauriPlugin}, Runtime};
  ///
  /// fn init<R: Runtime>() -> TauriPlugin<R> {
  ///   Builder::new("example")
  ///     .on_drop(|app| {
  ///       println!("plugin has been dropped and is no longer running");
  ///       // you can run cleanup logic here
  ///     })
  ///     .build()
  /// }
  /// ```
  #[must_use]
  pub fn on_drop<F>(mut self, on_drop: F) -> Self
  where
    F: FnOnce(AppHandle<R>) + Send + 'static,
  {
    self.on_drop.replace(Box::new(on_drop));
    self
  }

  /// Builds the [TauriPlugin].
  pub fn build(self) -> TauriPlugin<R, C> {
    TauriPlugin {
      name: self.name,
      app: None,
      invoke_handler: self.invoke_handler,
      setup: self.setup,
      js_init_script: self.js_init_script,
      on_navigation: self.on_navigation,
      on_page_load: self.on_page_load,
      on_webview_ready: self.on_webview_ready,
      on_event: self.on_event,
      on_drop: self.on_drop,
    }
  }
}

/// Plugin struct that is returned by the [`Builder`]. Should only be constructed through the builder.
pub struct TauriPlugin<R: Runtime, C: DeserializeOwned = ()> {
  name: &'static str,
  app: Option<AppHandle<R>>,
  invoke_handler: Box<InvokeHandler<R>>,
  setup: Option<Box<SetupHook<R, C>>>,
  js_init_script: Option<String>,
  on_navigation: Box<OnNavigation<R>>,
  on_page_load: Box<OnPageLoad<R>>,
  on_webview_ready: Box<OnWebviewReady<R>>,
  on_event: Box<OnEvent<R>>,
  on_drop: Option<Box<OnDrop<R>>>,
}

impl<R: Runtime, C: DeserializeOwned> Drop for TauriPlugin<R, C> {
  fn drop(&mut self) {
    if let (Some(on_drop), Some(app)) = (self.on_drop.take(), self.app.take()) {
      on_drop(app);
    }
  }
}

impl<R: Runtime, C: DeserializeOwned> Plugin<R> for TauriPlugin<R, C> {
  fn name(&self) -> &'static str {
    self.name
  }

  fn initialize(&mut self, app: &AppHandle<R>, config: JsonValue) -> Result<()> {
    self.app.replace(app.clone());
    if let Some(s) = self.setup.take() {
      (s)(
        app,
        PluginApi {
          name: self.name,
          handle: app.clone(),
          raw_config: Arc::new(config.clone()),
          config: serde_json::from_value(config)?,
        },
      )?;
    }
    Ok(())
  }

  fn initialization_script(&self) -> Option<String> {
    self.js_init_script.clone()
  }

  fn created(&mut self, window: Window<R>) {
    (self.on_webview_ready)(window)
  }

  fn on_navigation(&mut self, window: &Window<R>, url: &Url) -> bool {
    (self.on_navigation)(window, url)
  }

  fn on_page_load(&mut self, window: Window<R>, payload: PageLoadPayload) {
    (self.on_page_load)(window, payload)
  }

  fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
    (self.on_event)(app, event)
  }

  fn extend_api(&mut self, invoke: Invoke<R>) -> bool {
    (self.invoke_handler)(invoke)
  }
}

/// Plugin collection type.
#[default_runtime(crate::Wry, wry)]
pub(crate) struct PluginStore<R: Runtime> {
  store: Vec<Box<dyn Plugin<R>>>,
}

impl<R: Runtime> fmt::Debug for PluginStore<R> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let plugins: Vec<&str> = self.store.iter().map(|plugins| plugins.name()).collect();
    f.debug_struct("PluginStore")
      .field("plugins", &plugins)
      .finish()
  }
}

impl<R: Runtime> Default for PluginStore<R> {
  fn default() -> Self {
    Self { store: Vec::new() }
  }
}

impl<R: Runtime> PluginStore<R> {
  /// Adds a plugin to the store.
  ///
  /// Returns `true` if a plugin with the same name is already in the store.
  pub fn register<P: Plugin<R> + 'static>(&mut self, plugin: P) -> bool {
    let len = self.store.len();
    self.store.retain(|p| p.name() != plugin.name());
    let result = len != self.store.len();
    self.store.push(Box::new(plugin));
    result
  }

  /// Removes the plugin with the given name from the store.
  pub fn unregister(&mut self, plugin: &'static str) -> bool {
    let len = self.store.len();
    self.store.retain(|p| p.name() != plugin);
    len != self.store.len()
  }

  /// Initializes all plugins in the store.
  pub(crate) fn initialize(
    &mut self,
    app: &AppHandle<R>,
    config: &PluginConfig,
  ) -> crate::Result<()> {
    self.store.iter_mut().try_for_each(|plugin| {
      plugin
        .initialize(
          app,
          config.0.get(plugin.name()).cloned().unwrap_or_default(),
        )
        .map_err(|e| crate::Error::PluginInitialization(plugin.name().to_string(), e.to_string()))
    })
  }

  /// Generates an initialization script from all plugins in the store.
  pub(crate) fn initialization_script(&self) -> String {
    self
      .store
      .iter()
      .filter_map(|p| p.initialization_script())
      .fold(String::new(), |acc, script| {
        format!("{acc}\n(function () {{ {script} }})();")
      })
  }

  /// Runs the created hook for all plugins in the store.
  pub(crate) fn created(&mut self, window: Window<R>) {
    self
      .store
      .iter_mut()
      .for_each(|plugin| plugin.created(window.clone()))
  }

  pub(crate) fn on_navigation(&mut self, window: &Window<R>, url: &Url) -> bool {
    for plugin in self.store.iter_mut() {
      if !plugin.on_navigation(window, url) {
        return false;
      }
    }
    true
  }

  /// Runs the on_page_load hook for all plugins in the store.
  pub(crate) fn on_page_load(&mut self, window: Window<R>, payload: PageLoadPayload) {
    self
      .store
      .iter_mut()
      .for_each(|plugin| plugin.on_page_load(window.clone(), payload.clone()))
  }

  /// Runs the on_event hook for all plugins in the store.
  pub(crate) fn on_event(&mut self, app: &AppHandle<R>, event: &RunEvent) {
    self
      .store
      .iter_mut()
      .for_each(|plugin| plugin.on_event(app, event))
  }

  /// Runs the plugin `extend_api` hook if it exists. Returns whether the invoke message was handled or not.
  ///
  /// The message is not handled when the plugin exists **and** the command does not.
  pub(crate) fn extend_api(&mut self, plugin: &str, invoke: Invoke<R>) -> bool {
    for p in self.store.iter_mut() {
      if p.name() == plugin {
        return p.extend_api(invoke);
      }
    }
    invoke.resolver.reject(format!("plugin {plugin} not found"));
    true
  }
}
