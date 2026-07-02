import { tool } from "@opencode-ai/plugin"

export default tool({
  description: "Execute a SurrealQL query or command against the local/development SurrealDB instance to inspect tables, schemas, or record structures.",
  args: {
    query: tool.schema.string().describe("The exact SurrealQL statement to run (e.g., 'INFO FOR DB;', 'SELECT * FROM user LIMIT 5;')"),
  },
  async execute(args, context) {
    try {
      // Leverages Bun's native shell execution to pass the query safely to the surreal CLI
      const result = await Bun.$`surreal sql --endpoint http://localhost:8000 --namespace test --database test ${args.query}`.text();

      return result;
    } catch (error) {
      return `Database execution failed: ${error instanceof Error ? error.message : String(error)}`;
    }
  },
})
