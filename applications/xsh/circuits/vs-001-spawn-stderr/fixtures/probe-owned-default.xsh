# ARGV: xsh child token marker
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"

let command = process.command_argv(xsh, [xsh, child, token, marker.display()])
let handle = spawn command?
let status = wait handle?
if ! status.exited_with(0) {
  abort(74)
}
print "owned-default:waited"
