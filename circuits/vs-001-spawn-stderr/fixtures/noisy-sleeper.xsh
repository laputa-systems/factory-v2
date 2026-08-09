# Emits streams, publishes readiness, and performs its side effect only after a
# short delay. The owned-cancellation probe observes past this delay after
# cancellation, so a silently ignored cancellation publishes `completed`.
let token = ARGV[0]
let ready = fp"${ARGV[1]}"
let completed = fp"${ARGV[2]}"

# `print`/`eprint` from a killed interpreter may remain buffered. Direct,
# argument-vector `printf` children complete before readiness, giving the
# cancellation probe durable partial stream evidence without a shell boundary.
run printf "%s\\n" f"stdout:${token}" ?
run printf "%s\\n" f"stderr:${token}" >& 2 ?
ready.write(f"ready:${token}")?
time.sleep(250ms)?
completed.write(f"completed:${token}")?
