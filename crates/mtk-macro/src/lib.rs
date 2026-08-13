use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Derives field lenses for a struct.
///
/// For each named field in the struct, this derive macro generates:
/// A. A zero-sized lens struct implementing [`Lens<StructName, FieldType>`](::mtk::ui::Lens).
/// B. An associated constant on the struct matching the field's name (e.g. `AppState::my_field`)
///    that provides a static reference to the lens.
///
/// # Example
/// ```ignore
/// use mtk::Lens;
///
/// #[derive(Lens)]
/// struct AppState {
///     pub click_count: i32,
///     pub dark_mode: bool,
/// }
///
/// // Enables passing field lenses directly into views:
/// // my_view.adapt(AppState::click_count, AppMsg::CounterEvent)
/// ```
#[proc_macro_derive(Lens)]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let vis = &input.vis;

    let mut generated = Vec::new();

    if let Data::Struct(data_struct) = &input.data {
        if let Fields::Named(fields) = &data_struct.fields {
            for field in &fields.named {
                let field_name = field.ident.as_ref().unwrap();
                let field_type = &field.ty;

                let lens_name = quote::format_ident!("{}_{}_lens", name, field_name);

                let lens = quote! {
                    #[allow(non_camel_case_types)]
                    #vis struct #lens_name;

                    impl ::mtk::ui::Lens<#name, #field_type> for #lens_name {
                        fn get<'a>(&self, outer: &'a #name) -> &'a #field_type {
                            &outer.#field_name
                        }
                    }

                    impl #name {
                        #[allow(non_upper_case_globals)]
                        #vis const #field_name: #lens_name = #lens_name;
                    }
                };

                generated.push(lens);
            }
        }
    }

    let expanded = quote! {
        #(#generated)*
    };

    TokenStream::from(expanded)
}
