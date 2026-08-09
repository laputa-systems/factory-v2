# ARGV: xsh child token marker
#
# This deliberately has no stderr field. `process.run` must inherit the
# evaluator's parent stderr through SpawnManagedOptions::inherited_process_group.
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"

let command = process.command_argv(xsh, [xsh, child, token, marker.display()])
let status = process.run(command)?
if ! status.exited_with(0) {
  abort(82)
}
print "process-run-inherit:waited"
