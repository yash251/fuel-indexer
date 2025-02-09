use fuel_indexer_database_types::*;
use fuel_indexer_lib::utils::sha256_digest;
use sqlx::{pool::PoolConnection, types::JsonValue, Row, Sqlite};
use tracing::info;

#[cfg(feature = "metrics")]
use fuel_indexer_metrics::METRICS;

pub async fn put_object(
    conn: &mut PoolConnection<Sqlite>,
    query: String,
    bytes: Vec<u8>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.put_object_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(query);

    let query = builder.build();
    let query = query.bind(bytes);
    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn get_object(
    conn: &mut PoolConnection<Sqlite>,
    query: String,
) -> sqlx::Result<Vec<u8>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.get_object_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(query);

    let query = builder.build();

    let row = query.fetch_one(conn).await?;

    Ok(row.get(0))
}

pub async fn run_migration(conn: &mut PoolConnection<Sqlite>) -> sqlx::Result<()> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.run_migration_calls.inc();

    sqlx::migrate!().run(conn).await?;

    Ok(())
}

pub async fn run_query(
    conn: &mut PoolConnection<Sqlite>,
    query: String,
) -> sqlx::Result<JsonValue> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.run_query_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(query);

    let query = builder.build();

    Ok(query
        .fetch_all(conn)
        .await?
        .iter()
        .map(|r| r.get::<'_, JsonValue, usize>(0))
        .collect())
}

pub async fn execute_query(
    conn: &mut PoolConnection<Sqlite>,
    query: String,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.execute_query_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(query);

    let query = builder.build();

    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn root_columns_list_by_id(
    conn: &mut PoolConnection<Sqlite>,
    root_id: i64,
) -> sqlx::Result<Vec<RootColumns>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.root_columns_list_by_id_calls.inc();

    sqlx::query_as!(
        RootColumns,
        r#"SELECT
               id AS "id: i64", root_id AS "root_id: i64", column_name, graphql_type
           FROM graph_registry_root_columns
           WHERE root_id = $1"#,
        root_id
    )
    .fetch_all(conn)
    .await
}

pub async fn new_root_columns(
    conn: &mut PoolConnection<Sqlite>,
    cols: Vec<NewRootColumns>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.new_root_columns_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(
        "INSERT INTO graph_registry_root_columns (root_id, column_name, graphql_type)",
    );

    builder.push_values(cols.into_iter(), |mut b, new_col| {
        b.push_bind(new_col.root_id)
            .push_bind(new_col.column_name)
            .push_bind(new_col.graphql_type);
    });

    let query = builder.build();

    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn new_graph_root(
    conn: &mut PoolConnection<Sqlite>,
    root: NewGraphRoot,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.new_graph_root_calls.inc();

    let mut builder = sqlx::QueryBuilder::new(
        "INSERT INTO graph_registry_graph_root (version, schema_name, query, schema)",
    );

    builder.push_values(std::iter::once(root), |mut b, root| {
        b.push_bind(root.version)
            .push_bind(root.schema_name)
            .push_bind(root.query)
            .push_bind(root.schema);
    });

    let query = builder.build();

    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn graph_root_latest(
    conn: &mut PoolConnection<Sqlite>,
    name: &str,
) -> sqlx::Result<GraphRoot> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.graph_root_latest_calls.inc();

    sqlx::query_as!(
        GraphRoot,
        "SELECT * FROM graph_registry_graph_root WHERE schema_name = $1 ORDER BY id DESC LIMIT 1",
        name
    )
    .fetch_one(conn)
    .await
}

pub async fn type_id_list_by_name(
    conn: &mut PoolConnection<Sqlite>,
    name: &str,
    version: &str,
) -> sqlx::Result<Vec<TypeId>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.type_id_list_by_name_calls.inc();

    sqlx::query_as!(
        TypeId,
        "SELECT id, schema_version, schema_name, graphql_name, table_name FROM graph_registry_type_ids WHERE schema_name = $1 AND schema_version = $2",
        name,
        version
    )
    .fetch_all(conn)
    .await
}

