---
sidebar_position: 5
---

# Pyrsia and Docker

Once you have setup your [local environment](../get_involved/local_dev_setup.md) with Pyrsia you are now ready to run Pyrsia with Docker support.

## Integrating and Building with Docker

- Install [Docker](https://www.docker.com/get-started)
  - macOS and Windows: Compose is included in Docker Desktop
  - Linux: [Downloaded Compose](https://github.com/docker/compose#linux)

## Configure Docker Daemon for Pyrsia Network

Follow these steps to run a Pyrsia node and use it as the registry for all Docker Hub content.

1. `cd pyrsia/pyrsia_node`
2. You need to start the Pyrsia Node. To do so, you have 2 options:
   - Rust: `DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run`
   - Docker Compose: `docker-compose up --build`

   *Note*: ⚠️ Do not to stop this process, a running node is required for the
   following steps:

3. **configure Docker** to use Pyrsia, which is running at `http://localhost:7888`,
    open your Docker daemon settings and add this entry in the root JSON object:

    **On Linux platforms**:

    ```json
      "registry-mirrors": ["http://localhost:7888"]
    ```

    By default you can find the Docker daemon settings here `/etc/docker/daemon.json`.

    **On other platforms**:

    You can find the Docker daemon settings in Docker Desktop -> Preferences -> Docker Engine.

    If you're running Pyrsia inside Docker, the `registry-mirrors` settings can be set to `http://localhost:7888` as well. However if you're not running inside Docker, you cannot use `localhost` because localhost will point to the Docker Desktop environment. Instead you have to use the hostname of your host machine. If you don't know/have that, you can add this to `/etc/hosts` (on Mac) or `c:\windows\system32\drivers\etc\hosts` (on Windows):

    ```text
    127.0.0.1       my-pyrsia-host
    ```

    And then use that name in the Docker configuration file like this:

    ```json
    "registry-mirrors": ["http://my-pyrsia-host:7888"]
    ```

4. using another terminal, use `docker` to pull an image from Pyrsia:

    ```sh
    docker pull ubuntu
    ```

   (or pull any other Docker image of your choice)

    Optionally, you can inspect the Pyrsia node logs to check where the image came from. This can be either:

    - locally (if it was cached by Pyrsia before)
    - from the Pyrsia network
    - or from Docker Hub (if it wasn't previously available in the Pyrsia network)
