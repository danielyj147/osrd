use quote::quote;
use quote::ToTokens;

pub(crate) struct CreateBatchImpl {
    pub(super) model: syn::Ident,
    pub(super) table_name: syn::Ident,
    pub(super) table_mod: syn::Path,
    pub(super) row: syn::Ident,
    pub(super) changeset: syn::Ident,
    pub(super) field_count: usize,
}

impl ToTokens for CreateBatchImpl {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let Self {
            model,
            table_name,
            table_mod,
            row,
            changeset,
            field_count,
        } = self;

        tokens.extend(quote! {
            #[automatically_derived]
            #[async_trait::async_trait]
            impl crate::modelsv2::CreateBatch<#changeset> for #model {
                async fn create_batch<
                    I: std::iter::IntoIterator<Item = #changeset> + Send + 'async_trait,
                    C: Default + std::iter::Extend<Self> + Send,
                >(
                    conn: &mut crate::modelsv2::Connection,
                    values: I,
                ) -> crate::error::Result<C> {
                    use crate::modelsv2::Model;
                    use #table_mod::dsl;
                    use diesel::prelude::*;
                    use diesel_async::RunQueryDsl;
                    use futures_util::stream::TryStreamExt;
                    Ok(crate::chunked_for_libpq! {
                        #field_count,
                        values,
                        C::default(),
                        chunk => {
                            diesel::insert_into(dsl::#table_name)
                                .values(chunk)
                                .load_stream::<#row>(conn)
                                .await
                                .map(|s| s.map_ok(<#model as Model>::from_row).try_collect::<Vec<_>>())?
                                .await?
                        }
                    })
                }
            }
        });
    }
}
