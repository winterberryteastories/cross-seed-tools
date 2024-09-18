# cross-seed-tools

This is a small web service used together with cross-seed-tools, autobrr, sonarr and radarr:

* It allows announcing to multiple cross-seed instances (useful with autobrr since only one filter can match a release otherwise).
* It allows to send requests to cross-seed for both usenet and torrent downloads after imports into sonarr / radarr.
* It provides an endpoint to add torrents from your seedbox into your local client after they got imported into sonarr / radarr.

## Getting Started

First create a `.env` file (based on `.env.sample` found in the repository). Only `HOST` and `API_KEY` are required, the rest are optional depending on which of the three components you are planning to use. All three components can be used entirely seperate.

### Check for cross-seeds for sonarr / radarr imports

This is largely inspired by https://gist.github.com/zakkarry/ddc337a37b038cb84e6248fe8adebb46. I reimplemented it in Rust since I found Bash quite limiting in the amount of changes I could make.

* Make sure you set `XSEED_TORRENT_CLIENTS` to the torrent clients you want to include and `XSEED_USENET_CLIENTS` for the usenet clients.
* Additionally `CROSS_SEED_LOCAL_URL` and `CROSS_SEED_LOCAL_API_KEY` need to be set.
* Add the connection in sonarr / radarr:
  * Go to Settings -> Connect
  * Create a new connection
  * Select Webhook
  * For Sonarr:
    * Check "On Import Complete"
    * Use `http://cross-seed-tools:2469/xseed-sonarr` with method `POST`
  * For Radarr:
    * Check "On File Import" and "On File Upgrade"
    * Use `http://cross-seed-tools:2469/xseed-radarr` with method `POST`
  * Within the Headers select `X-Api-Key` as the "Key" and your api key as "Value".
  * Use "Test" to check if it's working!

### Announce multiplexing

Instead of announcing simply to one cross-seed, this endpoint announces to both the local and seedbox cross-seed instances. To use this within autobrr follow these steps:

* Make sure you set `CROSS_SEED_LOCAL_URL`, `CROSS_SEED_LOCAL_API_KEY`, `CROSS_SEED_SEEDBOX_URL` and `CROSS_SEED_SEEDBOX_API_KEY`.
* Replace your cross-seed webhook in autobrr with `http://cross-seed-tools.media-tools:2469/announce` and your "HTTP Request Headers" with `X-Api-Key=your-api-key`.

### Inject seedbox torrents into local qbittorrent after import

This endpoint can be used that on a file import in sonarr / radarr the torrent is automatically inserted into the local qbittorrent instance. It downloads the torrent file from the qbittorrent instance on the seedbox and inserts it into the local qbittorrent. You need to make sure you are syncing the file from the remote machine into the local torrent directory directly. Optionally you can also issue a cross-seed request to add cross seeds automatically as well ;)

* Make sure you set the following variables:
  * `QBITTORRENT_SEEDBOX_NAME` (the name your qbittorrent on the seedbox has in sonarr / radarr)
  * `QBITTORRENT_SEEDBOX_HOST`, `QBITTORRENT_SEEDBOX_USER` and `QBITTORRENT_SEEDBOX_PASSWORD`
  * `QBITTORRENT_LOCAL_HOST`, `QBITTORRENT_LOCAL_USER` and `QBITTORRENT_LOCAL_PASSWORD`
  * `QBITTORRENT_LOCAL_DIR` (the "root" directory of where the local torrent files are located, e.g. `/data/torrent`)
* Optionally set `CROSS_SEED_LOCAL_URL` and `CROSS_SEED_LOCAL_API_KEY` to automatically issue a `/webhook` request on the import.
* Add the connection in sonarr / radarr:
  * Go to Settings -> Connect
  * Create a new connection
  * Select Webhook
  * For Sonarr:
    * Check "On Import Complete"
    * Use `http://cross-seed-tools:2469/inject-seedbox-torrents-sonarr` with method `POST`
  * For Radarr:
    * Check "On File Import" and "On File Upgrade"
    * Use `http://cross-seed-tools:2469/inject-seedbox-torrents-radarr` with method `POST`
  * Within the Headers select `X-Api-Key` as the "Key" and your api key as "Value".
  * Use "Test" to check if it's working!

## Usage

The easiset way to run it is using the docker image with docker compose:

### Docker Compose Example

```
  cross-seed-tools:
    image: ghcr.io/winterberryteastories/winterberryteastories/cross-seed-tools:latest
    container_name: cross-seed-tools
    environment:
      - TZ=...
    volumes:
      - ./cross-seed-tools/.env:/.env
    ports:
      - 2469:2469
    restart: unless-stopped
```
