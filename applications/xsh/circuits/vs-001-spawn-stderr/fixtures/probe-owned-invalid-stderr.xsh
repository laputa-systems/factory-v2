# ARGV: xsh child token marker invalid-stderr-path
let xsh = ARGV[0]
let child = ARGV[1]
let token = ARGV[2]
let marker = fp"${ARGV[3]}"
let stderr = fp"${ARGV[4]}"

let command = process.command_argv(
  xsh,
  [xsh, child, token, marker.display()],
  stderr: stderr,
)
match spawn command {
  Err(ProcessError.Redirection {message: message}) => print "owned-invalid-stderr:setup-error"
  Err(error) => abort(75)
  Ok(_) => abort(76)
}
