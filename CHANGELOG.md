# v0.18.0 - 4 December 2023

- [Issue #1] Implement positional parameters for SQL;
- [Issue #2] Consider loading of sqlite extension on startup. (or load_extension statements in macro);
- Library updates.

# v0.17.1 - 3 October 2023

- Aligned to [the documentation](https://docs.sqliterg.dev) (for the first time);
- For backup and macros web services, the `Content-Type` header is not needed;
- At authentication failures, wait 1 second;
- Check for non-empty statement list in macros;
- Check that the backup directory is not the same as the database file;
- Library updates.

# v0.17.0 - 1 October 2023

First version
