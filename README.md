# tario

[![Current Version](https://img.shields.io/badge/version-0.1.1-blue)](https://github.com/vadzimshpak/tario/releases/tag/v0.1.1)

CLI utility for file sharing

## Installation

```bash
wget https://github.com/vadzimshpak/tario/releases/download/v0.1.1/tario_0.1.1-1_amd64.deb
sudo apt install ./tario_0.1.1-1_amd64.deb
```

## Client

```bash
tario ./my-folder
```

Upload to your own server:

```bash
tario ./my-folder --bucket-url 127.0.0.1:3000
```

## Server

```bash
tario --server
```

Custom host and port:

```bash
tario --server --url 127.0.0.1 --port 8080
```
