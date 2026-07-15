import { tool } from "@opencode-ai/plugin"

export default tool({
  description:
    "Run the documentation linter. Use after modifying any documentation to catch factual drift, stale links, banned terms, or code/doc mismatches.",
  args: {
    paths: tool.schema
      .array(tool.schema.string())
      .optional()
      .describe("Specific files or directories to lint; defaults to all Markdown docs"),
    verbose: tool.schema
      .boolean()
      .optional()
      .describe("Emit verbose output from the linter"),
  },
  async execute(args, context) {
    const flags = args.verbose ? ["--verbose"] : []
    const paths = args.paths ?? []

    try {
      const result = await Bun.$`cd ${context.worktree} && cargo xtask doc-lint ${flags} ${paths}`.text()
      return result
    } catch (error) {
      const stdout = (error as any)?.stdout ?? ""
      const stderr = (error as any)?.stderr ?? ""
      return `Doc lint failed:\n${stdout}\n${stderr}`
    }
  },
})
