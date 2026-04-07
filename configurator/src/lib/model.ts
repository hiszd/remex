import type { DateTime } from "surrealdb";

export interface ClientData {
  client_name: String;
  secret: String;
  hardware_hash: String;
  created_at: DateTime;
  updated_at: DateTime;
}

export interface Client {
  client_name: String;
  secret: String;
  hardware_hash: String;
  created_at: Date;
  updated_at: Date;
}
