# CCTV SMTP Alerts

Simple SMTP server to accept emails from CCTV NVR system alerts.
These are usually for builtin motion detection and healthchecks.

Since it should only be used by the CCTV system, it does not validate the actual request much.
Eg: sender to/from doesn't matter.

As long as the client sends `AUTH LOGIN` before sending `DATA`, it should be accepted.

### TODO:
- Upgrade connection to TLS.
- Restructure message reciever so closing connection is easier.
- Whitelist client IP when accepting connection.
- Actually finish all the event parsing + sending.