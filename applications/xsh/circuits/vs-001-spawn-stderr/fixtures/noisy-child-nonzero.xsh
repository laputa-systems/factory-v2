# Emits both streams and exits with a specified nonzero interpreter status.
#
# Using XSH itself keeps the positive matrix free of a shell-string child. The
# parent probes that a completed nonzero status is still status data while the
# stderr bytes survive redirection.
print "nonzero-stdout"
eprint "nonzero-stderr"
abort(23)
