#!/bin/bash -exu

pushd ..
sudo docker compose down
sudo docker compose up -d
popd

sqlx migrate run
sea generate entity -o ./src/db/seaorm/entities/
