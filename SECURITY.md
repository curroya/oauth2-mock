# Security

## Scope

oauth2-mock is a test fixture. It signs in anyone who types an email, and its signing key comes from a public seed. Running it where untrusted users can reach it is out of scope.

In scope:

- The image or binary doing something beyond what the README describes, such as reading files or reaching hosts it should not.
- A vulnerable dependency that ships in the image.
- The CI or publish workflows leaking the Docker Hub token or letting a pull request publish an image.

## Reporting

Report privately through **Security → Report a vulnerability** on this repository. Do not open a public issue.

Expect a first answer within a week.
