use proc_macro::*;

mod partial;

/// Generates a struct that can be constructed by merging partial definitions.
///
/// # Examples
///
/// ```
/// # use rupert_macros::{Partial, partial_derive, partial_struct};
///
/// #[derive(Debug, PartialEq, Partial)]
/// #[partial_derive(Clone, Debug, PartialEq)]
/// #[partial_struct(PartialStruct)]
/// struct FullStruct {
///     pub a: bool,
///     pub b: u32,
///     #[partial_default(String::from("c"))]
///     c: String,
/// }
///
/// let partial: PartialStruct = Default::default();
/// assert_eq!(partial.a, None);
/// assert_eq!(partial.b, None);
/// assert_eq!(partial.c, None);
/// assert_eq!(
///     FullStruct {
///         a: false,
///         b: 0,
///         c: "c".into(),
///     },
///     partial.clone().into(),
/// );
///
/// let other = PartialStruct {
///     b: Some(42),
///     ..Default::default()
/// };
///
/// let merged = partial.merge(other);
/// assert_eq!(merged.a, None);
/// assert_eq!(merged.b, Some(42));
/// assert_eq!(merged.c, None);
/// assert_eq!(
///     FullStruct {
///         a: false,
///         b: 42,
///         c: "c".into(),
///     },
///     merged.into(),
/// );
/// ```
#[proc_macro_derive(Partial, attributes(partial_struct, partial_default))]
pub fn partial_main(items: TokenStream) -> TokenStream {
    self::partial::transform(items)
}

#[proc_macro_attribute]
pub fn partial_struct(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

#[proc_macro_attribute]
pub fn partial_derive(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
