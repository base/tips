import {
  bigint,
  char,
  pgTable,
  text,
  timestamp,
  uuid,
} from "drizzle-orm/pg-core";

export const bundles = pgTable("bundles", {
  id: uuid().primaryKey().notNull(),
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
