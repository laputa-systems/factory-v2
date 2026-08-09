# ARGV: xsh child token marker stdout stderr
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stdout = fp"${ARGV[4]}"
let stderr = fp"${ARGV[5]}"

let handle = spawn run ${xsh} ${child} ${token} ${marker.display()} > stdout 2> stderr ?
let status = wait handle?
if ! status.exited_with(0) {
  abort(72)
}
print "spawn-run:waited"
