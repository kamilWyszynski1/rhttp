use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, GenericParam, Generics};

#[proc_macro_derive(FromStored)]
pub fn my_macro(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let name = input.ident;

    // Add a bound `T: FromParam` to every type parameter T.
    let generics = add_trait_bounds(input.generics);
    let (impl_generics, _, _) = generics.split_for_impl();

    // Generate an expression call FromParam on .0 field of a struct.
    let call = call_from_param(&input.data);

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        // The generated impl.
        impl #impl_generics core::request::FromStored for #name  {
            fn from_stored(stored: String) -> anyhow::Result<Self> {
               #call
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Add a bound `T: FromParam` to every type parameter T.
fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param
                .bounds
                .push(parse_quote!(core::request::FromParam));
        }
    }
    generics
}

fn call_from_param(data: &Data) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                syn::Fields::Unnamed(ref fields) => {
                    // Check if we only have 1 field, if so expand to expression:
                    //
                    // self.0.from_param(param)
                    if fields.unnamed.len() != 1 {
                        panic!("only single tuple value allowed");
                    }

                    let field = fields.unnamed.iter().next().unwrap().clone();
                    let ty = field.ty.clone();

                    quote_spanned!(field.span() =>
                        Ok(Self(#ty::from_stored(stored)?))
                    )
                }
                syn::Fields::Unit | syn::Fields::Named(_) => unimplemented!(),
            }
        }
        Data::Enum(_) | Data::Union(_) => unimplemented!(),
    }
}
