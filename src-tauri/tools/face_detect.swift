import AppKit
import Foundation
import Vision

/// Cheap on-device face rect + head yaw via Apple Vision.
/// Usage: waypoint-face-detect <image-path>
/// Prints one JSON line:
///   {"face":bool,"center_y":number|null,"center_x":number|null,"yaw":number|null}
/// Vision space: origin bottom-left; center_y 0 = bottom, 1 = top.
/// yaw (radians): 0 = facing camera; |yaw| large ⇒ looking left/right (away).

let args = CommandLine.arguments
guard args.count >= 2 else {
    fputs("usage: waypoint-face-detect <image-path>\n", stderr)
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

let request = VNDetectFaceRectanglesRequest()
let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
do {
    try handler.perform([request])
} catch {
    fputs("vision failed: \(error)\n", stderr)
    exit(1)
}

let faces = request.results ?? []
if let best = faces.max(by: { $0.boundingBox.width * $0.boundingBox.height
    < $1.boundingBox.width * $1.boundingBox.height })
{
    let box = best.boundingBox
    let centerY = box.origin.y + box.height / 2.0
    let centerX = box.origin.x + box.width / 2.0
    if let yawNum = best.yaw {
        print(
            String(
                format:
                    "{\"face\":true,\"center_y\":%.6f,\"center_x\":%.6f,\"yaw\":%.6f}",
                centerY,
                centerX,
                yawNum.doubleValue
            )
        )
    } else {
        print(
            String(
                format:
                    "{\"face\":true,\"center_y\":%.6f,\"center_x\":%.6f,\"yaw\":null}",
                centerY,
                centerX
            )
        )
    }
} else {
    print("{\"face\":false,\"center_y\":null,\"center_x\":null,\"yaw\":null}")
}
