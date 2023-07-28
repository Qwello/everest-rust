use crate::schema::interface::{Argument, Command, Type};
use crate::schema::{manifest::ProvidesEntry, Interface, Manifest};
use anyhow::{Context, Result};
use argh::FromArgs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use titlecase::titlecase;

// TODO(sirver): Using quote && syn here is probably overkill. I would fair better and get nicer
// code with just using strings.

#[derive(Debug, Serialize)]
struct ProvidesInterface {
    interface: String,
}

fn concat_words(words: &[&str]) -> String {
    let mut concatenated = String::new();
    for word in words {
        let capitalized = titlecase(word);
        concatenated.push_str(&capitalized);
    }
    concatenated
}

#[derive(Debug, Serialize)]
struct Metadata {
    module: String,
    provides: BTreeMap<String, ProvidesInterface>,
}

fn emit_metadata(
    module_name: &str,
    manifest_provides: &BTreeMap<String, ProvidesEntry>,
) -> Result<TokenStream> {
    let mut provides = BTreeMap::new();
    for (name, i) in manifest_provides {
        provides.insert(
            name.to_string(),
            ProvidesInterface {
                interface: i.interface.to_string(),
            },
        );
    }
    let metadata = serde_json::to_string(&Metadata {
        module: module_name.to_string(),
        provides,
    })
    .expect("always works for this type");

    Ok(quote! {
        const METADATA: &str = #metadata;
    })
}

fn type_for_argument(arg: &Argument) -> Result<TokenStream> {
    let s = match arg {
        Argument::Single(Type::Boolean) => quote! { bool },
        Argument::Single(Type::String(_)) => quote! { String },
        Argument::Single(_) => {
            todo!("not yet implemented: {arg:?}");
        }
        Argument::Multiple(_) => {
            // TODO(sirver): We do not further dig deep, we just accept any serde_json::Value if we
            // accept more than one. The user of the framework has to sort things out manually.
            quote! { ::serde_json::Value }
        }
    };
    Ok(s)
}

