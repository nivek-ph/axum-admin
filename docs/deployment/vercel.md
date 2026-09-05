# Vercel deployment

The backend and Admin Console deploy as two independent Vercel projects from this monorepo.

## Durable file storage

Vercel Functions must use an S3 storage because their local filesystem is not durable. Migration
creates a local default for development; replace it with an S3 storage through the Admin Console
or `/api/storages` before accepting production uploads. Loading that unused local record does not
create its root directory, so the API can boot on Vercel before the default is switched to S3.

The public base URL must already include any path that exposes the configured root; upload responses
append only the generated object name. Ensure the bucket or CDN makes that URL readable.
The backend accepts resumable 4 MiB chunks through `/api/files/uploads`, with a 1 GiB application
limit. The Admin Console waits for `Retry-After` and resumes when a chunk reaches the API rate
limit. Each request remains subject to Vercel's current request-body limit. Direct presigned browser
uploads are not part of this deployment mode.

The backend validates every stored driver before serving requests. An unknown driver, an empty
local root, a missing S3 bucket/region/public URL, or an incomplete access-key pair stops startup with
a storage error. The S3 public base must use an `http://` or `https://` URL. Each upload resolves the
current default from PostgreSQL, so warm function instances do not retain a stale default.

## Existing projects

Repository maintainers can deploy the configured projects through the manual
[Deploy to Vercel workflow](https://github.com/nivekph/axum-admin/actions/workflows/vercel.yml).

The workflow asks for:

1. The Git branch to deploy.
2. `both`, `backend`, or `frontend` as the target.
3. `preview` or `production` as the environment.
4. An optional API base URL override for the Admin Console.

It does not deploy on pushes or pull requests. A preview creates a non-production deployment;
production updates the configured production domains.

Configure these GitHub Actions repository secrets before running the workflow:

| Secret | Purpose |
| --- | --- |
| `VERCEL_ORG_ID` | Vercel account or team ID shared by both projects |
| `VERCEL_BACKEND_PROJECT_ID` | Backend Vercel project ID |
| `VERCEL_FRONTEND_PROJECT_ID` | Admin Console Vercel project ID |
| `VERCEL_TOKEN` | Vercel access token used by the workflow |

The Admin Console resolves its API base URL from the optional workflow input first, then from the
backend preview URL when deploying both projects, and finally from the `VITE_API_BASE_URL`
repository secret. Configure that secret as the fallback for frontend-only and production runs.

## Project configuration

| Component | Vercel project | CLI working directory | Configuration |
| --- | --- | --- | --- |
| Backend | `axum-admin` | Repository root | `vercel.json` and `api/axum.rs` |
| Admin Console | `axum-admin-web` | `apps/desktop` | `apps/desktop/vercel.json` |

For the existing `axum-admin-web` project deployed through GitHub Actions, leave
**Settings → Build and Deployment → Root Directory** empty. The workflow already runs the Vercel
CLI from `apps/desktop`; configuring the same directory in Vercel would resolve it as
`apps/desktop/apps/desktop`.

Vercel project names and production domains are managed separately. Renaming a project does not
replace an existing production domain; update the project's **Domains** settings separately.

Deployment uploads are isolated by `.vercelignore` at the repository root and
`apps/desktop/.vercelignore` for the Admin Console.

## New projects

Each Vercel Deploy Button creates a new Git repository and one Vercel project. Deploy the backend
and Admin Console separately, then configure `VITE_API_BASE_URL` with the deployed backend URL and
the `/api` suffix.

The frontend Deploy Button configures Root Directory as `apps/desktop` for Vercel's native Git
deployment. Keep that setting for projects created with the button. Clear it only when reusing the
GitHub Actions workflow, which already runs Vercel CLI from `apps/desktop`.
