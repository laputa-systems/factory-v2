# Deliberately does not start a child. It writes a fixed line that cannot satisfy
# the judge's varying-token observation or provide an owned lifecycle receipt.
# ARGV: stderr-path
let stderr = fp"${ARGV[0]}"
stderr.write("stderr:fixed")?
print "fake-log:written"
