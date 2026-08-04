import AppKit
import Foundation

let size = 1024.0
let outPath = CommandLine.arguments[1]

guard let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil,
    pixelsWide: Int(size), pixelsHigh: Int(size),
    bitsPerSample: 8, samplesPerPixel: 4,
    hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB,
    bytesPerRow: 0, bitsPerPixel: 0
) else { exit(1) }

NSGraphicsContext.saveGraphicsState()
let ctx = NSGraphicsContext(bitmapImageRep: rep)!
NSGraphicsContext.current = ctx
let cg = ctx.cgContext

// Rounded-square dark tile with vertical gradient (matches app theme)
let rect = CGRect(x: 0, y: 0, width: size, height: size)
let path = CGPath(roundedRect: rect, cornerWidth: 224, cornerHeight: 224, transform: nil)
cg.addPath(path)
cg.clip()
let colors = [
    NSColor(calibratedRed: 0.086, green: 0.110, blue: 0.150, alpha: 1).cgColor,
    NSColor(calibratedRed: 0.039, green: 0.051, blue: 0.071, alpha: 1).cgColor
] as CFArray
let grad = CGGradient(colorsSpace: CGColorSpaceCreateDeviceRGB(),
                      colors: colors, locations: [0, 1])!
cg.drawLinearGradient(grad, start: CGPoint(x: 0, y: size),
                      end: CGPoint(x: 0, y: 0), options: [])

// Centered emoji (Core Text substitutes Apple Color Emoji automatically)
let emoji = "🦅"
let para = NSMutableParagraphStyle()
para.alignment = .center
let attrs: [NSAttributedString.Key: Any] = [
    .font: NSFont.systemFont(ofSize: 640),
    .paragraphStyle: para
]
let str = NSAttributedString(string: emoji, attributes: attrs)
let ts = str.size()
str.draw(in: CGRect(x: (size - ts.width) / 2,
                    y: (size - ts.height) / 2,
                    width: ts.width, height: ts.height))

NSGraphicsContext.restoreGraphicsState()

guard let png = rep.representation(using: .png, properties: [:]) else { exit(1) }
try! png.write(to: URL(fileURLWithPath: outPath))
print("wrote \(outPath)")
