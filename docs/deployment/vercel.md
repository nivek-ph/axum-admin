# Vercel deployment

The backend and Admin Console deploy as two independent Vercel projects from this monorepo.

## Existing projects

Repository maintainers can deploy the configured projects through the manual
[Deploy to Vercel workflow](https://github.com/nivek-ph/axum-admin/actions/workflows/vercel.yml).

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

## File storage

Vercel Functions do not provide a durable writable filesystem. Configure the backend with an
S3-compatible bucket instead of the default local adapter:

| Variable | Required | Purpose |
| --- | --- | --- |
| `FILE_STORAGE_DRIVER=s3` | yes | Select the S3 adapter |
| `S3_BUCKET` | yes | Bucket name |
| `S3_PUBLIC_BASE_URL` | yes | Public bucket, CDN, or custom-domain URL |
| `S3_ROOT` | no | Object prefix; defaults to `uploads` |
| `S3_REGION` | provider-specific | AWS region or provider region |
| `S3_ENDPOINT` | provider-specific | Custom endpoint for R2, MinIO, and other compatible providers |
| `AWS_ACCESS_KEY_ID` | provider-specific | Access key ID |
| `AWS_SECRET_ACCESS_KEY` | provider-specific | Secret access key |
| `AWS_SESSION_TOKEN` | no | Temporary credential session token |
| `S3_VIRTUAL_HOST_STYLE` | no | Enable virtual-hosted-style requests; defaults to `false` |

The configured bucket must allow public reads because the existing file module returns direct file
URLs. Keep write credentials private in the backend project. OpenDAL also supports ambient AWS
credentials when explicit access keys are omitted.

Uploads still pass through the authenticated Rust endpoint. Vercel Functions reject request bodies
larger than 4.5 MB before the application-level 20 MiB limit is reached. Direct browser uploads and
presigned upload URLs are a separate follow-up for larger files.