pub async fn type_id_latest(
    conn: &mut PoolConnection<Sqlite>,
    schema_name: &str,
) -> sqlx::Result<String> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.type_id_latest_calls.inc();

    let latest = sqlx::query_as!(
        IdLatest,
        "SELECT schema_version FROM graph_registry_type_ids WHERE schema_name = $1 ORDER BY id",
        schema_name
    )
    .fetch_one(conn)
    .await?;

    Ok(latest.schema_version)
}

pub async fn type_id_insert(
    conn: &mut PoolConnection<Sqlite>,
    type_ids: Vec<TypeId>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.type_id_insert_calls.inc();

    let mut builder = sqlx::QueryBuilder::new("INSERT INTO graph_registry_type_ids (id, schema_version, schema_name, graphql_name, table_name)");

    builder.push_values(type_ids.into_iter(), |mut b, tid| {
        b.push_bind(tid.id)
            .push_bind(tid.schema_version)
            .push_bind(tid.schema_name)
            .push_bind(tid.graphql_name)
            .push_bind(tid.table_name);
    });

    let query = builder.build();

    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn schema_exists(
    conn: &mut PoolConnection<Sqlite>,
    name: &str,
    version: &str,
) -> sqlx::Result<bool> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.schema_exists_calls.inc();

    let versions = sqlx::query_as!(NumVersions, r#"SELECT count(*) as "num?: i64" FROM graph_registry_type_ids WHERE schema_name = $1 AND schema_version = $2"#, name, version).fetch_one(conn).await?;
    Ok(versions
        .num
        .expect("num field should be present in versions query results")
        > 0)
}

pub async fn new_column_insert(
    conn: &mut PoolConnection<Sqlite>,
    cols: Vec<NewColumn>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.new_column_insert_calls.inc();

    let mut builder = sqlx::QueryBuilder::new("INSERT INTO graph_registry_columns (type_id, column_position, column_name, column_type, nullable, graphql_type)");

    builder.push_values(cols.into_iter(), |mut b, new_col| {
        b.push_bind(new_col.type_id)
            .push_bind(new_col.column_position)
            .push_bind(new_col.column_name)
            .push_bind(new_col.column_type)
            .push_bind(new_col.nullable)
            .push_bind(new_col.graphql_type);
    });

    let query = builder.build();

    let result = query.execute(conn).await?;

    Ok(result.rows_affected() as usize)
}

pub async fn list_column_by_id(
    conn: &mut PoolConnection<Sqlite>,
    col_id: i64,
) -> sqlx::Result<Vec<Columns>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.list_column_by_id_calls.inc();

    sqlx::query_as!(Columns, r#"SELECT id, type_id, column_position as "column_position: i32", column_name, column_type, nullable, graphql_type FROM graph_registry_columns WHERE type_id = $1"#, col_id).fetch_all(conn).await
}

pub async fn columns_get_schema(
    conn: &mut PoolConnection<Sqlite>,
    name: &str,
    version: &str,
) -> sqlx::Result<Vec<ColumnInfo>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.columns_get_schema_calls.inc();

    sqlx::query_as!(
        ColumnInfo,
        r#"SELECT
               c.type_id as type_id,
               t.table_name as table_name,
               c.column_position as "column_position: i32",
               c.column_name as column_name,
               c.column_type as column_type
           FROM graph_registry_type_ids as t
           INNER JOIN graph_registry_columns as c
           ON t.id = c.type_id
           WHERE t.schema_name = $1
           AND t.schema_version = $2
           ORDER BY c.type_id, c.column_position"#,
        name,
        version
    )
    .fetch_all(conn)
    .await
}

pub async fn index_is_registered(
    conn: &mut PoolConnection<Sqlite>,
    namespace: &str,
    identifier: &str,
) -> sqlx::Result<Option<RegisteredIndex>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.index_is_registered_calls.inc();

    match sqlx::query_as!(
        RegisteredIndex,
        r#"SELECT id as "id!", namespace, identifier FROM index_registry WHERE namespace = $1 AND identifier = $2"#,
        namespace,
        identifier
    )
    .fetch_one(conn)
    .await
    {
        Ok(row) => Ok(Some(row)),
        Err(_e) => Ok(None),
    }
}

