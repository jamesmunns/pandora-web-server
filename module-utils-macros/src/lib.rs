// Copyright 2024 Wladimir Palant
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Macros for module-utils crate
//!
//! You normally shouldn’t use this crate directly but the `module-utils` crate instead.

mod derive_deserialize_map;
mod derive_request_filter;
mod merge_conf;
mod merge_opt;
#[cfg(test)]
mod tests;
mod utils;

use proc_macro::TokenStream;

/// This attribute macro merges the command-line arguments from all structs identified as field of
/// the current struct. The result will implement `structopt::StructOpt` and `Debug` automatically.
/// All field types are required to implement `structopt::StructOpt` and `Debug`.
///
/// ```rust
/// use pingora_core::server::configuration::Opt as ServerOpt;
/// use module_utils::merge_opt;
/// use static_files_module::StaticFilesOpt;
/// use structopt::StructOpt;
///
/// #[derive(Debug, StructOpt)]
/// struct MyAppOpt {
///     /// IP address and port for the server to listen on
///     #[structopt(long, default_value = "127.0.0.1:8080")]
///     listen: String,
/// }
///
/// /// Starts my great application.
/// ///
/// /// Additional application description just to make a structopt bug work-around work.
/// #[merge_opt]
/// struct Opt {
///     app: MyAppOpt,
///     server: ServerOpt,
///     static_files: StaticFilesOpt,
/// }
///
/// let opt = Opt::from_args();
/// println!("Application options: {:?}", opt.app);
/// println!("Pingora server options: {:?}", opt.server);
/// println!("Static files options: {:?}", opt.static_files);
/// ```
#[proc_macro_attribute]
pub fn merge_opt(_args: TokenStream, input: TokenStream) -> TokenStream {
    merge_opt::merge_opt(input).unwrap_or_else(|err| err.into_compile_error().into())
}

/// This attribute macro merges the configuration settings from all structs identified as field of
/// the current struct. The result will implement `DeserializeMap`, `Deserialize`, `Debug`
/// and `Default` traits automatically. All field types are required to implement
/// `DeserializeMap`, `Debug` and `Default`.
///
/// ```rust
/// use pingora_core::server::configuration::ServerConf;
/// use module_utils::{merge_conf, DeserializeMap, FromYaml};
/// use static_files_module::StaticFilesConf;
/// use std::path::PathBuf;
///
/// #[derive(Debug, Default, DeserializeMap)]
/// struct MyAppConf {
///     /// IP address and port for the server to listen on
///     listen: String,
/// }
///
/// #[merge_conf]
/// struct Conf {
///     app: MyAppConf,
///     server: ServerConf,
///     static_files: StaticFilesConf,
/// }
///
/// let conf = Conf::from_yaml(r#"
///     listen: 127.0.0.1:8080
///     error_log: error.log
///     root: .
/// "#).unwrap();
/// assert_eq!(conf.app.listen, String::from("127.0.0.1:8080"));
/// assert_eq!(conf.server.error_log, Some(String::from("error.log")));
/// assert_eq!(conf.static_files.root, Some(PathBuf::from(".")));
/// ```
///
/// Unknown fields will cause an error during deserialization:
///
/// ```rust
/// use compression_module::CompressionConf;
/// use module_utils::{merge_conf, FromYaml};
/// use static_files_module::StaticFilesConf;
///
/// #[merge_conf]
/// struct Conf {
///     compression: CompressionConf,
///     static_files: StaticFilesConf,
/// }
///
/// assert!(Conf::from_yaml(r#"
///     root: .
///     compression_level: 3
///     unknown_field: flagged
/// "#).is_err());
/// ```
#[proc_macro_attribute]
pub fn merge_conf(_attr: TokenStream, input: TokenStream) -> TokenStream {
    merge_conf::merge_conf(input).unwrap_or_else(|err| err.into_compile_error().into())
}

