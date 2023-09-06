- Add descriptive error messages on parsing
- Complete startup messages (for macros at init, ecc.)
- Distinguish between 4xx and 5xx in responses
- If there's a Values and a ValuesBatch, ValuesBatch "wins"/gives an error

# To doc

- A macro's statements are executed in a transaction
- Init macros and startup macros are executed in a general wrapper transaction, to be able to revert them
- If an init macro fails, the db is deleted
- If both password and hashedPassword are specified, password "wins"
- If there's a Values and a ValuesBatch, ValuesBatch "wins"/gives an error

# Test

- CORS
- If there's a Values and a ValuesBatch, ValuesBatch "wins"/gives an error