pub async fn register_index(
    conn: &mut PoolConnection<Sqlite>,
    namespace: &str,
    identifier: &str,
) -> sqlx::Result<RegisteredIndex> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.register_index_calls.inc();

    if let Some(index) = index_is_registered(conn, namespace, identifier).await? {
        return Ok(index);
    }

    let query = format!("INSERT INTO index_registry (namespace, identifier) VALUES ('{namespace}', '{identifier}') RETURNING *",
    );

    let row = sqlx::QueryBuilder::new(query)
        .build()
        .fetch_one(conn)
        .await?;

    let id = row.get(0);
    let namespace = row.get(1);
    let identifier = row.get(2);

    Ok(RegisteredIndex {
        id,
        namespace,
        identifier,
    })
}

pub async fn registered_indices(
    conn: &mut PoolConnection<Sqlite>,
) -> sqlx::Result<Vec<RegisteredIndex>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.registered_indices_calls.inc();

    sqlx::query_as!(RegisteredIndex, "SELECT * FROM index_registry")
        .fetch_all(conn)
        .await
}

pub async fn index_asset_version(
    conn: &mut PoolConnection<Sqlite>,
    index_id: &i64,
    asset_type: &IndexAssetType,
) -> sqlx::Result<i64> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.index_asset_version_calls.inc();

    match sqlx::query(&format!(
        "SELECT COUNT(*) FROM index_asset_registry_{} WHERE index_id = {}",
        asset_type.as_ref(),
        index_id,
    ))
    .fetch_one(conn)
    .await
    {
        Ok(row) => Ok(row.try_get::<'_, i64, usize>(0).unwrap_or(0)),
        Err(_e) => Ok(0),
    }
}

pub async fn register_index_asset(
    conn: &mut PoolConnection<Sqlite>,
    namespace: &str,
    identifier: &str,
    bytes: Vec<u8>,
    asset_type: IndexAssetType,
) -> sqlx::Result<IndexAsset> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.register_index_asset_calls.inc();

    let index = match index_is_registered(conn, namespace, identifier).await? {
        Some(index) => index,
        None => register_index(conn, namespace, identifier).await?,
    };

    let digest = sha256_digest(&bytes);

    if let Some(asset) =
        asset_already_exists(conn, &asset_type, &bytes, &index.id).await?
    {
        info!(
            "Asset({:?}) for Index({}) already registered.",
            asset_type,
            index.uid()
        );
        return Ok(asset);
    }

    let current_version = index_asset_version(conn, &index.id, &asset_type)
        .await
        .expect("Failed to get asset version.");

    let query = format!(
        "INSERT INTO index_asset_registry_{} (index_id, bytes, version, digest) VALUES ({}, $1, {}, '{}') RETURNING *",
        asset_type.as_ref(),
        index.id,
        current_version + 1,
        digest,
    );

    let row = sqlx::QueryBuilder::new(query)
        .build()
        .bind(bytes)
        .fetch_one(conn)
        .await?;

    info!(
        "Registered Asset({:?}) to Index({}).",
        asset_type,
        index.uid()
    );

    let id = row.get(0);
    let index_id = row.get(1);
    let version = row.get(2);
    let digest = row.get(3);
    let bytes = row.get(4);

    Ok(IndexAsset {
        id,
        index_id,
        version,
        digest,
        bytes,
    })
}

pub async fn latest_asset_for_index(
    conn: &mut PoolConnection<Sqlite>,
    index_id: &i64,
    asset_type: IndexAssetType,
) -> sqlx::Result<IndexAsset> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.latest_asset_for_index_calls.inc();

    let query = format!(
        "SELECT * FROM index_asset_registry_{} WHERE index_id = {} ORDER BY id DESC LIMIT 1",
        asset_type.as_ref(),
        index_id,
    );

    let row = sqlx::query(&query).fetch_one(conn).await?;

    let id = row.get(0);
    let index_id = row.get(1);
    let version = row.get(2);
    let digest = row.get(3);
    let bytes = row.get(4);

    Ok(IndexAsset {
        id,
        index_id,
        version,
        digest,
        bytes,
    })
}

