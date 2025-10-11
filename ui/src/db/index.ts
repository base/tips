import { drizzle } from "drizzle-orm/node-postgres";
import { Pool } from "pg";
import * as schema from "./schema";

const pool = new Pool({
  connectionString: process.env.TIPS_DATABASE_URL,
  ssl: {
    requestCert: false,
    rejectUnauthorized: false,
  },
});

export const db = drizzle(pool, { schema });
