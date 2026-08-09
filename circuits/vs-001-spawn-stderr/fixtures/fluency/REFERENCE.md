# Assigned process reference

Use `process.command_argv` with the assigned XSH executable as both command
target and first argv entry. Give only the command's `stderr` field the supplied
typed `Path` destination. `spawn command?` creates the owned handle and
`wait handle?` yields a `Status`; a completed nonzero status is data, so the
supervisor must exit nonzero after waiting.

The task deliberately does not use `process.spawn`: that API is a detached
record surface and cannot satisfy the owned-handle requirement.
