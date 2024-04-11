#!/usr/bin/env nushell

let script = '
sudo mv /tmp/benchmark /usr/local/bin/
sudo chown root:root /usr/local/bin/benchmark
sudo chmod +x /usr/local/bin/benchmark
sudo setcap CAP_NET_BIND_SERVICE=+eip /usr/local/bin/benchmark
sudo restorecon /usr/local/bin/benchmark

sudo systemctl daemon-reload
sudo systemctl restart benchmark
sleep 1
sudo systemctl --no-pager status benchmark
'

def main [
    branch: string
    --user (-u): string | null = null
] {
    let user = $user | default $env.USER
    let repo = 'scverse/benchmark'
    let run = gh --repo $repo run list --branch $branch --workflow=rust.yml --status=success --json=displayTitle,headSha,startedAt,databaseId | from json | get 0

    echo $'Downloading artifact for ($run.displayTitle) at ($run.headSha | str substring ..7) from ($run.startedAt)'
    rm --force --permanent /tmp/benchmark
    gh --repo $repo run download ($run | get databaseId) --name=Binary --dir=/tmp/

    let login = $'($user)@146.107.241.1'
    echo $'Deploying at ($login)'
    rsync -P /tmp/benchmark $'($login):/tmp/'
    rm /tmp/benchmark
    ssh -t $login $script
}
