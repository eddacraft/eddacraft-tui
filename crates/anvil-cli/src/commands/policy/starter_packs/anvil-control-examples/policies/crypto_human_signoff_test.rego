# Tests for the crypto_human_signoff policy.
#
# Every case uses PolicyInput v1 `input.diff.changed_files`.

package anvil.policies.crypto_human_signoff_test

import rego.v1

import data.anvil.policies.crypto_human_signoff

test_crypto_dir_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["crypto/src/aes.rs"]}}
}

test_nested_crypto_dir_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["crates/foo/crypto/mod.rs"]}}
}

test_pem_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["certs/server.pem"]}}
}

test_key_file_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["secrets/service.key"]}}
}

test_rustls_name_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["crates/rustls-config/src/lib.rs"]}}
}

test_openssl_name_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["vendor/openssl/build.rs"]}}
}

test_libsodium_name_violates if {
	count(crypto_human_signoff.violation) > 0 with input as {"diff": {"changed_files": ["deps/libsodium/src/aead.c"]}}
}

test_ordinary_source_is_clean if {
	count(crypto_human_signoff.violation) == 0 with input as {"diff": {"changed_files": ["src/app.rs"]}}
}
