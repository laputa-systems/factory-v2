# ARGV: xsh child token marker stdout stderr append(bool text)
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stdout = fp"${ARGV[4]}"
let stderr = fp"${ARGV[5]}"
let append = ARGV[6] == "true"

let command = process.command_argv(
  xsh,
  [xsh, child, token, marker.display()],
  stdout: stdout,
  stderr: stderr,
  stderr_append: append,
)
let status = process.run(command)?
if ! status.exited_with(0) {
  abort(70)
}
print "process-run:waited"
