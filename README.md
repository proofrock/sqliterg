# 🌿 Introduction

> *This is a rewrite in Rust of [ws4sqlite](https://github.com/proofrock/ws4sqlite), 30-50% faster, 10x less memory used, more flexible in respect to sqlite support. It is not a direct rewrite, more like a "sane" (I hope) redesign. You can read more about what's changed and how to migrate [here](https://docs.sqliterg.dev/features/migrating-from-ws4sqlite).*

**sqliterg** is a server-side application that, applied to one or more SQLite files, allows to perform SQL queries and statements on them via REST (or better, JSON over HTTP).

Full docs are available [here](https://docs.sqliterg.dev/) and a [tutorial](https://docs.sqliterg.dev/tutorial) too.

Possible use cases are the ones where remote access to a sqlite db is useful/needed, for example a data layer for a remote application, possibly serverless or even called from a web page ([*after security considerations*](https://docs.sqliterg.dev/security) of course).

As a quick example, after launching:

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

# 🎞️ Features

- A [**single executable file**](https://docs.sqliterg.dev/documentation/installation) (written in Rust);
- Can be [built](https://docs.sqliterg.dev/building-and-testing#supported-platforms) either against the system's SQLite or embedding one;
- HTTP/JSON access;
- Directly call `sqliterg` on a database (as above), many options available using a YAML companion file;
- [**In-memory DBs**](https://docs.sqliterg.dev/documentation/running#file-based-and-in-memory) are supported;
- Serving of [**multiple databases**](https://docs.sqliterg.dev/documentation/configuration-file) in the same server instance;
- Named or positional parameters in SQL are supported;
- [**Batching**](https://docs.sqliterg.dev/documentation/requests#batch-parameter-values-for-a-statement) of multiple value sets for a single statement;
- All queries of a call are executed in a [**transaction**](https://docs.sqliterg.dev/documentation/requests);
- For each query/statement, specify if a failure should rollback the whole transaction, or the failure is [**limited**](https://docs.sqliterg.dev/documentation/errors#managed-errors) to that query;
- "[**Stored Statements**](https://docs.sqliterg.dev/documentation/stored-statements)": define SQL in the server, and call it from the client;
- "[**Macros**](https://docs.sqliterg.dev/documentation/macros)": lists of statements that can be executed at db creation, at startup, periodically or calling a web service;
- [**Backups**](https://docs.sqliterg.dev/documentation/backup), rotated and also runnable at db creation, at startup, periodically or calling a web service;
- [**CORS**](https://docs.sqliterg.dev/documentation/configuration-file#corsorigin) mode, configurable per-db;
- [**Journal Mode**](https://docs.sqliterg.dev/documentation/configuration-file#journalmode) (e.g. WAL) can be configured;
- [**Embedded web server**](https://docs.sqliterg.dev/documentation/web-server) to directly serve web pages that can access `sqliterg` without CORS;
- [**Quite fast**](https://docs.sqliterg.dev/features/performances)!
- Comprehensive [**test suite**](https://docs.sqliterg.dev/building-and-testing#testing);
- [**Docker images**](https://docs.sqliterg.dev/documentation/installation/docker), for x86_64 and arm64;
- Binaries are provided with a bundled SQLite "inside" them, or linked against the system's installed SQLite.

### Security Features

* [**Authentication**](https://docs.sqliterg.dev/security#authentication) can be configured
  * on the client, either using HTTP Basic Authentication or specifying the credentials in the request;
  * on the server, either by specifying credentials (also with hashed passwords) or providing a query to look them up in the db itself;
  * customizable `Not Authorized` error code (if `401` is not optimal);
* A database can be opened in [**read-only mode**](https://docs.sqliterg.dev/security#read-only-databases) (only queries will be allowed);
* It's possible to enforce using [**only stored statements**](https://docs.sqliterg.dev/security#stored-statements-to-prevent-sql-injection), to avoid some forms of SQL injection and receiving SQL from the client altogether;
* [**CORS/Allowed Origin**](https://docs.sqliterg.dev/security#cors-allowed-origin) can be configured and enforced;
* It's possible to [**bind**](https://docs.sqliterg.dev/security#binding-to-a-network-interface) to a network interface, to limit access.

Some design choices:

* Very thin layer over SQLite. Errors and type translation, for example, are those provided by the SQLite driver;
* Doesn't include HTTPS, as this can be done easily (and much more securely) with a [reverse proxy](https://docs.sqliterg.dev/security#use-a-reverse-proxy-if-going-on-the-internet).

# 🥇 Credits

Kindly supported by [JetBrains for Open Source development](https://www.jetbrains.com/community/opensource/?utm_campaign=opensource&utm_content=approved&utm_medium=email&utm_source=newsletter&utm_term=jblogo#support).
