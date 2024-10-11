# CCTV SMTP Alerts

Simple SMTP server to accept emails from CCTV NVR system alerts.
These are usually for builtin motion detection and healthchecks.

Since it should only be used by the CCTV system, it does not validate the actual request much.
Eg: sender to/from doesn't matter.

As long as the client sends `AUTH LOGIN` before sending `DATA`, it should be accepted.

### Env vars


| Key                | Example        | Description                                            | Required |
|--------------------|----------------|--------------------------------------------------------|----------|
| CCTV_BIND_ADDR     | `0.0.0.0:1234` | SocketAddr IP address to bind for SMTP server.         | Yes      |
| CCTV_USERNAME      | `user`         | SMTP username.                                         | Yes      |
| CCTV_PASSWORD      | `pass`         | SMTP password.                                         | Yes      |
| CCTV_WEBHOOK_URL   | `https://...`  | The URL to send AlarmEvent webhook events to.          | Yes      |
| CCTV_WEBHOOK_KEY   | `token`        | Sent as `Authorization` header in webhook.             | Yes      |
| CCTV_ALARM_SUBJECT | `ALARM`        | The subject line for alarm events, others are ignored. | No       |
| CCTV_ALARM_IP      | `192.168.0.90` | The IP of the NVR, acts as a connection whitelist.     | No       |

### TODO:
- Upgrade connection to TLS.
- Restructure message reciever so closing connection is easier.