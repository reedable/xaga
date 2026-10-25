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
let muted = NSColor(calibratedWhite: 0.76, alpha: 1)
let accent = NSColor(calibratedRed: 0.68, green: 0.47, blue: 0.95, alpha: 1)
let warning = NSColor(calibratedRed: 1.0, green: 0.76, blue: 0.33, alpha: 1)

drawText(
  "Install Xaga safely",
  in: NSRect(x: 40, y: 438, width: 620, height: 42),
  font: .systemFont(ofSize: 28, weight: .semibold),
  color: white
)
drawText(
  "Do not open Xaga yet",
  in: NSRect(x: 40, y: 405, width: 620, height: 28),
  font: .systemFont(ofSize: 17, weight: .semibold),
  color: warning
)

let arrow = NSBezierPath()
arrow.lineWidth = 4
arrow.move(to: NSPoint(x: 285, y: 305))
arrow.line(to: NSPoint(x: 415, y: 305))
arrow.move(to: NSPoint(x: 397, y: 319))
arrow.line(to: NSPoint(x: 415, y: 305))
arrow.line(to: NSPoint(x: 397, y: 291))
accent.setStroke()
arrow.stroke()

drawText(
  "Copy Xaga to a writable folder you control",
  in: NSRect(x: 145, y: 185, width: 410, height: 30),
  font: .systemFont(ofSize: 15, weight: .medium),
  color: white
)
drawText(
  "Applications is optional and does not make an app trusted.",
  in: NSRect(x: 100, y: 159, width: 500, height: 24),
  font: .systemFont(ofSize: 13),
  color: muted
)

drawText(
  "Before launching: verify the SHA-256 shown on the GitHub release,\nthen follow the Terminal steps in the installation guide.",
  in: NSRect(x: 65, y: 104, width: 570, height: 64),
  font: .systemFont(ofSize: 15),
  color: white
)
drawText(
  "Do not use “Open Anyway”; it may become stuck.",
  in: NSRect(x: 65, y: 72, width: 570, height: 24),
  font: .systemFont(ofSize: 14, weight: .semibold),
  color: warning
)
drawText(
  "github.com/reedable/xaga  →  Installation",
  in: NSRect(x: 65, y: 37, width: 570, height: 24),
  font: .monospacedSystemFont(ofSize: 13, weight: .medium),
  color: accent
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
