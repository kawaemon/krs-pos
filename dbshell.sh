#!/bin/sh
set -exu
docker exec -it pos-postgres psql -U db pos
