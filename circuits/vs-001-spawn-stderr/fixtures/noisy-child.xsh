# Emits independently identifiable stdout and stderr before publishing completion.
#
# ARGV[0] is a case token and ARGV[1] is the completion-marker path. The marker
# exists only after both stream writes, so a detached consumer can be observed
# without sleeping for an arbitrary amount of time.
let token = ARGV[0]
let complete = fp"${ARGV[1]}"

print f"stdout:${token}"
eprint f"stderr:${token}"
complete.write(f"complete:${token}")?
