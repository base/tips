import {
  bigint,
  char,
  foreignKey,
  index,
  jsonb,
  pgEnum,
  pgTable,
  text,
  timestamp,
  unique,
  uuid,
} from "drizzle-orm/pg-core";

export const bundleState = pgEnum("bundle_state", [
  "Ready",
  "BundleLimit",
  "AccountLimits",
  "GlobalLimits",
  "IncludedInFlashblock",
  "IncludedInBlock",
]);

export const bundles = pgTable("bundles", {
  id: uuid().primaryKey().notNull(),
  state: bundleState().notNull(),
  senders: char({ length: 42 }).array(),
  // You can use { mode: "bigint" } if numbers are exceeding js number limitations
  minimumBaseFee: bigint("minimum_base_fee", { mode: "number" }),
  txnHashes: char("txn_hashes", { length: 66 }).array(),
  txs: text().array().notNull(),
  revertingTxHashes: char("reverting_tx_hashes", { length: 66 }).array(),
  droppingTxHashes: char("dropping_tx_hashes", { length: 66 }).array(),
  // You can use { mode: "bigint" } if numbers are exceeding js number limitations
  blockNumber: bigint("block_number", { mode: "number" }),
  // You can use { mode: "bigint" } if numbers are exceeding js number limitations
  minTimestamp: bigint("min_timestamp", { mode: "number" }),
  // You can use { mode: "bigint" } if numbers are exceeding js number limitations
  maxTimestamp: bigint("max_timestamp", { mode: "number" }),
  createdAt: timestamp("created_at", {
    withTimezone: true,
    mode: "string",
  }).notNull(),
  updatedAt: timestamp("updated_at", {
    withTimezone: true,
    mode: "string",
  }).notNull(),
});

export const simulations = pgTable(
  "simulations",
  {
    id: uuid().primaryKey().notNull(),
    bundleId: uuid("bundle_id").notNull(),
    // You can use { mode: "bigint" } if numbers are exceeding js number limitations
    blockNumber: bigint("block_number", { mode: "number" }).notNull(),
    blockHash: char("block_hash", { length: 66 }).notNull(),
    // You can use { mode: "bigint" } if numbers are exceeding js number limitations
    executionTimeUs: bigint("execution_time_us", { mode: "number" }).notNull(),
    // You can use { mode: "bigint" } if numbers are exceeding js number limitations
    gasUsed: bigint("gas_used", { mode: "number" }).notNull(),
    stateDiff: jsonb("state_diff").notNull(),
    createdAt: timestamp("created_at", {
      withTimezone: true,
      mode: "string",
    }).notNull(),
    updatedAt: timestamp("updated_at", {
      withTimezone: true,
      mode: "string",
    }).notNull(),
  },
  (table) => [
    index("idx_simulations_block_hash").using(
      "btree",
      table.blockHash.asc().nullsLast().op("bpchar_ops"),
    ),
    index("idx_simulations_block_number").using(
      "btree",
      table.blockNumber.asc().nullsLast().op("int8_ops"),
    ),
    index("idx_simulations_bundle_id").using(
      "btree",
      table.bundleId.asc().nullsLast().op("uuid_ops"),
    ),
    foreignKey({
      columns: [table.bundleId],
      foreignColumns: [bundles.id],
      name: "simulations_bundle_id_fkey",
    }).onDelete("cascade"),
    unique("simulations_bundle_id_block_hash_key").on(
      table.bundleId,
      table.blockHash,
    ),
  ],
);
