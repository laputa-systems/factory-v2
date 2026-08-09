# ARGV: xsh child token marker stdout
# `/dev/null` is supplied through the same typed Path field as a regular file.
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stdout = fp"${ARGV[4]}"
let null_sink = fp"/dev/null"

let command = process.command_argv(
  xsh,
  [xsh, child, token, marker.display()],
  stdout: stdout,
  stderr: null_sink,
)
let status = process.run(command)?
if ! status.exited_with(0) {
  abort(81)
}
print "process-run-null:waited"
