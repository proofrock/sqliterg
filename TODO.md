- Auth for macros
- CORS
- Transient connections (at db level)
- Backups (https://host:port/backup/db)
- Add descriptive error messages on parsing
- Tests!

# To doc

- a macro's statements are executed in a transaction
- init macros and startup macros are executed in a general wrapper transaction, to be able to revert them
- if an init macro fails, the db is deleted