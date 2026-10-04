import AVFoundation
import Foundation
import Speech

/// Short mic capture + on-device (when available) Speech recognition for Waypoint.
/// Usage:
///   waypoint-listen-once <seconds>
///   waypoint-listen-once --check-mic
/// Prints a single JSON object to stdout.

struct Payload: Encodable {
    let text: String
    let engine: String
    let audio_path: String?
    let note: String
    let error: String?
}

func emit(_ payload: Payload) {
    let encoder = JSONEncoder()
    encoder.outputFormatting = []
    if let data = try? encoder.encode(payload),
       let line = String(data: data, encoding: .utf8)
    {
        print(line)
    } else {
        print(
            #"{"text":"","engine":"macos-speech","audio_path":null,"note":"","error":"json encode failed"}"#
        )
    }
    fflush(stdout)
}

func micStatusLabel() -> String {
    switch AVCaptureDevice.authorizationStatus(for: .audio) {
    case .authorized: return "authorized"
    case .denied: return "denied"
    case .restricted: return "restricted"
    case .notDetermined: return "notDetermined"
    @unknown default: return "unknown"
    }
}

func requestMicAccess() -> Bool {
    let status = AVCaptureDevice.authorizationStatus(for: .audio)
    if status == .authorized { return true }
    if status == .denied || status == .restricted { return false }
    let sem = DispatchSemaphore(value: 0)
    var granted = false
    AVCaptureDevice.requestAccess(for: .audio) { ok in
        granted = ok
        sem.signal()
    }
    _ = sem.wait(timeout: .now() + 60)
    return granted
}

func requestSpeechAccess() -> SFSpeechRecognizerAuthorizationStatus {
    let current = SFSpeechRecognizer.authorizationStatus()
    if current != .notDetermined { return current }
    let sem = DispatchSemaphore(value: 0)
    var status = current
    SFSpeechRecognizer.requestAuthorization { next in
        status = next
        sem.signal()
    }
    _ = sem.wait(timeout: .now() + 60)
    return status
}

func recordWav(to url: URL, seconds: Double) throws {
    let settings: [String: Any] = [
        AVFormatIDKey: Int(kAudioFormatLinearPCM),
        AVSampleRateKey: 16_000,
        AVNumberOfChannelsKey: 1,
        AVLinearPCMBitDepthKey: 16,
        AVLinearPCMIsFloatKey: false,
        AVLinearPCMIsBigEndianKey: false,
    ]
    let recorder = try AVAudioRecorder(url: url, settings: settings)
    recorder.isMeteringEnabled = false
    guard recorder.prepareToRecord() else {
        throw NSError(
            domain: "WaypointVoice",
            code: 1,
            userInfo: [NSLocalizedDescriptionKey: "Could not prepare microphone recorder."]
        )
    }
    guard recorder.record() else {
        throw NSError(
            domain: "WaypointVoice",
            code: 2,
            userInfo: [
                NSLocalizedDescriptionKey:
                    "Could not start microphone. Check System Settings → Privacy & Security → Microphone."
            ]
        )
    }
    Thread.sleep(forTimeInterval: seconds)
    recorder.stop()
}

func transcribe(url: URL) throws -> (String, String) {
    guard let recognizer = SFSpeechRecognizer(locale: Locale(identifier: "en-US")) else {
        throw NSError(
            domain: "WaypointVoice",
            code: 3,
            userInfo: [NSLocalizedDescriptionKey: "Speech recognizer unavailable for en-US."]
        )
    }
    guard recognizer.isAvailable else {
        throw NSError(
            domain: "WaypointVoice",
            code: 4,
            userInfo: [
                NSLocalizedDescriptionKey:
                    "Speech recognition is unavailable right now. Try again in a moment."
            ]
        )
    }

    let request = SFSpeechURLRecognitionRequest(url: url)
    request.shouldReportPartialResults = false
    var engine = "macos-speech"
    if recognizer.supportsOnDeviceRecognition {
        request.requiresOnDeviceRecognition = true
        engine = "macos-speech-on-device"
    }

    let sem = DispatchSemaphore(value: 0)
    var finalText = ""
    var taskError: Error?

    let task = recognizer.recognitionTask(with: request) { result, error in
        if let error {
            taskError = error
            sem.signal()
            return
        }
        guard let result else { return }
        if result.isFinal {
            finalText = result.bestTranscription.formattedString
            sem.signal()
        }
    }

    // Drive the run loop so Speech callbacks can fire.
    let deadline = Date().addingTimeInterval(45)
    while Date() < deadline {
        if sem.wait(timeout: .now() + 0.05) == .success {
            break
        }
        RunLoop.current.run(mode: .default, before: Date(timeIntervalSinceNow: 0.05))
    }

    task.cancel()

    if let taskError {
        throw taskError
    }
    return (finalText.trimmingCharacters(in: .whitespacesAndNewlines), engine)
}

