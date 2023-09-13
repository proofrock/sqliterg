- Complete startup messages (for macros at init, ecc.)
- Comments
- Documentation

# To doc

- A macro's statements are executed in a transaction
- Init macros and startup macros are executed in a general wrapper transaction, to be able to revert them
- If an init macro fails, the db is deleted
- If both password and hashedPassword are specified, password "wins"
- If there's a Values and a ValuesBatch, it gives an error
- There's no protection from managing your own transaction; please be careful,
  and a commit/rollback is always done at the end
- HTTP Codes (in deciding order):
  - 500: error from SQLite
  - 409: mismatch with configuration (e.g. reference a stored statement that it's not there)
  - 400: request is "wrong"

# Test

- CORS
- ~
