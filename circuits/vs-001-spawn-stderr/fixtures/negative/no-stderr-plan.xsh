# Deliberately omits `stderr:`. It must fail a redirect expectation.
# ARGV: xsh child token marker stdout
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stdout = fp"${ARGV[4]}"

let command = process.command_argv(
  xsh,
  [xsh, child, token, marker.display()],
  stdout: stdout,
)
let status = process.run(command)?
if ! status.exited_with(0) {
  abort(80)
}
print "negative-no-stderr:waited"
