# benchmark machine

## Development plan:

1. [MVP](https://github.com/scverse/benchmark/milestone/1):

   Queue and runs exist on the benchmark server, which handles everything

2. [Dedicated queue](https://github.com/scverse/benchmark/milestone/2)

   To avoid a web server running in the background, have a VM or so that maintains the queue

3. [Cancellation](https://github.com/scverse/benchmark/milestone/3)

   Improve UX by allowing cancellation and other niceties

## MVP Setup

Assuming you have a `<user>` login with sudo rights on `scvbench`.

### One-time server setup
1. As the `benchmarker` user, install micromamba, then:

   ```shell
   micromamba create -n asv -c conda-forge conda mamba virtualenv asv
   micromamba run -n asv asv machine --yes
   ```

2. Update `LoadCredentialEncrypted` lines in `benchmark.service` using

   ```shell
   sudo systemd-creds encrypt --name=webhook_secret secret.txt -
   sudo systemd-creds encrypt --name=github_token scverse-bot-pat.txt -
   shred secret.txt scverse-bot-pat.txt
   ```

3. Copy the `benchmark.service` file to the system, enable and start the service:

   ```console
   $ rsync benchmark.service <user>@scvbench:
   $ ssh <user>@scvbench
   scvbench$ sudo mv benchmark.service /etc/systemd/system/
   scvbench$ sudo systemctl enable --now benchmark
   ```

### Deployment
1. Make changes in `<branch>` (either `main` or a PR branch) and wait until CI finishes.
2. Run `nu scripts/deploy.nu <branch> --user=<user>`.
3. Trigger a run,
   e.g. remove and re-add the <kbd>benchmark</kbd> label in [PR 11][].

You can use `journalctl -u benchmark -f` to tail the logs.

[PR 11]: https://github.com/scverse/benchmark/pull/11
