FROM oven/bun:alpine as frontend-build
WORKDIR /usr/src/frontend
COPY frontend/public/ ./public/
COPY frontend/src/ ./src/
COPY frontend/index.html \
     frontend/package.json \
     frontend/svelte.config.js \
     frontend/tsconfig.app.json \
     frontend/tsconfig.json \
     frontend/tsconfig.node.json \
     frontend/vite.config.ts ./
RUN ["bun", "install"]
RUN ["bun", "run", "build"]

FROM rust:alpine as backend-build
RUN ["apk", "add", "--no-cache", "musl-dev"]
WORKDIR /usr/src/backend
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/src/ ./src/
RUN ["cargo", "build", "--release"]

FROM alpine:latest as app
RUN ["adduser", "-D", "app"]
USER app
WORKDIR /home/app
COPY --from=frontend-build /usr/src/frontend/dist/ ./static/
COPY --from=backend-build /usr/src/backend/target/release/backend ./
COPY /backend/config/base.toml /backend/config/prod.toml ./config/
ENV APP_ENVIRONMENT=prod
CMD ["./backend"]
