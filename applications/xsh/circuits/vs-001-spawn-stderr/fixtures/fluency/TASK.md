# Paired XSH fluency task

Write `supervise.xsh`. It receives paths to an XSH binary, a child XSH script,
and an error-log destination. Start the child concurrently through XSH's typed
process API, redirect only child stderr to the destination, preserve child
stdout, wait through an owned handle, and propagate a nonzero child status.

Do not invoke a shell, construct a shell command string, use `process.spawn`,
or fake the child's output. The evaluator varies paths, output, prior log
contents, and exit status.

Only this task, `REFERENCE.md`, the assigned `bin/` tools, and `fixtures/` are
in the actor workspace. There is no XSH source checkout.
