# This child proves whether C03's fake supervisor actually starts the assigned
# child. It otherwise has the same varied `space` output shape as the fluency
# fixture case.
# ARGV: marker
let marker = fp"${ARGV[0]}"
marker.write("child-ran")?
print "stdout:space"
eprint "stderr:space"
