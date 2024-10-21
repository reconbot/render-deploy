# Render Deploy

I want to be able to trigger production deploys and wait for them to happen. You need to have an env var `RENDER_API_KEY` set with your api key. I made this to scratch and itch and I chose rust for fun.

```bash
# trigger a deploy of the service with the latest commit and wait for it to go live
$ render-deploy -w $SERVICE_NAME 
```

## Help output

```bash
Usage: render-deploy [OPTIONS] --api-key <API_KEY> <NAME> [COMMIT]

Arguments:
  <NAME>    name of your service
  [COMMIT]  optional commit to deploy (otherwise head of the default branch)

Options:
  -w, --wait               Wait for the deploy to finish or fail
  -a, --api-key <API_KEY>  [env: RENDER_API_KEY=]
  -t, --timeout <TIMEOUT>  wait for deploy timeout in seconds, doesn't cancel the
                           deploy just exits [default: 600]
  -h, --help               Print help
  -V, --version            Print version
```
