import AppKit
import Foundation
import Vision

/// Fast on-device OCR via Apple Vision (.fast = lower CPU).
/// Usage: waypoint-ocr <image-path>
/// Prints recognized text to stdout.

let args = CommandLine.arguments
guard args.count >= 2 else {
    fputs("usage: waypoint-ocr <image-path>\n", stderr)
    exit(2)
}

let path = args[1]
let url = URL(fileURLWithPath: path)
guard let image = NSImage(contentsOf: url) else {
    fputs("could not load image\n", stderr)
    exit(1)
}
guard let tiff = image.tiffRepresentation,
      let rep = NSBitmapImageRep(data: tiff),
      let cgImage = rep.cgImage
else {
    fputs("could not decode image\n", stderr)
    exit(1)
}

let request = VNRecognizeTextRequest()
request.recognitionLevel = .fast
request.usesLanguageCorrection = false
request.recognitionLanguages = ["en-US"]

let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
do {
    try handler.perform([request])
} catch {
    fputs("vision failed: \(error)\n", stderr)
    exit(1)
}

let observations = request.results ?? []
var lines: [String] = []
for obs in observations {
    if let candidate = obs.topCandidates(1).first, candidate.confidence >= 0.35 {
        let t = candidate.string.trimmingCharacters(in: .whitespacesAndNewlines)
        if !t.isEmpty {
            lines.append(t)
        }
    }
}

print(lines.joined(separator: "\n"))
