use fuel_indexer_database_types::directives;
use fuel_indexer_lib::utils::local_repository_root;
use fuel_indexer_schema::utils::{
    build_schema_fields_and_types_map, build_schema_objects_set, get_join_directive_info,
    inject_native_entities_into_schema, schema_version, BASE_SCHEMA,
};
use fuel_indexer_types::type_id;
use graphql_parser::parse_schema;
use graphql_parser::schema::{
    Definition, Document, Field, ObjectType, SchemaDefinition, Type, TypeDefinition,
};
use lazy_static::lazy_static;
use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

lazy_static! {
    static ref COPY_TYPES: HashSet<&'static str> =
        HashSet::from(["Json", "Charfield", "Identity", "Blob"]);
}

fn process_type(
    types: &HashSet<String>,
    typ: &Type<String>,
    nullable: bool,
) -> proc_macro2::TokenStream {
    match typ {
        Type::NamedType(t) => {
            if !types.contains(t) {
                panic!("Type '{t}' is undefined.",);
            }

            let id = format_ident! {"{}", t };

            if nullable {
                quote! { Option<#id> }
            } else {
                quote! { #id }
            }
        }
        Type::ListType(_t) => panic!("Got a list type, we don't handle this yet..."),
        Type::NonNullType(t) => process_type(types, t, false),
    }
}

fn process_field(
    types: &HashSet<String>,
    field: &Field<String>,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::Ident,
    proc_macro2::TokenStream,
) {
    let Field {
        name, field_type, ..
    } = field;
    let typ = process_type(types, field_type, true);
    let ident = format_ident! {"{}", name};

    let (is_nullable, column_type) = get_column_type(typ.clone());

    let extractor = if is_nullable {
        quote! {
            let item = vec.pop().expect("Missing item in row.");
            let #ident = match item {
                FtColumn::#column_type(t) => t,
                _ => panic!("Invalid column type: {:?}.", item),
            };
        }
    } else {
        quote! {
            let item = vec.pop().expect("Missing item in row.");
            let #ident = match item {
                FtColumn::#column_type(t) => match t {
                    Some(inner_type) => { inner_type },
                    None => {
                        panic!("Non-nullable type is returning a None value.")
                    }
                },
                _ => panic!("Invalid column type: {:?}.", item),
            };
        }
    };

    (typ, ident, extractor)
}

fn process_fk_field<'a>(
    types: &HashSet<String>,
    obj: &ObjectType<'a, String>,
    field: &Field<'a, String>,
    types_map: &HashMap<String, String>,
    is_nullable: bool,
) -> (
    proc_macro2::TokenStream,
    proc_macro2::Ident,
    proc_macro2::TokenStream,
) {
    let directives::Join {
        field_name,
        reference_field_type_name,
        ..
    } = get_join_directive_info(field, obj, types_map);

    let field_type: Type<'a, String> = Type::NamedType(reference_field_type_name);
    let typ = process_type(types, &field_type, is_nullable);
    let ident = format_ident! {"{}", field_name.to_lowercase()};

    let (_, column_type) = get_column_type(typ.clone());

    let extractor = if is_nullable {
        quote! {
            let item = vec.pop().expect("Missing item in row.");
            let #ident = match item {
                FtColumn::#column_type(t) => t,
                _ => panic!("Invalid column type: {:?}.", item),
            };
        }
    } else {
        quote! {
            let item = vec.pop().expect("Missing item in row.");
            let #ident = match item {
                FtColumn::#column_type(t) => match t {
                    Some(inner_type) => { inner_type },
                    None => {
                        panic!("Non-nullable type is returning a None value.")
                    }
                },
                _ => panic!("Invalid column type: {:?}.", item),
            };
        }
    };

    (typ, ident, extractor)
}

