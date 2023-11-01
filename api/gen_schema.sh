#!/bin/bash -exu

pushd ..
docker compose down
docker compose up -d
popd

sqlx migrate run
sea generate entity -o ./src/db/seaorm/entities/
