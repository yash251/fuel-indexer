# Types

Below is a mapping of GraphQL schema types to their Postgres equivalents, referencing [Postgres 14](https://www.postgresql.org/docs/14/datatype.html) data types.

| Sway Type | GraphQL Schema Type | Postgres Type |
|------|----------|----------|
| u64 | ID | bigint primary key |
| b256 | Address | varchar(64) |
| str[4] | Bytes4 | varchar(16) |
| str[8] | Bytes8 | varchar(64) |
| str[32] | Bytes32 | varchar(64) |
| str[32] | AssetId | varchar(64) |
| b256 | ContractId | varchar(64) |
| str[32] | Salt | varchar(64) |
| u32 | UInt4 | integer |
| u64 | UInt8 | bigint |
| i64 | Timestamp | timestamp |
| str[] | Blob | bytes |
| str[32] | MessageId | varchar(64) |
| bool | Boolean | bool |
|  | Json | json |
|  | Charfield | varchar(255) |
|  | Blob | varchar(10485760) |

## Example

Let's define an `Event` struct in a Sway contract:

```sway
struct Event {
    id: u64,
    address: Address,
    block_height: u64,
}
```

The corresponding GraphQL schema to mirror this `Event` struct would resemble:

```graphql
type Event {
    id: ID!
    account: Address!
    block_height: UInt8!
}
```

And finally, this GraphQL schema will generate the following Postgres schema:

```text
                                           Table "schema.event"
    Column   |     Type    | Collation | Nullable | Default | Storage  | Compression | Stats target | Description
--------------+-------------+-----------+----------+---------+----------+-------------+--------------+-------------
 id           |    bigint   |           | not null |         | plain        |             |              |
 block_height |    bigint   |           | not null |         | plain    |             |              |
 address      | varchar(64) |           | not null |         | plain    |             |              |
 object       |    bytea    |           | not null |         | extended |             |              |
Indexes:
    "event_pkey" PRIMARY KEY, btree (id)
Access method: heap
```