fn emit_command(cmd_name: &str, cmd: &Command) -> Result<TokenStream> {
    let cmd_ident = format_ident!("{}", cmd_name);

    let mut doc = format!("{}\n\n", cmd.description);
    let mut args = Vec::new();
    for (arg_name, arg) in &cmd.arguments {
        doc.push_str(&format!(
            "`{}`: {}\n",
            arg_name,
            arg.description
                .as_ref()
                .map(|s| &s as &str)
                .unwrap_or("not documented")
        ));
        let arg_type = type_for_argument(&arg.arg)?;
        let arg_ident = format_ident!("{}", arg_name);
        args.push(quote! { #arg_ident: #arg_type, });
    }

    let result = match &cmd.result {
        None => quote! { () },
        Some(r) => {
            doc.push_str(&format!(
                "\nReturns: {}",
                r.description
                    .as_ref()
                    .map(|s| &s as &str)
                    .unwrap_or("not documented\n")
            ));
            let arg_type = type_for_argument(&r.arg)?;
            quote! { #arg_type }
        }
    };

    let trimmed_doc = doc.trim();
    Ok(quote! {
        #[doc = #trimmed_doc]
        async fn #cmd_ident(&mut self, #(#args)*) -> ::everest::Result<#result>;
    })
}

fn emit_interface_service_trait(
    provides_entry: &ProvidesEntry,
    interface: &Interface,
) -> Result<TokenStream> {
    let trait_name = format_ident!("{}", concat_words(&[&provides_entry.interface, "service"]));

    let mut cmds = Vec::new();

    for (cmd_name, cmd) in &interface.cmds {
        cmds.push(emit_command(cmd_name, &cmd)?);
    }
    let description = &provides_entry.description;
    Ok(quote! {
        #[doc = #description]
        #[async_trait::async_trait]
        pub trait #trait_name {
            #( #cmds )*
        }
    })
}

/// Returns [ InterfaceService, InterfaceService ]
fn emit_module_struct_generics_traits(manifest: &Manifest) -> Vec<TokenStream> {
    let mut entries = Vec::new();
    for (slot_name, provides_entry) in manifest.provides.iter() {
        let trait_name = format_ident!("{}", concat_words(&[&provides_entry.interface, "service"]));
        entries.push(quote! { #trait_name });
    }
    entries
}

/// Returns [ Slot1ServiceImpl, Slot2ServiceImpl ]
fn emit_module_struct_generics_impls(manifest: &Manifest) -> Vec<TokenStream> {
    let mut entries = Vec::new();
    for (slot_name, provides_entry) in manifest.provides.iter() {
        let impl_name = format_ident!("{}", concat_words(&[slot_name, "service", "impl"]));
        entries.push(quote! { #impl_name });
    }
    entries
}

fn emit_command_implementation_glue(
    slot_name: &str,
    cmd_name: &str,
    cmd: &Command,
) -> Result<TokenStream> {
    let module_ident = format_ident!("{}_service", slot_name);
    let cmd_ident = format_ident!("{}", cmd_name);

    let mut args_define = Vec::new();
    let mut args_call = Vec::new();
    for (arg_name, arg) in &cmd.arguments {
        let arg_ident = format_ident!("{}", arg_name);
        let arg_type = type_for_argument(&arg.arg)?;
        args_define.push(quote! {
            let #arg_ident: #arg_type = ::serde_json::from_value(
                data.args
                    .remove(#arg_name)
                    .ok_or(everest::Error::MissingArgument(#arg_name))?,
                )
                .map_err(|_| everest::Error::InvalidArgument(#arg_name))?;
            // TODO(sirver): Validation should happen here (pattern, minimum, maximum, minLen and
            // so on)
        });
        args_call.push(quote! { #arg_ident });
    }

    let call = quote! {
        #[allow(clippy::let_unit_value)]
        let retval = module.#module_ident.#cmd_ident( #( #args_call ),* ).await?;
    };

    Ok(quote! {
        #cmd_name => {
            #( #args_define )*
            #call
            module
                .publish(
                    &format!("{}/cmd", #slot_name),
                    serde_json::to_string(&::everest::Command::Result {
                        name,
                        data: ::everest::ResultData {
                            id: data.id,
                            origin: module.module_name.clone(),
                            retval: {
                                #[allow(clippy::useless_conversion)]
                                retval.into()
                            },
                        },
                    })
                    .expect("serialization should be infallible for this data type"),
                )
                .await?;
        }
    })
}

fn emit_interface_service_glue(
    manifest: &Manifest,
    slot_name: &str,
    interface: &Interface,
) -> Result<TokenStream> {
    let module_name = format_ident!("{}_service", slot_name);
    let trait_name = format_ident!(
        "{}",
        concat_words(&[&manifest.provides[slot_name].interface, "service"])
    );
    let generic_name = format_ident!("{}", concat_words(&[slot_name, "service", "impl"]));
    let generics_impl = emit_module_struct_generics_impls(manifest);

    let append_cmd_topic = if interface.cmds.is_empty() {
        quote! {}
    } else {
        quote! { rv.insert(format!("everest/{module_name}/{}/cmd", #slot_name)); }
    };

    let mut cmd_impls = Vec::new();
    for (cmd_name, cmd) in &interface.cmds {
        cmd_impls.push(emit_command_implementation_glue(slot_name, cmd_name, cmd)?);
    }

    Ok(quote! {
        mod #module_name {
            use super::{ #trait_name, Module };

            pub fn generate_topics(module_name: &str) -> ::std::collections::HashSet<String> {
                let mut rv = ::std::collections::HashSet::new();
                #append_cmd_topic
                rv
            }

            pub async fn handle_mqtt_message<#generic_name: #trait_name>(
                module: &mut Module< #( #generics_impl ),* >,
                payload: &[u8],
            ) -> ::everest::Result<()> {
                // TODO(sirver): This quietly ignores wrong input.
                let Ok(cmd) = ::serde_json::from_slice::<::everest::Command>(payload) else {
                    return Ok(());
                };
                let (name, mut data) = match cmd {
                    ::everest::Command::Call { name, data } => (name, data),
                    ::everest::Command::Result { .. } => return Ok(()),
                };

                match &name as &str {
                    #( #cmd_impls ),*
                    _ => {
                        // Everest ignores unknown commands without error message.
                    }
                }
                Ok(())
            }
        }
    })
}

fn emit_module_struct(module_name: &str, manifest: &Manifest) -> Result<TokenStream> {
    let generics_traits = emit_module_struct_generics_traits(manifest);
    let generics_impl = emit_module_struct_generics_impls(manifest);
    let mut service_names = Vec::new();
    let mut service_topics = Vec::new();
    for (slot_name, provides_entry) in manifest.provides.iter() {
        service_names.push(format_ident!("{}_service", slot_name));
        service_topics.push(format_ident!("{}_service_topics", slot_name));
    }

    Ok(quote! {
        pub struct Module< #( #generics_impl: #generics_traits),* > {
            client: ::rumqttc::AsyncClient,
            event_loop: ::rumqttc::EventLoop,
            module_name: String,
            #(
            #service_names: #generics_impl,
            #service_topics: ::std::collections::HashSet<String>
            ),*
        }

        impl< #( #generics_impl: #generics_traits),* > Module<#( #generics_impl ),*> {
            pub async fn init( #( #service_names: #generics_impl),* ) -> ::everest::Result<Self> {
                let (client, event_loop, module_name) = everest::initialize_mqtt(#module_name);

                #(
                    let #service_topics = #service_names::generate_topics(&module_name);
                    for t in #service_topics.iter() {
                        client.subscribe(t, ::rumqttc::QoS::ExactlyOnce).await?;
                    }
                )*

                let m = Module {
                    client,
                    event_loop,
                    module_name,
                    #(
                    #service_names,
                    #service_topics,
                    ),*
                };
                m.publish("metadata", METADATA).await?;
                m.publish("ready", "true").await?;
                Ok(m)
            }

            pub async fn loop_forever(&mut self) -> ::everest::Result<()> {
                use rumqttc::{Event, Packet};

                loop {
                    let msg = self.event_loop.poll().await?;
                    match msg {
                        Event::Incoming(Packet::Publish(data)) => {
                            #(
                            if self.#service_topics.contains(&data.topic as &str) {
                                main_service::handle_mqtt_message(self, &data.payload).await?;
                            }
                            )*
                        }
                        Event::Outgoing(_) | Event::Incoming(_) => (),
                    }
                }
            }

            async fn publish(&self, topic: &str, value: impl Into<Vec<u8>>) -> ::everest::Result<()> {
                self.client
                    .publish(
                        &format!("everest/{}/{topic}", self.module_name),
                        ::rumqttc::QoS::ExactlyOnce,
                        false,
                        value,
                    )
                    .await?;
                Ok(())
            }
        }
    })
}

pub fn emit(module_name: String, manifest_path: PathBuf, everest_core: PathBuf) -> Result<String> {
    let blob = fs::read_to_string(&manifest_path).context("reading manifest file")?;
    let manifest: Manifest = serde_yaml::from_str(&blob)?;

    let mut tokens: Vec<TokenStream> = Vec::new();
    tokens.push(emit_metadata(&module_name, &manifest.provides)?);
    for (slot_name, provides_entry) in manifest.provides.iter() {
        let p = everest_core.join(format!("interfaces/{}.yaml", provides_entry.interface));
        let blob = fs::read_to_string(&p).with_context(|| format!("Reading {p:?}"))?;
        let interface_yaml: Interface = serde_yaml::from_str(&blob)?;

        tokens.push(emit_interface_service_trait(
            &provides_entry,
            &interface_yaml,
        )?);
        tokens.push(emit_interface_service_glue(
            &manifest,
            slot_name,
            &interface_yaml,
        )?);
    }

    tokens.push(emit_module_struct(&module_name, &manifest)?);

    let out = quote! {
        #( #tokens )*
    }
    .to_string();

    Ok(out)
}
