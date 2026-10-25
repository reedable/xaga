#!/usr/bin/env swift

import AppKit
import Foundation

// Finder's title and bottom bars reduce the visible background area by roughly
// 60 points. Keep the composition within the top 500 points of this canvas.
let canvasSize = NSSize(width: 700, height: 560)
let outputURL: URL = {
  if CommandLine.arguments.count > 1 {
    return URL(fileURLWithPath: CommandLine.arguments[1])
  }
  return URL(fileURLWithPath: "src-tauri/dmg/background.png")
}()

let image = NSImage(size: canvasSize)
image.lockFocus()

let background = NSGradient(
  starting: NSColor(calibratedRed: 0.055, green: 0.063, blue: 0.082, alpha: 1),
  ending: NSColor(calibratedRed: 0.105, green: 0.078, blue: 0.170, alpha: 1)
)!
background.draw(in: NSRect(origin: .zero, size: canvasSize), angle: -90)

func drawText(
  _ text: String,
  in rect: NSRect,
  font: NSFont,
  color: NSColor,
  alignment: NSTextAlignment = .center
) {
  let paragraph = NSMutableParagraphStyle()
  paragraph.alignment = alignment
  paragraph.lineBreakMode = .byWordWrapping
  paragraph.lineSpacing = 2
  text.draw(
    in: rect,
    withAttributes: [
      .font: font,
      .foregroundColor: color,
      .paragraphStyle: paragraph,
    ]
  )
}

let white = NSColor(calibratedWhite: 0.96, alpha: 1)

drawText(
  "Before opening Xaga",
  in: NSRect(x: 40, y: 497, width: 620, height: 38),
  font: .systemFont(ofSize: 28, weight: .semibold),
  color: white
)
drawText(
  "1. Verify the SHA-256 on this DMG with GitHub Releases",
  in: NSRect(x: 134, y: 450, width: 520, height: 22),
  font: .systemFont(ofSize: 16, weight: .semibold),
  color: white,
  alignment: .left
)
drawText(
  "shasum -a 256 /path/to/Xaga.dmg",
  in: NSRect(x: 151, y: 429, width: 500, height: 20),
  font: .monospacedSystemFont(ofSize: 14, weight: .regular),
  color: white,
  alignment: .left
)

drawText(
  "2. Drag Xaga to a\nwritable folder.",
  in: NSRect(x: 240, y: 333, width: 220, height: 45),
  font: .systemFont(ofSize: 16, weight: .semibold),
  color: white
)

drawText(
  "3. In Terminal,",
  in: NSRect(x: 134, y: 256, width: 520, height: 22),
  font: .systemFont(ofSize: 16, weight: .semibold),
  color: white,
  alignment: .left
)
drawText(
  "xattr -dr com.apple.quarantine /path/to/Xaga.app",
  in: NSRect(x: 151, y: 235, width: 500, height: 20),
  font: .monospacedSystemFont(ofSize: 14, weight: .regular),
  color: white,
  alignment: .left
)
drawText(
  "4. Open the copied Xaga.app.",
  in: NSRect(x: 134, y: 194, width: 520, height: 22),
  font: .systemFont(ofSize: 16, weight: .semibold),
  color: white,
  alignment: .left
)
drawText(
  "Full instructions",
  in: NSRect(x: 134, y: 140, width: 520, height: 18),
  font: .systemFont(ofSize: 12, weight: .semibold),
  color: white,
  alignment: .left
)
drawText(
  "https://github.com/reedable/xaga/blob/develop/docs/installation.md",
  in: NSRect(x: 134, y: 121, width: 540, height: 20),
  font: .monospacedSystemFont(ofSize: 12, weight: .regular),
  color: white,
  alignment: .left
)

image.unlockFocus()

guard
  let tiff = image.tiffRepresentation,
  let bitmap = NSBitmapImageRep(data: tiff),
  let png = bitmap.representation(using: .png, properties: [:])
else {
  fputs("Unable to render DMG background\n", stderr)
  exit(1)
}

try FileManager.default.createDirectory(
  at: outputURL.deletingLastPathComponent(),
  withIntermediateDirectories: true
)
try png.write(to: outputURL)
