use std::collections::BTreeSet;

use proc_macro::TokenStream;
use quote::quote;
use syn::{braced, parenthesized, parse_macro_input, Expr, Ident, Token, Type};
use syn::parse::{Error, Parse, ParseStream, Result};
use syn::punctuated::Punctuated;

#[proc_macro]
pub fn fsm(input: TokenStream) -> TokenStream {
    let Fsm {
        name,
        error,
        context,
        states,
        events,
        transitions,
    } = parse_macro_input!(input as Fsm);

    let mut state_impls = quote! {};
    for state in &states {
        state_impls = quote! {
            #state_impls

            impl tamata::State<#name> for #state {}
        };
    }

    let mut event_impls = quote! {};
    for event in &events {
        event_impls = quote! {
            #event_impls

            impl tamata::Event<#name> for #event {}
        };
    }

    let state_enum_name = quote::format_ident!("{}State", name);
    let mut state_enum_variants = quote! {};
    for state in &states {
        state_enum_variants = quote!{
            #state_enum_variants
            #state(#state),
        }
    }
    let state_enum = quote! {
        #[derive(Debug)]
        pub enum #state_enum_name {
            #state_enum_variants
        }
    };

    let mut state_enum_from_impls = quote! {};
    for state in &states {
        state_enum_from_impls = quote! {
            #state_enum_from_impls

            impl From<#state> for #state_enum_name {
                fn from(state: #state) -> #state_enum_name {
                    #state_enum_name :: #state(state)
                }
            }
        }
    }

    let event_enum_name = quote::format_ident!("{}Event", name);
    let mut event_enum_variants = quote! {};
    for event in &events {
        event_enum_variants = quote!{
            #event_enum_variants
            #event(#event),
        }
    }
    let event_enum = quote! {
        #[derive(Debug)]
        pub enum #event_enum_name {
            #event_enum_variants
        }
    };

    let mut event_enum_from_impls = quote! {};
    for event in &events {
        event_enum_from_impls = quote! {
            #event_enum_from_impls

            impl From<#event> for #event_enum_name {
                fn from(event: #event) -> #event_enum_name {
                    #event_enum_name :: #event(event)
                }
            }
        }
    }

    let mut enum_transitions = quote! {};
    for transition in &transitions {
        let state = &transition.state;
        let event = &transition.event;
        let next = &transition.next;
        let action = &transition.action;

        if let Some(action) = action {
            enum_transitions = quote! {
                #enum_transitions

                (#state_enum_name::#state(s), #event_enum_name::#event(e)) => {
                    impl tamata::Transition<#name, #event> for #state {
                        type Next = #next;

                        fn send(
                            self,
                            event: #event,
                            ctx: #context,
                        ) -> Result<#next, #error> {
                            (#action)(self, event, ctx)
                        }
                    }

                    let next = tamata::Transition::<#name, #event>::send(s, e, ctx)?;
                    let next = #state_enum_name::#next(next);
                    tamata::Sent::Valid(next)
                },
            }
        } else {
            enum_transitions = quote! {
                #enum_transitions

                (#state_enum_name::#state(s), #event_enum_name::#event(e)) => {
                    let next = tamata::Transition::<#name, #event>::send(s, e, ctx)?;
                    let next = #state_enum_name::from(next);
                    tamata::Sent::Valid(next)
                },
            }
        };
    }

    let impl_state_enum = quote! {
        impl #state_enum_name {
            pub fn send(
                self,
                event: impl Into<#event_enum_name>,
                ctx: #context
            ) -> Result<tamata::Sent<#name>, #error> {
                let next = match (self, event.into()) {
                    #enum_transitions
                    (state, event) => {
                        tamata::Sent::Invalid(state, event)
                    }
                };

                Ok(next)
            }
        }
    };

    let impl_fsm = quote! {
        impl tamata::Fsm for #name {
            type Error = #error;
            type Context = #context;

            type State = #state_enum_name;
            type Event = #event_enum_name;
        }
    };

    let expanded = quote! {
        #impl_fsm

        #state_impls

        #event_impls

        #state_enum

        #state_enum_from_impls

        #event_enum

        #event_enum_from_impls

        #impl_state_enum
    };

    TokenStream::from(expanded)
}

struct Fsm {
    name: Ident,
    error: Type,
    context: Type,
    states: Vec<Ident>,
    events: Vec<Ident>,
    transitions: Vec<Transition>,
}

impl Parse for Fsm {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;

        input.parse::<Token![,]>()?;

        let error = input.parse::<Ident>()?;
        if error != "Error" {
            return Err(Error::new(error.span(), "expected `Error`"));
        }
        input.parse::<Token![=]>()?;
        let error: Type = input.parse()?;

        input.parse::<Token![,]>()?;

        let context = input.parse::<Ident>()?;
        if context != "Context" {
            return Err(Error::new(context.span(), "expected `Context`"));
        }
        input.parse::<Token![=]>()?;
        let context: Type = input.parse()?;

        // Optional trailing comma.
        let _ = input.parse::<Token![,]>();

        let transitions;
        braced!(transitions in input);
        let transitions: Punctuated<Transition, Token![,]> =
            transitions.parse_terminated(Transition::parse)?;

        let transitions: Vec<_> = transitions.into_iter().collect();

        // Optional trailing comma.
        let _ = input.parse::<Token![,]>();

        let mut states = BTreeSet::default();
        let mut events = BTreeSet::default();

        for transition in &transitions {
            states.insert(transition.state.clone());
            states.insert(transition.next.clone());
            events.insert(transition.event.clone());
        }

        let states: Vec<_> = states.into_iter().collect();
        let events: Vec<_> = events.into_iter().collect();

        Ok(Fsm {
            name,
            error,
            context,
            states,
            events,
            transitions,
        })
    }
}

struct Transition {
    state: Ident,
    event: Ident,
    next: Ident,
    action: Option<Expr>,
}

impl Parse for Transition {
    fn parse(input: ParseStream) -> Result<Self> {
        let state: Ident = input.parse()?;

        let events;
        parenthesized!(events in input);
        let events: Punctuated<Ident, Token![,]> =
            events.parse_terminated(Ident::parse)?;

        let event: Ident = events.into_iter().next().unwrap();

        input.parse::<Token![->]>()?;

        let next: Ident = input.parse()?;

        let action = if input.peek(Token![=]) {
            input.parse::<Token![=]>()?;
            let action: Expr = input.parse()?;
            Some(action)
        } else {
            None
        };

        Ok(Transition {
            state,
            event,
            next,
            action,
        })
    }
}
