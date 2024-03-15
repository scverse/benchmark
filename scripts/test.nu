# Source this file using `source test.nu`, then run e.g.:
#
# env "SECRET_TOKEN=It's a Secret to Everybody" cargo run
#
# and in another shell:
#
# gh-hook http://localhost:3000/ (open ./src/fixtures/test.hook-pr-sync.json) --full --allow-errors

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
