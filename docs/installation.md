# Installing Xaga

Xaga currently supports Apple Silicon Macs. Intel Macs are not supported.

Download the latest appropriate build from the repository's
[GitHub Releases](https://github.com/reedable/xaga/releases) page and open the
downloaded disk image.

You can evaluate Xaga by opening it directly from the mounted disk image. Do
not copy an application into **Applications** merely because its installation
instructions say to: its location does not establish that it is safe. First
confirm that you obtained Xaga from the source you intended, decide whether you
trust that source and this build, and complete the Gatekeeper steps below only
if you do.

## Opening the current ad-hoc-signed build

Xaga's macOS releases carry an ad-hoc signature so macOS can verify that the
application bundle is internally consistent. They are not yet signed with an
Apple Developer ID or notarized through Apple's paid Developer Program. The
first time you open Xaga, macOS therefore blocks it with an **“Xaga.app” Not
Opened** message:

![macOS warning that Xaga.app could not be verified](images/xaga_app_not_opened.png)

Only override this protection if you downloaded Xaga from this repository's
GitHub Releases page and trust that copy:

1. In the warning, click **Done**.
2. Open **Apple menu → System Settings → Privacy & Security**.
3. Scroll down to **Security** and find the message that Xaga was blocked.
4. Click **Open Anyway**.
5. Authenticate with your Mac password or Touch ID, then confirm by clicking
   **Open**.

macOS remembers the exception for that copy of Xaga, so it can be opened
normally afterward. **Open Anyway** is available for about an hour after the
blocked launch attempt. See Apple's
[instructions for opening an unnotarized app](https://support.apple.com/102445)
for more information.

## Installing permanently

After you have evaluated Xaga and decided to keep it, quit Xaga and drag
**Xaga.app** from the mounted disk image into **Applications**. This gives the
app a stable location and lets you eject the disk image; it does not make the
app trusted or safer. macOS may ask you to approve the moved copy again.
