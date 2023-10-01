To test CORS, setup two sqliterg like this (suppose to be in project's base dir):

```bash
sqliterg --mem-db test::tests/cors/test.yaml &
sqliterg --serve-dir tests/cors/ --port 12322 --index-file index.html &
```

Then visit `http://localhost:12322` with a browser; in the network debugger you should find that the OPTIONS call (preflight) and the POST call are successful.