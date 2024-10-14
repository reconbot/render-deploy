# Render Deploy

I want to be able to trigger production deploys and wait for them to happen. You need to have an env var `RENDER_API_KEY` set with your api key.

```bash
# all options are optional
$ render-deploy "SERVICE NAME" -c commitSHA --wait
```

