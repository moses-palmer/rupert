use proc_macro::*;
use quote::{quote, ToTokens};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{
    parse_macro_input, parse_quote, AngleBracketedGenericArguments, Attribute,
    GenericArgument, Ident, ItemStruct, Path, PathArguments, PathSegment,
    Token, Type, TypePath, VisPublic, Visibility,
};

/// The name of the attribute providing the partial struct name.
const PARTIAL_STRUCT_ATTR: &str = "partial_struct";

/// The name of the attribute providing derives for the partial struct.
const PARTIAL_DERIVE_ATTR: &str = "partial_derive";

/// The name of the attribute providing the default value.
const PARTIAL_DEFAULT_ATTR: &str = "partial_default";

pub fn transform(items: TokenStream) -> TokenStream {
    let partial = parse_macro_input!(items as Partial);

    let definition = partial.definition().unwrap();
    let implementation = partial.implementation().unwrap();

    quote! {
        #definition
        #implementation
    }
    .into()
}

/// A parsed struct.
struct Partial {
    /// The mergable struct definition.
    struct_definition: ItemStruct,
}

impl Partial {
    /// The definition of the partial struct.
    ///
    /// This is a copy of the wrapped struct with every field type wrapped in
    /// [`Option`](core::option::Option).
    pub fn definition(&self) -> Result<impl ToTokens, String> {
        // Rename the struct
        let name = self.partial_name()?;

        // Add derives
        let derives = self.partial_derive();

        let partial_default_attr =
            Ident::new(PARTIAL_DEFAULT_ATTR, Span::call_site().into());
        let partial_struct_attr =
            Ident::new(PARTIAL_STRUCT_ATTR, Span::call_site().into());
        let fields = self.struct_definition.fields.iter().map(|field| {
            let mut field = field.clone();

            // Wrap the type
            field.ty = wrap(
                Ident::new("Option", Span::call_site().into()),
                field
                    .attrs
                    .iter()
                    .filter(|attr| attr.path.is_ident(&partial_struct_attr))
                    .next()
                    .map(|attr| {
                        syn::parse_str(&attr.tokens.to_string()).unwrap()
                    })
                    .unwrap_or_else(|| field.ty),
            );

            // Strip default value attributes
            field.attrs.retain(|attr| {
                !(attr.path.is_ident(&partial_default_attr)
                    || attr.path.is_ident(&partial_struct_attr))
            });

            // Ensure all fields are public
            field.vis = Visibility::Public(VisPublic {
                pub_token: <Token![pub]>::default(),
            });

            field
        });

        let (g, _, w) = self.struct_definition.generics.split_for_impl();
        Ok(quote! {
            #(#derives)*
            pub struct #name #g #w where {
                #(
                    #fields,
                )*
            }
        })
    }

    /// The implementaion of the seed,
    pub fn implementation(&self) -> Result<impl ToTokens, String> {
        let target_struct = self.struct_name();
        let name = self.partial_name()?;
        let field_names = self.field_names()?;
        let field_values = self.field_values(&Ident::new(
            PARTIAL_DEFAULT_ATTR,
            Span::call_site().into(),
        ));

        let (i, g, w) = self.struct_definition.generics.split_for_impl();
        Ok(quote! {
            impl #i #name #g #w{
                /// Merges this partial struct with another one.
                ///
                /// # Arguments
                /// *  `other` - The other struct. Values present in this item
                ///    take precendence.
                pub fn merge(self, other: Self) -> Self {
                    Self {
                        #(
                            #field_names: other
                                .#field_names
                                .or_else(|| self.#field_names),
                        )*
                    }
                }

                pub fn or_else<F: FnOnce() -> Self>(self, f: F) -> Self {
                    self.merge(f())
                }

                pub fn unwrap_or_else<F: FnOnce() -> Self>(
                    self,
                    f: F,
                ) -> #target_struct #g {
                    self.merge(f()).into()
                }
            }

            impl #i Default for #name #g #w {
                fn default() -> Self {
                    Self {
                        #(
                            #field_names: Default::default(),
                        )*
                    }
                }
            }

            impl #i From<#name #g> for #target_struct #g #w {
                fn from(source: #name #g) -> Self {
                    Self {
                        #(
                            #field_names: source
                                .#field_names
                                .unwrap_or_else(|| #field_values)
                                .into(),
                        )*
                    }
                }
            }
        })
    }

    /// The name of the original struct.
    fn struct_name(&self) -> Ident {
        self.struct_definition.ident.clone()
    }

    /// The name of the partial struct.
    fn partial_name(&self) -> Result<Ident, String> {
        let name_attr =
            Ident::new(PARTIAL_STRUCT_ATTR, Span::call_site().into());
        self.struct_definition
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident(&name_attr))
            .next()
            .map(|attr| {
                Ident::new(
                    &unparenthesize(&attr.tokens.to_string()),
                    self.struct_definition.ident.span(),
                )
            })
            .ok_or_else(|| {
                format!(
                    "the attribute {} must specify the name of the partial \
                    struct",
                    PARTIAL_STRUCT_ATTR,
                )
            })
    }

    /// The list of derive attributes to apply to the partial struct.
    fn partial_derive(&self) -> Vec<Attribute> {
        let name_attr =
            Ident::new(PARTIAL_DERIVE_ATTR, Span::call_site().into());
        self.struct_definition
            .attrs
            .iter()
            .filter(|attr| attr.path.is_ident(&name_attr))
            .flat_map(|attr| attr.tokens.clone().into_iter())
            .map(|attr| parse_quote! { #[derive #attr ] })
            .collect()
    }

    /// The field names.
    ///
    /// This method will fail if the wrapped struct is a tuple struct.
    fn field_names(&self) -> Result<Vec<impl ToTokens>, String> {
        self.struct_definition
            .fields
            .iter()
            .map(|field| {
                field
                    .ident
                    .clone()
                    .ok_or_else(|| "tuple structs not supported".into())
            })
            .collect()
    }

    /// The default values for the field values.
    ///
    /// If the attribute `attr_ident` is present, its tokens will be used as
    /// the default values, otherwise `Default::default()` is used.
    ///
    /// # Arguments
    /// *  `attr_ident` - The identifier for the attribute containing a default
    ///    value.
    fn field_values(&self, attr_ident: &Ident) -> Vec<impl ToTokens> {
        self.struct_definition
            .fields
            .iter()
            .map(|field| {
                field
                    .attrs
                    .iter()
                    .filter(|attr| attr.path.is_ident(attr_ident))
                    .next()
                    .map(|attr| attr.tokens.clone())
                    .unwrap_or_else(|| "Default::default()".parse().unwrap())
            })
            .collect()
    }
}

impl Parse for Partial {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            struct_definition: input.parse()?,
        })
    }
}

/// Wraps a type in another type as a generic parameter.
///
/// # Arguments
/// *  `wrapper` - The wrapper type.
/// *  `ty` - The wrapped type.
fn wrap(wrapper: Ident, ty: Type) -> Type {
    Type::Path(TypePath {
        qself: None,
        path: Path {
            leading_colon: None,
            segments: Punctuated::from_iter([PathSegment {
                ident: wrapper,
                arguments: PathArguments::AngleBracketed(
                    AngleBracketedGenericArguments {
                        colon2_token: None,
                        lt_token: <Token![<]>::default(),
                        gt_token: <Token![>]>::default(),
                        args: Punctuated::from_iter([GenericArgument::Type(
                            ty,
                        )]),
                    },
                ),
            }]),
        },
    })
}

fn unparenthesize(string: &String) -> String {
    let mut characters = string.chars();
    if characters.next() == Some('(') && characters.last() == Some(')') {
        string[1..string.len() - 1].into()
    } else {
        String::new()
    }
}
