# Cryptographic-path sign-off — agents must not change cryptographic code
# until a human records an exception.
#
# Matching paths emit the `violation` family. On MCP pre-write the default
# interrupt posture then vetoes the write until `anvil exception grant --policy
# crypto-human-signoff`. This is an engineering control on the working-tree
# diff, not a cryptographic audit.
#
# Reads only PolicyInput v1 `diff.changed_files`.

package anvil.policies.crypto_human_signoff

import rego.v1

crypto_path(path) if startswith(lower(path), "crypto/")

crypto_path(path) if contains(lower(path), "/crypto/")

crypto_path(path) if endswith(lower(path), ".pem")

crypto_path(path) if endswith(lower(path), ".key")

crypto_path(path) if contains(lower(path), "rustls")

crypto_path(path) if contains(lower(path), "openssl")

crypto_path(path) if contains(lower(path), "libsodium")

violation contains msg if {
	some path in input.diff.changed_files
	crypto_path(path)
	msg := sprintf(
		"`%s` looks like cryptographic code or key material. Agents must not change it until a human records `anvil exception grant --policy crypto-human-signoff --reason \"...\"`.",
		[path],
	)
}