func requireSpeechAuthorized() {
    let speechStatus = requestSpeechAccess()
    guard speechStatus == .authorized else {
        let hint: String
        switch speechStatus {
        case .denied:
            hint =
                "Speech Recognition permission denied. Enable Waypoint in System Settings → Privacy & Security → Speech Recognition."
        case .restricted:
            hint = "Speech Recognition is restricted on this Mac."
        default:
            hint = "Speech Recognition was not authorized."
        }
        emit(
            Payload(
                text: "",
                engine: "macos-speech",
                audio_path: nil,
                note: "",
                error: hint
            )
        )
        exit(3)
    }
}

func emitTranscription(of wav: URL, bytesNote: String) {
    do {
        let (text, engine) = try transcribe(url: wav)
        if text.isEmpty {
            emit(
                Payload(
                    text: "",
                    engine: engine,
                    audio_path: wav.path,
                    note: "\(bytesNote); no speech detected.",
                    error: nil
                )
            )
            return
        }
        emit(
            Payload(
                text: text,
                engine: engine,
                audio_path: wav.path,
                note: "\(bytesNote) · \(engine)",
                error: nil
            )
        )
    } catch {
        emit(
            Payload(
                text: "",
                engine: "macos-speech",
                audio_path: wav.path,
                note: bytesNote,
                error: error.localizedDescription
            )
        )
        exit(1)
    }
}

func main() {
    let args = Array(CommandLine.arguments.dropFirst())
    if args.contains("--check-mic") {
        print(micStatusLabel())
        return
    }

    if let idx = args.firstIndex(of: "--transcribe-file"),
       args.indices.contains(idx + 1)
    {
        requireSpeechAuthorized()
        let path = args[idx + 1]
        let wav = URL(fileURLWithPath: path)
        guard FileManager.default.fileExists(atPath: path) else {
            emit(
                Payload(
                    text: "",
                    engine: "macos-speech",
                    audio_path: path,
                    note: "",
                    error: "Audio file not found: \(path)"
                )
            )
            exit(4)
        }
        let attrs = try? FileManager.default.attributesOfItem(atPath: path)
        let bytes = (attrs?[.size] as? NSNumber)?.intValue ?? 0
        emitTranscription(of: wav, bytesNote: "Transcribing \(bytes) bytes")
        return
    }

    let seconds = max(2.0, min(12.0, Double(args.first ?? "4") ?? 4.0))

    if !requestMicAccess() {
        emit(
            Payload(
                text: "",
                engine: "macos-speech",
                audio_path: nil,
                note: "",
                error:
                    "Microphone permission denied. Enable Waypoint in System Settings → Privacy & Security → Microphone."
            )
        )
        exit(2)
    }

    requireSpeechAuthorized()

    let dir = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
        .appendingPathComponent("waypoint-voice", isDirectory: true)
    try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
    let wav = dir.appendingPathComponent("listen-\(Int(Date().timeIntervalSince1970 * 1000)).wav")

    do {
        try recordWav(to: wav, seconds: seconds)
        let attrs = try FileManager.default.attributesOfItem(atPath: wav.path)
        let bytes = attrs[.size] as? NSNumber ?? 0
        if bytes.intValue < 1000 {
            emit(
                Payload(
                    text: "",
                    engine: "macos-speech",
                    audio_path: wav.path,
                    note: "Recorded \(bytes) bytes.",
                    error:
                        "Mic capture was nearly empty. Check the input device and Microphone permission."
                )
            )
            exit(4)
        }
        emitTranscription(of: wav, bytesNote: "Recorded \(bytes) bytes via AVAudioRecorder")
    } catch {
        emit(
            Payload(
                text: "",
                engine: "macos-speech",
                audio_path: wav.path,
                note: "",
                error: error.localizedDescription
            )
        )
        exit(1)
    }
}

main()
