# Supermarket Tracker

Tracks supermarket prices across New Zealand online supermarkets.

### Starting the application

- Make sure to be running a Postgres instance with a blank `supermarket_tracker` database created.
  - The app will initialize all tables for you.

### Usage

```
supermarket-tracker

USAGE:
    supermarket-tracker [OPTIONS] [SUBCOMMAND]

OPTIONS:
    --no-insert				Skips insertion of new products/prices to database
```

### Architecture

Core application is written in Rust.
