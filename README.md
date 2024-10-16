# Render Deploy

I want to be able to trigger production deploys and wait for them to happen. You need to have an env var `RENDER_API_KEY` set with your api key.

```bash
# all options are optional
$ render-deploy -c $COMMIT_SHA --wait $SERVICE_NAME
$ render-deploy -w $SERVICE_NAME # trigger a deploy of the service with the latest commit and wait for it to go live
```
