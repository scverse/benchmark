# In one shell run:
#
# ```nushell
# cargo run -- --dry-run serve --secret-token "It's a Secret to Everybody"
# ```
#
# After installing `jaq` and `libgcrypt` in another nushell (activated via the `nu` command after installation), run the following:
#
# ```nushell
# source scripts/test.nu
# gh-hook http://localhost:3000/ (open ./src/fixtures/test.hook-pr-sync.json) --full --allow-errors
# ```

# Test function mimicking GH webhook delivery
def gh-hook [
    url: string
    payload: any
    --secret: string = "It's a Secret to Everybody"
    --full
    --allow-errors
] {
    # `jaq -c` is necessary because of https://github.com/nushell/nushell/issues/11900
    let sig = ($payload | to json -r | jaq -c | str trim | hmac256 $secret | $'sha256=($in)')
    http post --content-type=application/json --headers {'X-Hub-Signature-256': $sig } $url $payload --full=$full --allow-errors=$allow_errors
}
