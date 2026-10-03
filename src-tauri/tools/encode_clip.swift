import AppKit
import AVFoundation
import Foundation

/// Encode a list of JPEG paths into an H.264 mp4 using AVFoundation (no ffmpeg).
/// Usage: waypoint-encode-clip <fps> <out.mp4> <frame1.jpg> [frame2.jpg ...]

func fail(_ message: String) -> Never {
  fputs("encode_clip: \(message)\n", stderr)
  exit(1)
}

guard CommandLine.arguments.count >= 4 else {
  fail("usage: waypoint-encode-clip <fps> <out.mp4> <frame.jpg>...")
}

guard let fps = Double(CommandLine.arguments[1]), fps >= 1 else {
  fail("invalid fps")
}

let outURL = URL(fileURLWithPath: CommandLine.arguments[2])
let framePaths = Array(CommandLine.arguments.dropFirst(3))
guard !framePaths.isEmpty else { fail("no frames") }

var images: [CGImage] = []
var width = 0
var height = 0
for path in framePaths {
  let url = URL(fileURLWithPath: path)
  guard let nsImage = NSImage(contentsOf: url),
        let tiff = nsImage.tiffRepresentation,
        let rep = NSBitmapImageRep(data: tiff),
        let cg = rep.cgImage
  else {
    fail("cannot read \(path)")
  }
  if width == 0 {
    width = cg.width
    height = cg.height
  }
  images.append(cg)
}

// Even dimensions for H.264
width = (width / 2) * 2
height = (height / 2) * 2
guard width > 0, height > 0 else { fail("bad frame size") }

try? FileManager.default.removeItem(at: outURL)

guard let writer = try? AVAssetWriter(outputURL: outURL, fileType: .mp4) else {
  fail("AVAssetWriter init failed")
}

let videoSettings: [String: Any] = [
  AVVideoCodecKey: AVVideoCodecType.h264,
  AVVideoWidthKey: width,
  AVVideoHeightKey: height,
]
let input = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
input.expectsMediaDataInRealTime = false

let attrs: [String: Any] = [
  kCVPixelBufferPixelFormatTypeKey as String: Int(kCVPixelFormatType_32BGRA),
  kCVPixelBufferWidthKey as String: width,
  kCVPixelBufferHeightKey as String: height,
]
let adaptor = AVAssetWriterInputPixelBufferAdaptor(
  assetWriterInput: input,
  sourcePixelBufferAttributes: attrs
)

guard writer.canAdd(input) else { fail("cannot add writer input") }
writer.add(input)

guard writer.startWriting() else {
  fail(writer.error?.localizedDescription ?? "startWriting failed")
}
writer.startSession(atSourceTime: .zero)

let frameDuration = CMTime(value: 1, timescale: CMTimeScale(max(1, Int32(fps.rounded()))))
var frameIndex: Int64 = 0

for cg in images {
  while !input.isReadyForMoreMediaData {
    Thread.sleep(forTimeInterval: 0.002)
  }
  var buffer: CVPixelBuffer?
  let status = CVPixelBufferCreate(
    kCFAllocatorDefault,
    width,
    height,
    kCVPixelFormatType_32BGRA,
    attrs as CFDictionary,
    &buffer
  )
  guard status == kCVReturnSuccess, let pixelBuffer = buffer else {
    fail("CVPixelBufferCreate failed")
  }

  CVPixelBufferLockBaseAddress(pixelBuffer, [])
  if let ctx = CGContext(
    data: CVPixelBufferGetBaseAddress(pixelBuffer),
    width: width,
    height: height,
    bitsPerComponent: 8,
    bytesPerRow: CVPixelBufferGetBytesPerRow(pixelBuffer),
    space: CGColorSpaceCreateDeviceRGB(),
    bitmapInfo: CGImageAlphaInfo.premultipliedFirst.rawValue | CGBitmapInfo.byteOrder32Little.rawValue
  ) {
    ctx.interpolationQuality = .medium
    ctx.draw(cg, in: CGRect(x: 0, y: 0, width: width, height: height))
  }
  CVPixelBufferUnlockBaseAddress(pixelBuffer, [])

  let time = CMTimeMultiply(frameDuration, multiplier: Int32(frameIndex))
  if !adaptor.append(pixelBuffer, withPresentationTime: time) {
    fail(writer.error?.localizedDescription ?? "append failed")
  }
  frameIndex += 1
}

input.markAsFinished()

let sem = DispatchSemaphore(value: 0)
writer.finishWriting {
  sem.signal()
}
_ = sem.wait(timeout: .now() + 60)

if writer.status != .completed {
  fail(writer.error?.localizedDescription ?? "finishWriting failed")
}

exit(0)
