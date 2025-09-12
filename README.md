# Secure Share
### A cross-platform file sharing app built using Tauri.

## Dependencies
Ensure rust and node are installed: \
rustc >= 1.89 \
node >= 20.10 \
npm >= 9.6 

### Debian
Run
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

### Windows
Install the Microsoft C++ Build tools [here](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)

Then run:
`winget install --id Rustlang.Rustup` in powershell as an administrator

### MacOS
Ensure the xcode developer tools are installed using:
`xcode-select --intstall`

## Running the app
Clone the repository then run: 
`$ npm run tauri build` \
to build the application for your system. \
Or: \
`$npm run tauri dev` \
to start the development server and view live changes on save


## Recommended IDE Setup.
[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
