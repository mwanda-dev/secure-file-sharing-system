# Secure Share
A Local Secure File Sharing System built in Rust and Tauri

## Prerequisites
For complilation, you will need: 
rustc >= 1.89.0
node >= 20.10.0
npm >= 10.2.3

## System Specific Dependencies
### Debian
Run: 
```
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev
```

### Other Linux Distros
Check for your distro in [Tauri v2 Quick Start](https://v2.tauri.app/start/prerequisites/#linux)

### Window 10 and Later
You need the Microsoft C++ Build tools: 
[Microsoft Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)

### macOS
Ensure the Xcode Command Line Tools are installed: 

```
xcode-select --install
```

## Building the app for your system
With the prerequisites installed, in the project directory run:
```
npm install
npm install -D @tauri-apps/cli@latest
npm run tauri build
```

## For Development
To start the Dev server and open up an instance of the application run:
```
npm run tauri dev
```


### Recommended Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
