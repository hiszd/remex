import type { Plugin } from "@opencode-ai/plugin"

export const DocLintGuard: Plugin = async (ctx) => {
  return {
    "file.edited": async (input: any, _output: any) => {
      const file: string | undefined = input?.path ?? input?.file ?? input?.uri
      if (!file || !file.endsWith(".md")) {
        return
      }

      try {
        await Bun.$`cd ${ctx.worktree} && cargo xtask doc-lint -- ${file}`.text()
      } catch (error) {
        const stdout = (error as any)?.stdout ?? ""
        const stderr = (error as any)?.stderr ?? ""
        await ctx.client.tui.showToast({
          body: {
            message: `Doc lint issues in ${file}: ${stdout || stderr || "see linter output"}`,
            variant: "warning",
          },
        })
      }
    },
  }
}
