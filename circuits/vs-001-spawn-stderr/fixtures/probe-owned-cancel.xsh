# ARGV: xsh child token ready-marker completion-marker stdout stderr
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let ready = fp"${ARGV[3]}"
let completed = fp"${ARGV[4]}"
let stdout = fp"${ARGV[5]}"
let stderr = fp"${ARGV[6]}"

let command = process.command_argv(
  xsh,
  [xsh, child, token, ready.display(), completed.display()],
  stdout: stdout,
  stderr: stderr,
)
let handle = spawn command?
var observed = false
for _ in range(500) {
  if ready.exists()? {
    observed = true
    break
  }
  time.sleep(10ms)?
}
if ! observed {
  abort(78)
}
handle.cancel(signal: "TERM", kill_after: 0ms)?
time.sleep(400ms)?
if completed.exists()? {
  abort(79)
}
print "owned-cancel:returned"
