# ARGV: xsh nonzero-child stderr-path
let xsh = ARGV[0]
let child = ARGV[1]
let stderr = fp"${ARGV[2]}"
let command = process.command_argv(
  xsh,
  [xsh, child],
  stderr: stderr,
)
let handle = spawn command?
let status = wait handle?
if ! status.exited_with(23) {
  abort(77)
}
print "owned-nonzero:status-23"