pub async fn latest_assets_for_index(
    conn: &mut PoolConnection<Sqlite>,
    index_id: &i64,
) -> sqlx::Result<IndexAssetBundle> {
    #[cfg(feature = "metrics")]
    METRICS.db.postgres.latest_assets_for_index_calls.inc();

    let wasm = latest_asset_for_index(conn, index_id, IndexAssetType::Wasm)
        .await
        .expect("Failed to retrieve wasm asset.");
    let schema = latest_asset_for_index(conn, index_id, IndexAssetType::Schema)
        .await
        .expect("Failed to retrieve schema asset.");
    let manifest = latest_asset_for_index(conn, index_id, IndexAssetType::Manifest)
        .await
        .expect("Failed to retrieve manifest asset.");

    Ok(IndexAssetBundle {
        wasm,
        schema,
        manifest,
    })
}

pub async fn asset_already_exists(
    conn: &mut PoolConnection<Sqlite>,
    asset_type: &IndexAssetType,
    bytes: &Vec<u8>,
    index_id: &i64,
) -> sqlx::Result<Option<IndexAsset>> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.asset_already_exists_calls.inc();

    let digest = sha256_digest(bytes);

    let query = format!(
        "SELECT * FROM index_asset_registry_{} WHERE index_id = {} AND digest = '{}'",
        asset_type.as_ref(),
        index_id,
        digest
    );

    match sqlx::QueryBuilder::new(query).build().fetch_one(conn).await {
        Ok(row) => {
            let id = row.get(0);
            let index_id = row.get(1);
            let version = row.get(2);
            let digest = row.get(3);
            let bytes = row.get(4);

            Ok(Some(IndexAsset {
                id,
                index_id,
                version,
                digest,
                bytes,
            }))
        }
        Err(_e) => Ok(None),
    }
}

pub async fn index_id_for(
    conn: &mut PoolConnection<Sqlite>,
    namespace: &str,
    identifier: &str,
) -> sqlx::Result<i64> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.index_id_for_calls.inc();

    let query = format!(
        "SELECT id FROM index_registry WHERE namespace = '{namespace}' AND identifier = '{identifier}'",
    );

    let row = sqlx::query(&query).fetch_one(conn).await?;

    let id: i64 = row.get(0);

    Ok(id)
}

pub async fn start_transaction(conn: &mut PoolConnection<Sqlite>) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.start_transaction_calls.inc();

    execute_query(conn, "BEGIN".into()).await
}

pub async fn commit_transaction(
    conn: &mut PoolConnection<Sqlite>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.commit_transaction_calls.inc();

    execute_query(conn, "COMMIT".into()).await
}

pub async fn revert_transaction(
    conn: &mut PoolConnection<Sqlite>,
) -> sqlx::Result<usize> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.revert_transaction_calls.inc();

    execute_query(conn, "ROLLBACK".into()).await
}

pub async fn remove_index(
    conn: &mut PoolConnection<Sqlite>,
    namespace: &str,
    identifier: &str,
) -> sqlx::Result<()> {
    #[cfg(feature = "metrics")]
    METRICS.db.sqlite.remove_index.inc();

    let index_id = index_id_for(conn, namespace, identifier).await?;

    execute_query(
        conn,
        format!("DELETE FROM index_asset_registry_wasm WHERE index_id = {index_id}",),
    )
    .await?;

    execute_query(
        conn,
        format!("DELETE FROM index_asset_registry_manifest WHERE index_id = {index_id}",),
    )
    .await?;

    execute_query(
        conn,
        format!("DELETE FROM index_asset_registry_schema WHERE index_id = {index_id}",),
    )
    .await?;

    execute_query(
        conn,
        format!("DELETE FROM index_registry WHERE id = {index_id}"),
    )
    .await?;

    Ok(())
}
