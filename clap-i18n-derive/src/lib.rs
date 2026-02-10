//! Derive macro for clap-i18n-richformatter
//!
//! This crate provides the `#[clap_i18n]` attribute macro that adds
//! internationalized help and version flags to clap command structs.

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Adds i18n support for clap help and version flags.
///
/// This attribute macro:
/// - Disables the built-in help and version flags
/// - Implements a custom `CommandFactory` that adds translated help and version args
///
/// # Example
/// ```ignore
/// use clap::Parser;
/// use clap_i18n_richformatter::clap_i18n;
///
/// #[derive(Parser)]
/// #[clap_i18n]
/// struct MyArgs {
///     #[arg(long)]
///     name: String,
/// }
/// ```
#[proc_macro_attribute]
pub fn clap_i18n(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();

    // Extract existing fields
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "clap_i18n only supports structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(&input, "clap_i18n only supports structs")
                .to_compile_error()
                .into();
        }
    };

    let existing_fields = fields.iter();

    let expanded = quote! {
        #(#attrs)*
        #vis struct #name #impl_generics #where_clause {
            #(#existing_fields,)*
        }

        impl #name {
            /// Returns a clap Command with i18n-translated help and version flags.
            pub fn command_i18n() -> clap::Command {
                use clap::CommandFactory;

                // Get translated strings at runtime (no compile-time validation)
                let usage_heading: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-usage-heading").into_boxed_str()
                );
                let options_heading: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-options-heading").into_boxed_str()
                );
                let commands_heading: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-commands-heading").into_boxed_str()
                );
                let help_help: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-help-help").into_boxed_str()
                );
                let version_help: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-help-version").into_boxed_str()
                );

                let arguments_heading: &'static str = Box::leak(
                    clap_i18n_richformatter::__private::get_translation("clap-arguments-heading").into_boxed_str()
                );

                // Build help template with styled usage heading (bold + underline like other headings)
                let help_template: clap::builder::StyledStr = {
                    use std::fmt::Write;
                    use clap::builder::styling::{Style, Effects};
                    let mut s = clap::builder::StyledStr::new();
                    let header = Style::new().effects(Effects::BOLD | Effects::UNDERLINE);
                    let _ = write!(s, "{{before-help}}{{about-with-newline}}\n");
                    let _ = write!(s, "{header}{usage_heading}{header:#} {{usage}}\n\n");
                    let _ = write!(s, "{{all-args}}{{after-help}}");
                    s
                };

                let mut cmd = Self::command()
                    // Disable built-in help and version
                    .disable_help_flag(true)
                    .disable_version_flag(true)
                    // Set translated help template
                    .help_template(help_template)
                    // Translate section headings via clap's heading API
                    .next_display_order(900)
                    .next_help_heading(options_heading)
                    .subcommand_help_heading(commands_heading)
                    .subcommand_value_name(commands_heading);

                // Update all positional arguments to use translated heading
                let positional_ids: Vec<_> = cmd.get_positionals()
                    .map(|a| a.get_id().clone())
                    .collect();

                for id in positional_ids {
                    cmd = cmd.mut_arg(id, |a| a.help_heading(arguments_heading));
                }

                // Update all option arguments to use translated heading
                let option_ids: Vec<_> = cmd.get_opts()
                    .map(|a| a.get_id().clone())
                    .collect();

                for id in option_ids {
                    cmd = cmd.mut_arg(id, |a| a.help_heading(options_heading));
                }

                // Add translated help and version args
                cmd.arg(
                        clap::Arg::new("help")
                            .short('h')
                            .long("help")
                            .action(clap::ArgAction::Help)
                            .help(help_help)
                            .global(true)
                    )
                    // .arg(
                    //     clap::Arg::new("version")
                    //         .short('V')
                    //         .long("version")
                    //         .action(clap::ArgAction::Version)
                    //         .help(version_help)
                    //         .global(true)
                    // )
            }

            /// Parse arguments with i18n-translated help, version, and error messages.
            /// Automatically initializes the localizer.
            /// Returns a Result with the parsed args or a clap::Error.
            pub fn parse_i18n() -> Result<Self, clap::error::Error<clap_i18n_richformatter::ClapI18nRichFormatter>> {
                use clap::FromArgMatches;
                clap_i18n_richformatter::init_clap_rich_formatter_localizer();
                Self::command_i18n()
                    .try_get_matches()
                    .and_then(|matches| Self::from_arg_matches(&matches))
                    .map_err(|e| {
                        e.apply::<clap_i18n_richformatter::ClapI18nRichFormatter>()
                    })
            }

            /// Parse arguments and exit on error with formatted i18n message.
            /// Automatically initializes the localizer.
            /// Use this for simple CLI apps that should exit on parse errors.
            pub fn parse_i18n_or_exit() -> Self {
                Self::parse_i18n().unwrap_or_else(|e| e.exit())
            }

            pub fn parse_i18n_from<I, T>(iter: I) -> Result<Self, clap::error::Error<clap_i18n_richformatter::ClapI18nRichFormatter>>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                use clap::FromArgMatches;
                clap_i18n_richformatter::init_clap_rich_formatter_localizer();
                Self::command_i18n()
                    .try_get_matches_from(iter)
                    .and_then(|mut matches| Self::from_arg_matches_mut(&mut matches))
                    .map_err(|e| {
                        e.apply::<clap_i18n_richformatter::ClapI18nRichFormatter>()
                    })
            }

            /// Parse arguments from iterator with i18n-translated content.
            ///
            /// # Arguments
            /// * `iter` - Iterator of command line arguments
            /// * `language_tag` - Optional language tag (e.g., Some("en-US")). When None, defaults to Chinese (zh-CN).
            ///
            /// # Example
            /// ```ignore
            /// // Use English
            /// let result = MyCmd::parse_i18n_from_with_language_tag(args, Some("en-US"));
            /// // Use default (Chinese)
            /// let result = MyCmd::parse_i18n_from_with_language_tag(args, None);
            /// ```
            pub fn parse_i18n_from_with_language_tag<I, T>(
                iter: I,
                language_tag: Option<&str>,
            ) -> Result<Self, clap::error::Error<clap_i18n_richformatter::ClapI18nRichFormatter>>
            where
                I: IntoIterator<Item = T>,
                T: Into<std::ffi::OsString> + Clone,
            {
                use clap::FromArgMatches;
                clap_i18n_richformatter::with_clap_rich_formatter_localizer_for_language_tag(
                    language_tag,
                    || {
                        Self::command_i18n()
                            .try_get_matches_from(iter)
                            .and_then(|mut matches| Self::from_arg_matches_mut(&mut matches))
                            .map_err(|e| {
                                let formatted = e.apply::<clap_i18n_richformatter::ClapI18nRichFormatter>();
                                // Render immediately while in the language scope
                                let rendered = formatted.render();
                                // Store the pre-rendered message for later retrieval
                                clap_i18n_richformatter::__private::set_prerendered_error(rendered.to_string());
                                formatted
                            })
                    },
                )
            }

            /// Get usage/help message with i18n-translated content.
            ///
            /// # Arguments
            /// * `language_tag` - Optional language tag (e.g., Some("en-US")). When None, defaults to Chinese (zh-CN).
            ///
            /// # Example
            /// ```ignore
            /// // Get English help
            /// let usage = MyCmd::get_usage_i18n_with_language_tag(Some("en-US"));
            /// // Get default (Chinese) help
            /// let usage = MyCmd::get_usage_i18n_with_language_tag(None);
            /// ```
            pub fn get_usage_i18n_with_language_tag(language_tag: Option<&str>) -> String {
                clap_i18n_richformatter::with_clap_rich_formatter_localizer_for_language_tag(
                    language_tag,
                    || {
                        Self::command_i18n().render_help().to_string()
                    },
                )
            }

            /// Get long help message with i18n-translated content.
            ///
            /// # Arguments
            /// * `language_tag` - Optional language tag (e.g., Some("en-US")). When None, defaults to Chinese (zh-CN).
            ///
            /// # Example
            /// ```ignore
            /// // Get English long help
            /// let usage = MyCmd::get_long_usage_i18n_with_language_tag(Some("en-US"));
            /// // Get default (Chinese) long help
            /// let usage = MyCmd::get_long_usage_i18n_with_language_tag(None);
            /// ```
            pub fn get_long_usage_i18n_with_language_tag(language_tag: Option<&str>) -> String {
                clap_i18n_richformatter::with_clap_rich_formatter_localizer_for_language_tag(
                    language_tag,
                    || {
                        Self::command_i18n().render_long_help().to_string()
                    },
                )
            }
        }
    };

    TokenStream::from(expanded)
}