/// This macro will automatically implement `RequestFilter` by chaining the handlers identified
/// in the struct’s fields.
///
/// Each handler has to implement `RequestFilter` trait. The handlers will be called in the order
/// in which they are listed. Each handler can prevent the subsequent handlers from being called by
/// returning `RequestFilterResult::ResponseSent` or `RequestFilterResult::Handled`.
///
/// The configuration and context for the struct will be implemented implicitly. These will have
/// the configuration/context of the respective handler in a field with the same name as the
/// handler in this struct.
///
/// ```rust
/// use module_utils::{FromYaml, RequestFilter};
/// use compression_module::CompressionHandler;
/// use static_files_module::StaticFilesHandler;
///
/// #[derive(Debug, RequestFilter)]
/// struct Handler {
///     compression: CompressionHandler,
///     static_files: StaticFilesHandler,
/// }
///
/// type Conf = <Handler as RequestFilter>::Conf;
///
/// let conf = Conf::from_yaml(r#"
///     root: .
///     compression_level: 3
/// "#).unwrap();
/// let handler: Handler = conf.try_into().unwrap();
/// ```
///
/// As this uses `#[merge_conf]` macro for configurations internally, unknown fields in
/// configuration will cause an error during deserialization:
///
/// ```rust
/// use module_utils::{FromYaml, RequestFilter};
/// use compression_module::CompressionHandler;
/// use static_files_module::StaticFilesHandler;
///
/// #[derive(Debug, RequestFilter)]
/// struct Handler {
///     compression: CompressionHandler,
///     static_files: StaticFilesHandler,
/// }
///
/// type Conf = <Handler as RequestFilter>::Conf;
///
/// assert!(Conf::from_yaml(r#"
///     root: .
///     compression_level: 3
///     unknown_field: flagged
/// "#).is_err());
/// ```
#[proc_macro_derive(RequestFilter)]
pub fn derive_request_filter(input: TokenStream) -> TokenStream {
    derive_request_filter::derive_request_filter(input)
        .unwrap_or_else(|err| err.into_compile_error().into())
}

/// This macro will automatically implement `DeserializeMap` and `serde::Deserialize` traits for a
/// structure.
///
/// This allows `#[merge_conf]` macro to merge this structure efficiently without an
/// intermediate storage that `#[serde(flatten)]` would use. It also allows flagging unsupported
/// configuration fields in merged configurations, effectively implementing
/// `#[serde(deny_unknown_fields)]` that would have been incompatible with `#[serde(flatten)]`.
///
/// The individual fields need to implement `serde::Deserialize`. The following field attributes
/// are supported, striving for compatibility with the corresponding
/// [Serde field attributes](https://serde.rs/field-attrs.html):
///
/// * `#[module_utils(rename = "name")]` or
///   `#[module_utils(rename(deserialize = "name"))]`
///
///   Deserialize this field with the given name instead of its Rust name.
/// * `#[module_utils(alias = "name")]`
///
///   Deserialize this field from the given name or from its Rust name. May be repeated to specify
///   multiple possible names for the same field.
/// * `#[module_utils(skip)]` or `#[serde(skip_deserializing)]`
///
///   Skip this field when deserializing, always use the default value instead.
/// * `#[module_utils(deserialize_with = "path")]`
///
///   Deserialize this field using a function that is different from its implementation of
///   `serde::Deserialize`. The given function must be callable as
///   `fn<'de, D>(D) -> Result<T, D::Error> where D: serde::Deserializer<'de>`, although it may
///   also be generic over `T`. Fields used with `deserialize_with` are not required to implement
///   `serde::Deserialize`.
/// * `#[serde(with = "module")]`
///
///   Same as `deserialize_with` but `$module::deserialize` will be used as the `deserialize_with`
///   function.
///
/// Unknown fields will cause a deserialization error, missing fields will be returned with their
/// default value. Essentially,
/// [Serde container attributes](https://serde.rs/container-attrs.html)
/// `#[serde(deny_unknown_fields)]` and `#[serde(default)]` are implied.
///
/// Example:
///
/// ```rust
/// use module_utils::{DeserializeMap, FromYaml, merge_conf};
///
/// #[derive(Debug, Default, DeserializeMap)]
/// struct Conf1 {
///     value1: u32,
/// }
///
/// #[derive(Debug, Default, DeserializeMap)]
/// struct Conf2 {
///     #[module_utils(rename = "Value2")]
///     value2: String,
///     #[module_utils(skip)]
///     value3: Option<bool>,
/// }
///
/// #[merge_conf]
/// struct Conf {
///     conf1: Conf1,
///     conf2: Conf2,
/// }
///
/// let conf = Conf::from_yaml(r#"
///     value1: 12
///     Value2: "Hi!"
/// "#).unwrap();
///
/// assert_eq!(conf.conf1.value1, 12);
/// assert_eq!(conf.conf2.value2, String::from("Hi!"));
/// assert!(conf.conf2.value3.is_none());
/// ```
#[proc_macro_derive(DeserializeMap, attributes(module_utils))]
pub fn derive_deserialize_map(input: TokenStream) -> TokenStream {
    derive_deserialize_map::derive_deserialize_map(input)
        .unwrap_or_else(|err| err.into_compile_error().into())
}
