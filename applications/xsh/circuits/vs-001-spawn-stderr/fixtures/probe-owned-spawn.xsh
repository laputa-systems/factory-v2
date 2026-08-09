# ARGV: xsh child token marker stdout stderr
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stdout = fp"${ARGV[4]}"
let stderr = fp"${ARGV[5]}"

let command = process.command_argv(
  xsh,
  [xsh, child, token, marker.display()],
  stdout: stdout,
  stderr: stderr,
)
let handle = spawn command?
let status = wait handle?
if ! status.exited_with(0) {
  abort(71)
}
print "owned-spawn:waited"
