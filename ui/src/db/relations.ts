import { relations } from "drizzle-orm/relations";
import { bundles, simulations } from "./schema";

export const simulationsRelations = relations(simulations, ({ one }) => ({
  bundle: one(bundles, {
    fields: [simulations.bundleId],
    references: [bundles.id],
  }),
}));

export const bundlesRelations = relations(bundles, ({ many }) => ({
  simulations: many(simulations),
}));
