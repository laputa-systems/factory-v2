# This only constructs a typed command, then fabricates fixture-specific bytes.
# It deliberately never starts the child and therefore has no owned lifecycle.
# ARGV: assigned-xsh child-script error-log child-marker
let xsh = ARGV[0]
let child = ARGV[1]
let stderr = fp"${ARGV[2]}"
let marker = ARGV[3]
let command = process.command_argv(xsh, [xsh, child, marker])
stderr.write("stderr:alpha\n")?
print "stdout:alpha"