#[allow(clippy::too_many_arguments)]
fn process_type_def(
    query_root: &str,
    namespace: &str,
    types: &HashSet<String>,
    typ: &TypeDefinition<String>,
    processed: &mut HashSet<String>,
    primitives: &HashSet<String>,
    types_map: &HashMap<String, String>,
    is_native: bool,
) -> Option<proc_macro2::TokenStream> {
    match typ {
        TypeDefinition::Object(obj) => {
            if obj.name == *query_root {
                return None;
            }

            let name = &obj.name;
            let type_id = type_id(namespace, name);
            let mut block = quote! {};
            let mut row_extractors = quote! {};
            let mut construction = quote! {};
            let mut flattened = quote! {};

            for field in &obj.fields {
                let (mut type_name, mut field_name, mut ext) =
                    process_field(types, field);

                let (is_nullable, mut column_type_name) =
                    get_column_type(type_name.clone());

                let mut column_type_name_str = column_type_name.to_string();

                if processed.contains(&column_type_name_str)
                    && !primitives.contains(&column_type_name_str)
                {
                    (type_name, field_name, ext) =
                        process_fk_field(types, obj, field, types_map, is_nullable);
                    column_type_name = type_name.clone();
                    column_type_name_str = column_type_name.to_string();
                }

                processed.insert(column_type_name_str.clone());

                let decoder = if is_nullable {
                    quote! { FtColumn::#column_type_name(self.#field_name), }
                } else {
                    quote! { FtColumn::#column_type_name(Some(self.#field_name.clone())), }
                };

                block = quote! {
                    #block
                    #field_name: #type_name,
                };

                row_extractors = quote! {
                    #ext

                    #row_extractors
                };

                construction = quote! {
                    #construction
                    #field_name,
                };

                flattened = quote! {
                    #flattened
                    #decoder
                };
            }
            let strct = format_ident! {"{}", name};

            processed.insert(strct.to_string());

            if is_native {
                Some(quote! {
                    #[derive(Debug, PartialEq, Eq, Hash)]
                    pub struct #strct {
                        #block
                    }

                    #[async_trait::async_trait]
                    impl Entity for #strct {
                        const TYPE_ID: i64 = #type_id;

                        fn from_row(mut vec: Vec<FtColumn>) -> Self {
                            #row_extractors
                            Self {
                                #construction
                            }
                        }

                        fn to_row(&self) -> Vec<FtColumn> {
                            vec![
                                #flattened
                            ]
                        }

                        async fn load(id: u64) -> Option<Self> {
                            unsafe {
                                match &db {
                                    Some(d) => {
                                        match d.lock().await.get_object(Self::TYPE_ID, id).await {
                                            Some(bytes) => {
                                                let columns: Vec<FtColumn> = bincode::deserialize(&bytes).expect("Serde error.");
                                                let obj = Self::from_row(columns);
                                                Some(obj)
                                            },
                                            None => None,
                                        }
                                    }
                                    None => None,
                                }
                            }
                        }

                        async fn save(&self) {
                            unsafe {
                                match &db {
                                    Some(d) => {
                                        d.lock().await.put_object(
                                            Self::TYPE_ID,
                                            self.to_row(),
                                            serialize(&self.to_row())
                                        ).await;
                                    }
                                    None => {},
                                }
                            }
                        }
                    }
                })
            } else {
                Some(quote! {
                    #[derive(Debug, PartialEq, Eq, Hash)]
                    pub struct #strct {
                        #block
                    }

                    impl Entity for #strct {
                        const TYPE_ID: i64 = #type_id;

                        fn from_row(mut vec: Vec<FtColumn>) -> Self {
                            #row_extractors
                            Self {
                                #construction
                            }
                        }

                        fn to_row(&self) -> Vec<FtColumn> {
                            vec![
                                #flattened
                            ]
                        }
                    }
                })
            }
        }
        obj => panic!("Unexpected type: {obj:?}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn process_definition(
    query_root: &str,
    namespace: &str,
    types: &HashSet<String>,
    definition: &Definition<String>,
    processed: &mut HashSet<String>,
    primitives: &HashSet<String>,
    types_map: &HashMap<String, String>,
    is_native: bool,
) -> Option<proc_macro2::TokenStream> {
    match definition {
        Definition::TypeDefinition(def) => process_type_def(
            query_root, namespace, types, def, processed, primitives, types_map,
            is_native,
        ),
        Definition::SchemaDefinition(_def) => None,
        def => {
            panic!("Unhandled definition type: {def:?}");
        }
    }
}

