# Program Registrey

## run migration

```sh
sqlx migrate run
```

## /upload-program

```sh
curl --location 'http://127.0.0.1:3000/upload-program' \
--form 'program=@"./hdp.json"' \
--form 'version="0"'
```

## /get-program

```sh
curl --location --request GET 'http://127.0.0.1:3000/get-program?program_hash=0x343995a543ac64616d33fa77670cfa4e498691c96c2d964a0a07181dff4ce81' \
--header 'Content-Type: application/json'
```
