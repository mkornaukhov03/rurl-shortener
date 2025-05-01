FROM golang:1.24.2-bullseye AS build

COPY . .

RUN go build .
ENTRYPOINT ./victoria-test
