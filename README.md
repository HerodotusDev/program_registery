# Program Registery

The Program Registry is a server that accepts compiled Cairo programs (also known as CASM files), fetches the compiler version dynamically, computes the correct program hash, and stores the bytecode of the Cairo program along with the program hash.

The program hash is a unique calculation of the program bytecode. Based on the compiler version, the computation might vary slightly. You can also retrieve the exact hash via the `cairo-run` command from [cairo-lang](https://github.com/starkware-libs/cairo-lang).

Our main motivation is to have a registry of Cairo programs so that we can retrieve the compiled Cairo program using the given program hash.

## Quick start (local)

### 1. environment variable

Make sure to have `DATABASE_URL` on `.env` file.

### 2. run migration

```sh
sqlx migrate run
```

### 3. run program

```sh
cargo run -r
```

## /upload-program

```sh
curl --location 'http://127.0.0.1:3000/upload-program' \
--form 'program=@"./hdp.json"' \
--form 'version="0"'
```

response:

```sh
0x343995a543ac64616d33fa77670cfa4e498691c96c2d964a0a07181dff4ce81
```

## /get-program

```sh
curl --location --request GET 'http://127.0.0.1:3000/get-program?program_hash=0x343995a543ac64616d33fa77670cfa4e498691c96c2d964a0a07181dff4ce81' \
--header 'Content-Type: application/json'
```
