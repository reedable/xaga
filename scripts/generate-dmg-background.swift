#!/usr/bin/env swift

import AppKit
import Foundation

let canvasSize = NSSize(width: 700, height: 500)
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
  in: NSRect(x: 40, y: 448, width: 620, height: 38),
  font: .systemFont(ofSize: 28, weight: .semibold),
  color: white
)
drawText(
  "1. In Terminal, run shasum -a 256 on this DMG.\nCompare the result with the SHA-256 on GitHub Releases.",
  in: NSRect(x: 70, y: 375, width: 560, height: 48),
  font: .systemFont(ofSize: 15, weight: .medium),
  color: white
)

drawText(
  "2. Drag Xaga to a\nwritable folder.",
  in: NSRect(x: 240, y: 280, width: 220, height: 48),
  font: .systemFont(ofSize: 15, weight: .medium),
  color: white
)

drawText(
  "3. In Terminal,",
  in: NSRect(x: 65, y: 180, width: 570, height: 24),
  font: .systemFont(ofSize: 15, weight: .medium),
  color: white
)
drawText(
  "xattr -dr com.apple.quarantine /path/to/Xaga.app",
  in: NSRect(x: 40, y: 139, width: 620, height: 28),
  font: .monospacedSystemFont(ofSize: 14, weight: .medium),
  color: white
)
drawText(
  "4. Open the copied Xaga.app.",
  in: NSRect(x: 65, y: 108, width: 570, height: 25),
  font: .systemFont(ofSize: 16, weight: .semibold),
  color: white
)
drawText(
  "Full instructions: github.com/reedable/xaga  →  Installation",
  in: NSRect(x: 40, y: 58, width: 620, height: 24),
  font: .monospacedSystemFont(ofSize: 12, weight: .medium),
  color: white
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
