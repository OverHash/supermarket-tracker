# Supermarket Tracker

Tracks supermarket prices across New Zealand online supermarkets.

## Starting the application

- Setup relevant environment variables (a `.env` file can be used for convenience, see [`.env.example`](env.example))
- Run a Postgres instance with a blank `supermarket_tracker` database created (e.g., with `docker compose up -d`)
  - The app will initialize all tables for you when first run

## CLI Usage

```
supermarket-tracker

Usage:
    supermarket-tracker [OPTIONS] [SUBCOMMAND]

Options:
    --supermarket <SUPERMARKET>     The supermarket to run price tracking on [countdown]
    --no-insert                     Optionally skips insertion of new products/prices to database
```

### Architecture

Core application is written in Rust. Read more in the [ARCHITECTURE.md](./ARCHITECTURE.md) document.

### Using with Docker

[Docker](https://www.docker.com/) can be used to host the Postgres database, and perform all the initial work of setup.

To use, ensure you have a `.env` file (if you don't, simply `cp .env.example .env`) and run `docker compose up -d` to start the services. This will expose a Postgres database on port 5432 of the host machine.

To stop, use `docker compose down` to stop all the containers, and `docker compose down --volumes` to delete the volumes as well.

### How much data is tracked?

I currently have around ~800,000 price points from ~23,000 products tracked since October 2022.

If this data would be of use, please [contact me](https://x.com/OverHashDev). I have mostly used it for my own fun statistical analysis,
and comparing data against what Stats NZ produces.

### Migrating Docker Containers Between Hosts

I've had some trouble finding resources for this online, so I thought posting these instructions would be helpful.

Replace `$CONTAINER_NAME`, `$DATABASE_NAME` and `$DOCKER_FILENAME` accordingly for your system.

1. On machine A, run `docker exec -it $CONTAINER_NAME bash`
2. Once in a bash terminal in the docker container, run `pg_dumpall -c -U postgres | gzip > ./tmp/dump_$(date +"%Y-%m-%d_%H_%M_%S").gz`
3. Verify file looks good in the docker container, and note the location of it (noted as `$DOCKER_FILENAME`).
4. Transfer from docker container to host with `docker cp $CONTAINER_NAME:/tmp/$DOCKER_FILENAME /tmp/$DOCKER_FILENAME`
5. Now transfer the file over to the host, using whatever means you prefer.
6. We now use `$CONTAINER_NAME` to refer to the fresh postgres instance created. This instance must have an empty `$DATABASE_NAME` created (e.g., with `docker compose up -d`).
7. Use `docker cp ./$DOCKER_FILENAME$ $CONTAINER_NAME$:/tmp/$DOCKER_FILENAME$`
8. `docker exec -it $CONTAINER_NAME bash`
9. `gunzip /tmp/$DOCKER_FILENAME | psql -U postgres -d $DATABASE_NAME`
10. All done!

Cheers to https://stackoverflow.com/questions/24718706/backup-restore-a-dockerized-postgresql-database for the suggestions.
