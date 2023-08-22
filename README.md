# rust-gitlab-search-api

Rust implementation of gitlab search api.

Currently it support serch in groups, search project by id, and search by project name.

This code can be used as cli or lib, currently cli only supported group ids search.

cli: 

```
cargo run -- -u <url> -t <token> -v -g <group_id_1> -g <group_id_2> -q <search_term>
```
or 
```
cargo build --release
cd target/release
./rust-gitlab-search-api -- -u <url> -t <token> -v -g <group_id_1> -g <group_id_2> -q <search_term>
```

see main.rs for library usage.
