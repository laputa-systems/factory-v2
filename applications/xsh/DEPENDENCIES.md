# XSH application dependency boundary

The XSH application adds no direct Rust or JavaScript dependency to the generic
trusted workspace. Its parsing-only adapter uses the public
`society-content` byte-identity type through the path recorded in its isolated
workspace manifest.

The application’s executable slice may require an assigned product binary,
source checkout, and ordinary host evaluator tools. Those are sealed or
recorded application inputs, not dependencies of generic trusted physics and
not authority to alter a generic runtime profile. The exact generic Pi-host
dependency and its qualification advisory remain owned by
[`../../DEPENDENCIES.md`](../../DEPENDENCIES.md).

Adding an application dependency requires an application-local contract
decision, an exact lock update, and a focused application judge. It must not
silently alter the generic dependency graph.
