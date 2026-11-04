# Publish a multi-platform backend alpha

The **Docker alpha** workflow builds and tests native `linux/amd64` and
`linux/arm64` images, then publishes one multi-platform tag to `carfdev/cuenora`.
This is a testing artifact, not a production deployment or security certification.

## One-time setup

In Docker Hub, create `carfdev/cuenora`. For anonymous downloads it must be public.
In `cuenora/cuenora` GitHub Actions settings, configure:

| Kind | Name | Value |
| --- | --- | --- |
| Variable | `DOCKERHUB_USERNAME` | `carfdev` |
| Secret | `DOCKERHUB_TOKEN` | Docker Hub token with Read & Write, no Delete |

Never put the token in source, build arguments, `.env` or logs. A personal Docker
Hub token is account-wide, not restricted to this one repository; use an expiry
and revoke it when no longer needed.

## Publish

1. Merge the workflow and intended backend revision into `main` after review/CI.
2. Open **Actions → Docker alpha → Run workflow**, selecting **main**.
3. Wait for both native build/test jobs and the publication job to succeed.
4. Read the published tag and source SHA in the run summary.
5. Inspect the manifest and retain its digest for staging:

   ```bash
   docker buildx imagetools inspect carfdev/cuenora:0.1.0-alpha.N
   docker pull --platform linux/amd64 carfdev/cuenora:0.1.0-alpha.N
   docker pull --platform linux/arm64 carfdev/cuenora:0.1.0-alpha.N
   ```

Replace `N` with that workflow run number (not the run ID). PR validation runs also
consume run numbers, so the first published tag need not end in `.1`.
No `latest` tag is published. Use **Re-run all jobs**, not failed jobs only, so both artifacts belong to the
same attempt. A rerun uses the same alpha tag; base image tags may
have changed, so it is not immutable. Pin staging to `carfdev/cuenora@sha256:…`.

## Checks and failure boundaries

Build jobs have no registry credentials. They use the existing Docker runtime
checks: unprivileged user, native architecture, default/overridden listener,
liveness, OpenAPI rendering and rejected-setting redaction. The exact checked
images are transferred as one-day Actions artifacts, not rebuilt for publication.
No database tests or vulnerability scan are added by these image smoke checks;
existing backend CI and isolated PostgreSQL verification remain separate gates.

Only manual dispatch on `main` can publish, after both architectures pass.
Temporary architecture tags use `build-<run-id>-<attempt>-<architecture>`.
If a push fails, intermediate tags may remain; do not deploy them or delete them
automatically. The final tag is created after both architecture uploads, then
inspected for exactly the two requested platforms. A failure after manifest push
may mean the tag exists: inspect it before rerunning or claiming publication failed.

The Dockerfile still contains API/OpenAPI binaries only. It neither applies
migrations nor packages PostgreSQL or the recurrence job. Configure secrets at
runtime and follow [deployment prerequisites](SETUP.md#cloudflare-proxied-dns--dokploytraefik--api)
before exposing any container publicly.

For a disposable, empty alpha.2 database, follow the
[one-time initialization guide](INITIALIZE_ALPHA_DATABASE.md).

## Verification status

Native AMD64/ARM64 checks and alpha.2 publication succeeded; see the [completed execution record](../plans/completed/2026-11-04-docker-alpha.md).
A successful workflow configuration check does not prove either image was built
or published. Do not mark P14.02 or the wider release phase complete on that basis.
