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
let _detached = process.spawn(command)?
var observed = false
for _ in range(500) {
  if marker.exists()? {
    observed = true
    break
  }
  time.sleep(10ms)?
}
if ! observed {
  abort(73)
}
print "detached-spawn:started"
