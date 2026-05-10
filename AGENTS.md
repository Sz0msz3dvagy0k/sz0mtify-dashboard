# AGENTS.md

Instructions for AI coding agents working in this repository.

## Main rule

Do not pretend a task is finished. Solve the actual problem, verify it, and explain what changed.

## Forbidden behavior

Never use any of these as a final solution:

- `TODO`
- `FIXME`
- `not implemented`
- placeholder functions
- fake/mock data unless the task explicitly asks for it
- commented-out code as a replacement for working code
- empty catch blocks
- silent error ignoring
- broad rewrites when a small fix is enough

If something cannot be implemented because information is missing, stop and explain exactly what is missing.

## Bug fixing workflow

For every bug fix:

1. Reproduce or trace the bug first.
2. Identify the real root cause before editing.
3. Make the smallest correct fix.
4. Add or update a test when practical.
5. Run the relevant verification command.
6. Report the root cause, changed files, verification result, and remaining uncertainty.

Do not claim the bug is fixed unless it was verified or the reasoning is precise and complete.

## API/request debugging rules

When a request returns an empty response, wrong response, or error:

1. Trace the full path:
   - frontend caller
   - request URL
   - request method
   - headers/auth
   - backend route/API handler
   - service function
   - database/external API call
   - response serialization
2. Check whether the server is returning an empty body, the client is parsing incorrectly, or an upstream call is empty.
3. Do not replace the issue with a placeholder response.
4. Do not hide errors by returning empty arrays/objects unless that is the documented behavior.
5. Prefer explicit errors and logs over silent failure.

## Editing rules

- Keep changes minimal and focused on the requested task.
- Preserve existing code style and naming.
- Do not rename files, move folders, or change architecture unless necessary.
- Do not remove user code unless it is clearly wrong or obsolete.
- Do not introduce new dependencies unless there is a strong reason.
- Do not change formatting across unrelated files.

## Verification

Before finishing, run the most relevant available command, such as:

- `npm test`
- `npm run test`
- `npm run lint`
- `npm run typecheck`
- `npm run build`
- a direct `curl` command for API bugs

Use the commands that actually exist in this repo. If a command fails because of environment setup, missing secrets, missing dependencies, or unavailable services, report that clearly.

## Final response format

End every coding task with:

```text
Root cause:
- ...

Changed files:
- ...

Verification:
- Command: ...
- Result: ...

Remaining uncertainty:
- ...
```

If no files were changed, say so.

## Security and secrets

- Never commit secrets, API keys, tokens, cookies, or passwords.
- Use environment variables for sensitive values.
- Do not print secret values in logs.
- Do not weaken authentication, authorization, or validation to make a bug disappear.

## Dependency and environment rules

- Use the package manager already used by the repo.
- Do not switch package managers without being asked.
- Do not delete lockfiles unless explicitly required.
- Do not assume external services are available; handle failures clearly.

## Communication rules

- Be direct and specific.
- State uncertainty instead of guessing.
- Ask for missing required information only when the task cannot continue without it.
- Do not say something is done when it is only partially done.
