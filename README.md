<picture align="left" >
  <source media="(prefers-color-scheme: dark)" srcset="/iloader.svg">
  <img align="left" width="90" height="90" src="/iloader-dark.svg">
</picture>

<div id="user-content-toc">
  <ul style="list-style: none;">
    <summary>
      <h1>iloader</h1>
    </summary>
  </ul>
</div>

---

[![Build iloader](https://github.com/nab138/iloader/actions/workflows/build.yml/badge.svg)](https://github.com/nab138/iloader/actions/workflows/build.yml)

Install SideStore (or other apps) and import your pairing file with ease

Currently, due to a bug, iloader can't correctly allocate keychain access entitlements for LiveContainer. The Multiple LiveContainers feature will not work and apps will all share keychain data (for example, Google account credentials). To avoid this, install SideStore first, then install LiveContainer through SideStore. This will be fixed in a future update.

## Known Issues

### machineId Parsing Error

If you encounter an error "Failed to parse: machineId" or "Parse(\"machineId\")" when loading certificates or installing apps:

**Possible causes:**
- Apple has changed their API format
- Some certificates have unexpected machineId format
- Library compatibility issue

**Solutions (try in order):**
1. **Log out and log back in** - This refreshes your session with Apple
2. **Revoke all certificates** - Go to Certificates section and revoke all existing certificates, then create new ones
3. **Check for updates** - Make sure you're using the latest version of iloader
4. **Report the issue** - If the problem persists, create an issue on GitHub with:
   - Error message details
   - Your macOS version
   - iloader version
   - Steps to reproduce

This error occurs inside the `isideload` library when parsing Apple's API response. The iloader team is working with the library maintainers to resolve this issue.

## Troubleshooting

### Diagnostic Information

iloader now includes enhanced logging to help diagnose issues. To view logs:

**macOS:**
- Open Console.app
- Filter by process name "iloader"
- Or run: `log stream --predicate 'processImagePath CONTAINS "iloader"' --level info`

**Windows:**
- Check Event Viewer for application logs

**Linux:**
- Check system logs: `journalctl -u iloader` or `dmesg | grep iloader`

### Certificate Caching

iloader now caches certificate data for 5 minutes to improve performance. If you need to force a refresh:
- Click the "Refresh" button in the Certificates section
- Or restart the application

### Error Reporting

When reporting errors, please include:
1. Full error message (including technical details)
2. Steps to reproduce
3. Your operating system and version
4. iloader version
5. Relevant log entries (if available)

<img width="1918" height="998" alt="iloader0" src="https://github.com/user-attachments/assets/93cd135d-6d89-46ee-9b9f-12c596806911" />

## How to use

- Install usbmuxd for your platform
  - Windows: [iTunes](https://apple.co/ms)
  - macOS: Included
  - Linux: Potentially included, if not, install via your package manager
- Install the latest version for your platform from the [releases](https://github.com/nab138/iloader/releases)
- Plug in your iDevice to your computer
- Open the app
- Sign into your Apple ID
- Select your action (e.g. install SideStore)

## Features

- Install SideStore (or LiveContainer + SideStore), import certificate and place pairing file automatically
- Install other IPAs
- Manage pairing files in common apps like StikDebug, SideStore, Protokolle, etc
- See and revoke development certificates
- See App IDs
- Save multiple apple ID credentials

## Credits

- Icon made by [Transistor](https://github.com/transistor-exe)
- UI improved by [StephenDev0](https://github.com/StephenDev0)
- [idevice](https://github.com/jkcoxson/idevice) by [jkcoxson](https://github.com/jkcoxson) for communicating with iOS devices
- [isideload](https://github.com/nab138/isideload) for installing apps
- [idevice_pair](https://github.com/jkcoxson/idevice_pair) was used as a reference for pairing file management
- App made with [tauri](https://tauri.app)
## Future Plans

- Set a "default" account to automatically log into
- Import SideStore account info automatically
- Mount DDI and open sidestore after installation
- Check for developer mode and warn about it if not enabled
- Improved error messages
