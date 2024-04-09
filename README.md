# benchmark machine

## Development plan:

1. [MVP](https://github.com/scverse/benchmark/milestone/1):

   Queue and runs exist on the benchmark server, which handles everything

2. [Dedicated queue](https://github.com/scverse/benchmark/milestone/2)

   To avoid a web server running in the background, have a VM or so that maintains the queue

3. [Cancellation](https://github.com/scverse/benchmark/milestone/3)

   Improve UX by allowing cancellation and other niceties

## Usage

Eventually, we’ll just have a project-wide webhook like this. For now, if you want to test:

1. Add a [asv config][] to your project (either the project root or a <samp>benchmarks</samp> directory)
2. Add a webhook to your scverse project with these [webhook settings][], i.e.
   - Content type: <samp>application/json</samp>
   - Let me select individual events → **Pull Requests**
3. Add a label <kbd>benchmark</kbd> to a PR authored by a trusted user.
4. Watch [scverse-benchmarks][] add and update a comment with the PR’s performance impact.

[asv config]: https://asv.readthedocs.io/en/v0.6.1/using.html
[webhook settings]: https://github.com/scverse/benchmark/settings/hooks/464592128
[scverse-benchmarks]: https://github.com/apps/scverse-benchmark

## MVP Setup

All these currently assume you have a <samp>&lt;user></samp> login with sudo rights on the <samp>scvbench</samp> server.

### Debugging

- Use `journalctl -u benchmark -f` on the server to tail the logs of the service.
- Check GitHub’s page for [Hook deliveries][].

[Hook deliveries]: https://github.com/scverse/benchmark/settings/hooks/464592128?tab=deliveries

### One-time server setup
1. As the <samp>benchmarker</samp> user, install micromamba, then:

   ```shell
   micromamba create -n asv -c conda-forge conda mamba virtualenv asv
   micromamba run -n asv asv machine --yes
   ```

   (use `micromamba activate asv` to make `asv` available in your PATH)

2. Update `LoadCredentialEncrypted` lines in <samp>benchmark.service</samp> using

   ```shell
   sudo systemd-creds encrypt --name=webhook_secret secret.txt -
   sudo systemd-creds encrypt --name=app_key app-key.pem -
   shred secret.txt app-key.pem
   ```

3. Copy the <samp>benchmark.service</samp> file to the system, enable and start the service:

   ```console
   $ rsync benchmark.service <user>@scvbench:
   $ ssh <user>@scvbench
   scvbench$ sudo mv benchmark.service /etc/systemd/system/
   scvbench$ sudo systemctl enable --now benchmark
   ```

Further steps:
1. Setup chrony (<samp>/etc/chrony.conf</samp>) to use internal servers

   ```ini
   server 146.107.1.13 trust
   server 146.107.5.10
   ```

2. [Performance setup](https://github.com/scverse/benchmark/issues/1)

### Deployment
1. Make changes in <samp>&lt;branch></samp> (either <samp>main</samp> or a PR branch) and wait until CI finishes.
2. Run `nu scripts/deploy.nu <branch> --user=<user>`.
3. Trigger a run,
   e.g. remove and re-add the <kbd>benchmark</kbd> label in [PR 11][].

[PR 11]: https://github.com/scverse/benchmark/pull/11

## Development

For local development:

1. Start the server locally
2. use `scripts/test.nu` to send a payload (check the script for examples for both steps)
