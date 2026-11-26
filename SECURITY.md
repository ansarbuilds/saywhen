# Security and privacy

saywhen uses your camera and microphone and types into other apps, so we take security and privacy reports seriously.

## Reporting a problem

Please do **not** open a public issue for a security or privacy problem.

Use GitHub's private reporting instead: go to the **Security** tab of this repository and click **Report a vulnerability**. You should hear back within a week.

## API keys

Optional cloud clean-up can use a key you provide. That key is stored in the system keychain on macOS and Windows (or a private file if the keychain is unavailable). It is not written into `settings.local.json`, and it is not passed on a process command line. Remote clean-up addresses must use HTTPS. A local Ollama address does not need a key.

## What counts

- Camera frames or audio being saved, logged or sent anywhere without the user opting in.
- Text being typed or Enter being pressed in an app the user did not expect.
- A way around the risky command guard or the trusted app list.
- Anything that lets another app or website control saywhen.

## Supported versions

Only the latest release gets fixes.
