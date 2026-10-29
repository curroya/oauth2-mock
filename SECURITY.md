# Security

## Scope

**Do not use in production.** Anyone who can reach oauth2-mock can sign in as any user without a password, and with the default `KEY_SEED` anyone can forge its ID tokens. It is for testing and development only. Publish it on `127.0.0.1` (`-p 127.0.0.1:5556:5556`) so only your machine can reach it. Running it where untrusted users can reach it is out of scope.

In scope:

- The image or binary doing something beyond what the README describes, such as reading files or reaching hosts it should not.
- A vulnerable dependency that ships in the image.
- The CI or publish workflows leaking the Docker Hub token or letting a pull request publish an image.

## Reporting

Report privately through **Security → Report a vulnerability** on this repository. Do not open a public issue.

Expect a first answer within a week.
