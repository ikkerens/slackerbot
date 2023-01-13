#!/bin/sh
cd ./migration/
cargo run -- fresh
cd ..
sea-orm-cli generate entity -o entity/src/entities
