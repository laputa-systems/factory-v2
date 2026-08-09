# Deliberately prohibited: it proves why byte-level redirection alone is not
# sufficient evidence for VS-001. This fixture is inspected, never executed.
let command = process.command_argv(
  "sh",
  ["sh", "-c", "exec \"$@\" 2> \"$1\"", "--", "child", "stderr.log"],
)
process.run(command)?
