# This can redirect bytes, but process.spawn returns the forbidden detached record.
let xsh = ARGV[0]
let child = ARGV[1]
let stderr = fp"${ARGV[2]}"
let command = process.command_argv(xsh, [xsh, child], stderr: stderr)
let spawned = process.spawn(command)?
