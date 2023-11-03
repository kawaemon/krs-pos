from rust as base
workdir /app

run curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
run cargo binstall cargo-chef

# --
from base as plan

copy . .
workdir /app/api
run cargo chef prepare --recipe-path plan.json

# --

from base as build

# わざとデバッグビルドで行きます。プログラムにそこまで自信がないので debug-assertions 等々がほしい
workdir /app/api
copy --from=plan /app/api/plan.json .
run cargo chef cook --recipe-path plan.json

workdir /app
copy . .
run cd api && cargo build

# --

from debian:slim
workdir /app

run cargo binstall sqlx-cli

copy web .
copy api/migrations .
copy --from=build /app/api/target/debug/api .
