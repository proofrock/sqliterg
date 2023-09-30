# 🌿 Introduction & Credits

*This is a rewrite in Rust of [ws4sqlite](https://github.com/proofrock/ws4sqlite), 30-50% faster, 10x less memory used, more flexible in respect to sqlite support. It is not a direct rewrite, more like a "sane" (I hope) [redesign](https://github.com/proofrock/sqliterg/blob/main/CHANGES_FROM_WS4SQLITE.md).*

**sqliterg** is a server-side application that, applied to one or more SQLite files, allows to perform SQL queries and statements on them via REST (or better, JSON over HTTP).

Possible use cases are the ones where remote access to a sqlite db is useful/needed, for example a data layer for a remote application, possibly serverless or even called from a web page (_after security considerations_ of course).

As a quick example, after launching

```bash
sqliterg --db mydatabase.db
```

It's possible to make a POST call to `http://localhost:12321/mydatabase`, e.g. with the following body:

```json
{
    "transaction": [
        {
            "statement": "INSERT INTO TEST_TABLE (ID, VAL, VAL2) VALUES (:id, :val, :val2)",
            "values": { "id": 1, "val": "hello", "val2": null }
        },
        {
            "query": "SELECT * FROM TEST_TABLE"
        }
    ]
}
```

Obtaining an answer of:

```json
{
    "results": [
        {
            "success": true,
            "rowsUpdated": 1
        },
        {
            "success": true,
            "resultSet": [
                { "ID": 1, "VAL": "hello", "VAL2": null }
            ]
        }
    ]
}
```

# 🥇 Features

- A [**single executable file**](https://germ.gitbook.io/sqliterg/documentation/installation) (written in Rust);
- Can be built against the system's SQLite or embedding one (thanks to [rusqlite](https://docs.rs/rusqlite/latest/rusqlite/));
- HTTP/JSON access;
- Directly call `sqliterg` on a database (as above), many options available using a YAML companion file;
- [**In-memory DBs**] are supported (https://germ.gitbook.io/sqliterg/documentation/configuration-file#path);
- Serving of [**multiple databases**](https://germ.gitbook.io/sqliterg/documentation/configuration-file) in the same server instance;
- [**Batching**](https://germ.gitbook.io/sqliterg/documentation/requests#batch-parameter-values-for-a-statement) of multiple value sets for a single statement;
- All queries of a call are executed in a [**transaction**](https://germ.gitbook.io/sqliterg/documentation/requests);
- For each query/statement, specify if a failure should rollback the whole transaction, or the failure is [**limited**](https://germ.gitbook.io/sqliterg/documentation/errors#managed-errors) to that query;
- "[**Stored Statements**](https://germ.gitbook.io/sqliterg/documentation/stored-statements)": define SQL in the server, and call it from the client;
- "[**Macros**](https://germ.gitbook.io/sqliterg/documentation/macros)": lists of statements that can be executed at db creation, at startup, periodically or calling a web service;
- **Backups**, rotated and also runnable at db creation, at startup, periodically or calling a web service;
- [**CORS**](https://germ.gitbook.io/sqliterg/documentation/configuration-file#corsorigin) mode, configurable per-db;
- [**Journal Mode**](https://sqlite.org/wal.html) (e.g. WAL) can be configured;
- [**Embedded web server**](https://germ.gitbook.io/sqliterg/documentation/web-server) to directly serve web pages that can access sqliterg without CORS;- [Quite fast](features/performances.md)!
- Comprehensive test suite (`make test`);

### Security Features

* [**Authentication**](https://germ.gitbook.io/sqliterg/documentation/security.md#authentication) can be configured
  * on the client, either using HTTP Basic Authentication or specifying the credentials in the request;
  * on the server, either by specifying credentials (also with hashed passwords) or providing a query to look them up in the db itself;
  * customizable `Not Authorized` error code (if 401 is not optimal)
* A database can be opened in [**read-only mode**](https://germ.gitbook.io/sqliterg/documentation/security.md#read-only-databases) (only queries will be allowed);
* It's possible to enforce using [**only stored statements**](https://germ.gitbook.io/sqliterg/documentation/security.md#stored-statements-to-prevent-sql-injection), to avoid some forms of SQL injection and receiving SQL from the client altogether;
* [**CORS Allowed Origin**](https://germ.gitbook.io/sqliterg/documentation/security.md#cors-allowed-origin) can be configured and enforced;
* It's possible to [**bind**](https://germ.gitbook.io/sqliterg/documentation/security.md#binding-to-a-network-interface) to a network interface, to limit access.

Some design choices:

* Very thin layer over SQLite. Errors and type translation, for example, are those provided by the SQLite driver;
* Doesn't include HTTPS, as this can be done easily (and much more securely) with a [reverse proxy](https://germ.gitbook.io/sqliterg/documentation/security.md#use-a-reverse-proxy-if-going-on-the-internet);