fn get_query_root(types: &HashSet<String>, ast: &Document<String>) -> String {
    let schema = ast.definitions.iter().find_map(|def| {
        if let Definition::SchemaDefinition(d) = def {
            Some(d)
        } else {
            None
        }
    });

    let SchemaDefinition { query, .. } = schema.expect("Schema definition not found.");

    let name = query
        .as_ref()
        .expect("Schema definition must specify a query root.")
        .into();

    if !types.contains(&name) {
        panic!("Query root not defined.");
    }

    name
}

fn const_item(id: &str, value: &str) -> proc_macro2::TokenStream {
    let ident = format_ident! {"{}", id};

    let fn_ptr = format_ident! {"get_{}_ptr", id.to_lowercase()};
    let fn_len = format_ident! {"get_{}_len", id.to_lowercase()};

    quote! {
        const #ident: &'static str = #value;

        #[no_mangle]
        fn #fn_ptr() -> *const u8 {
            #ident.as_ptr()
        }

        #[no_mangle]
        fn #fn_len() -> u32 {
            #ident.len() as u32
        }
    }
}

pub(crate) fn process_graphql_schema(
    namespace: String,
    schema_path: String,
    is_native: bool,
) -> proc_macro2::TokenStream {
    let path = match local_repository_root() {
        Some(p) => Path::new(&p).join(schema_path),
        None => PathBuf::from(&schema_path),
    };

    let mut file = match File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            proc_macro_error::abort_call_site!(
                "Could not open schema file {:?} {:?}",
                path,
                e
            )
        }
    };

    let mut text = String::new();
    file.read_to_string(&mut text).expect("IO error");

    let text = inject_native_entities_into_schema(&text);

    let base_ast = match parse_schema::<String>(BASE_SCHEMA) {
        Ok(ast) => ast,
        Err(e) => {
            proc_macro_error::abort_call_site!("Error parsing graphql schema {:?}", e)
        }
    };
    let (primitives, _) = build_schema_objects_set(&base_ast);

    let ast = match parse_schema::<String>(&text) {
        Ok(ast) => ast,
        Err(e) => {
            proc_macro_error::abort_call_site!("Error parsing graphql schema {:?}", e)
        }
    };
    let (mut types, _) = build_schema_objects_set(&ast);
    types.extend(primitives.clone());

    let namespace_tokens = const_item("NAMESPACE", &namespace);
    let version = const_item("VERSION", &schema_version(&text));

    let mut output = quote! {
        #namespace_tokens
        #version
    };

    let query_root = get_query_root(&types, &ast);

    let mut processed: HashSet<String> = HashSet::new();
    let types_map: HashMap<String, String> = build_schema_fields_and_types_map(&ast);

    for definition in ast.definitions.iter() {
        if let Some(def) = process_definition(
            &query_root,
            &namespace,
            &types,
            definition,
            &mut processed,
            &primitives,
            &types_map,
            is_native,
        ) {
            output = quote! {
                #output
                #def
            };
        }
    }
    output
}

// Note: This may have to change once we support list types -- deekerno
fn get_column_type(typ: TokenStream) -> (bool, TokenStream) {
    let mut is_option_type = false;
    let tokens: TokenStream = typ
        .into_iter()
        .filter(|token| {
            if let TokenTree::Ident(ident) = token {
                let is_option_token = *ident == "Option";
                if is_option_token {
                    is_option_type = true;
                    return false;
                }

                // Keep ident tokens that are not "Option"
                return true;
            }
            false
        })
        .collect::<TokenStream>();

    (is_option_type, tokens)
}
