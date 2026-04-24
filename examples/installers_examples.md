## CHMER Installer Examples

These are quick examples for every supported installer target.

### Linux / macOS / BSD (shell installer)

```sh
curl -fsSL https://github.com/HSR-projects/chmer/releases/latest/download/install.sh | sh
```

### Windows (PowerShell installer)

```powershell
iwr -useb https://github.com/HSR-projects/chmer/releases/latest/download/install.ps1 | iex
```

### Windows (batch installer wrapper)

```bat
install.bat
```

### Windows uninstall

```bat
uninstall.bat
```

### Custom release base URL

```sh
CHMER_RELEASE_BASE="https://your-host/path/to/release" sh install.sh
```

```powershell
$env:CHMER_RELEASE_BASE = "https://your-host/path/to/release"
.\install.ps1
```

### Install without assets pack

```sh
CHMER_WITH_ASSETS=0 sh install.sh
```

```powershell
$env:CHMER_WITH_ASSETS = "0"
.\install.ps1
```

### Custom install locations

```sh
CHMER_INSTALL_DIR="$HOME/.local/bin" CHMER_ASSET_DIR="$HOME/.local/share/chmer" sh install.sh
```

```powershell
$env:CHMER_INSTALL_DIR = "$env:LOCALAPPDATA\CHMER\bin"
$env:CHMER_ASSET_DIR = "$env:LOCALAPPDATA\CHMER\assets"
.\install.ps1
```
