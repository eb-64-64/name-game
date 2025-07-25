start:
    podman kube play name-game.yaml

clean:
    podman pod rm name-game

stop:
    podman pod stop name-game

build:
    podman build . --tag name-game-app
    cd valkey/prod && podman build . --tag name-game-valkey-prod
