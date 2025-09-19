![Base](./docs/logo.png)

# TIPS - Transaction Inclusion & Prioritization Stack

> [!WARNING]
> This repository is an experiment to enable bundles, transaction simulation and transaction tracing for Base. 
> It's being used to explore ideas and experiment. It is currently not production ready.

## Architecture Overview

The project consists of several components:

### 🗄️ Datastore (`crates/datastore`)
Postgres storage layer that provides API's to persist and retrieve bundles.