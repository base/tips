# Local troubleshooting

This page lists common issues when running TIPS locally and quick ways to resolve them.

## Docker compose: ports already in use

Symptoms:
- `bind: address already in use`
Fix:
- Stop conflicting containers or change the mapped ports in `docker-compose*.yml`.

## Postgres connection failures

Symptoms:
- application logs show `connection refused` or auth errors
Fix:
- Confirm the DB container is healthy.
- Ensure `.env` values match the compose service settings.
- If you changed credentials, recreate containers:
  - `docker compose down -v`
  - `docker compose up -d`

## Migrations / schema mismatch

Symptoms:
- runtime errors on missing columns / tables
Fix:
- Run the project’s migration step (if provided), or reset the local database volume.

## Kafka not reachable

Symptoms:
- timeouts when publishing/consuming events
Fix:
- Verify Kafka is up and the bootstrap URL matches your local config.
- Restart Kafka + dependent services if needed.

## UI not updating

Symptoms:
- UI loads but shows stale data
Fix:
- Hard refresh the browser.
- Verify the backend services are running and returning fresh responses.
