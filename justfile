list:
    @just --list

app-logs:
    podman pod logs name-game -c name-game-app

clean:
    podman pod rm name-game

cold-start:
    podman kube play name-game.yaml

start:
    podman pod start name-game

stop:
    podman pod stop name-game

build:
    podman build . --tag name-game-app
    cd valkey/prod && podman build . --tag name-game-valkey-prod
