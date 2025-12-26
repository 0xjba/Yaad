import SwiftRs
import ScreenCaptureKit
import Vision
import AppKit

@_cdecl("capture_active_window")
public func capture_active_window() -> SRString {
    let semaphore = DispatchSemaphore(value: 0)
    var result = ""
    
    Task {
        do {
            // 1. Define what we want to ignore (Ourself)
            let myPID = ProcessInfo.processInfo.processIdentifier
            
            // 2. Fetch all shareable content
            // We explicitly EXCLUDE desktop windows (wallpapers)
            let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: true)
            
            // 3. SMART SELECTION:
            // SCShareableContent returns windows sorted by Z-Order (Front to Back).
            // We iterate through them and pick the first one that:
            //   - Is NOT our app
            //   - Is on screen (visible)
            //   - Has a title (ignoring invisible overlay windows)
            //   - Is on the "Normal" window level (Layer 0) - This avoids Menu Bars, Docks, etc.
            if let targetWindow = content.windows.first(where: { window in
                return window.owningApplication?.processID != myPID &&
                       window.isOnScreen &&
                       window.windowLayer == 0 && // Standard application windows only
                       window.title != nil && 
                       !window.title!.isEmpty // Skip empty/ghost windows
            }) {
                
                let filter = SCContentFilter(desktopIndependentWindow: targetWindow)
                let config = SCStreamConfiguration()
                config.width = Int(targetWindow.frame.width)
                config.height = Int(targetWindow.frame.height)
                config.showsCursor = false
                config.captureResolution = .best
                
                // 4. One-Shot Capture (macOS 14+)
                if let image = try? await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config) {
                    let base64 = convertImageToBase64(image)
                    let ocrText = await performOCR(image: image)
                    
                    // Return JSON with both base64 and OCR text
                    let json: [String: String] = [
                        "image": base64,
                        "ocr": ocrText,
                        "app_name": targetWindow.owningApplication?.applicationName ?? "Unknown"
                    ]
                    
                    if let jsonData = try? JSONSerialization.data(withJSONObject: json),
                       let jsonString = String(data: jsonData, encoding: .utf8) {
                        result = jsonString
                    } else {
                        result = "ERROR: JSON serialization failed"
                    }
                } else {
                    result = "ERROR: Capture failed"
                }
            } else {
                result = "ERROR: No valid target window found behind Yaad"
            }
        } catch {
            result = "ERROR: \(error.localizedDescription)"
        }
        semaphore.signal()
    }
    
    semaphore.wait()
    return SRString(result)
}

func performOCR(image: CGImage) async -> String {
    return await withCheckedContinuation { continuation in
        let request = VNRecognizeTextRequest { (request, error) in
            guard let observations = request.results as? [VNRecognizedTextObservation] else {
                continuation.resume(returning: "")
                return
            }
            
            let recognizedText = observations.compactMap { observation in
                observation.topCandidates(1).first?.string
            }.joined(separator: "\n")
            
            continuation.resume(returning: recognizedText)
        }
        
        request.recognitionLevel = .accurate
        
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        do {
            try handler.perform([request])
        } catch {
            continuation.resume(returning: "")
        }
    }
}

func convertImageToBase64(_ image: CGImage) -> String {
    let uiImage = NSImage(cgImage: image, size: NSSize(width: image.width, height: image.height))
    guard let tiffData = uiImage.tiffRepresentation,
          let bitmapImage = NSBitmapImageRep(data: tiffData),
          let pngData = bitmapImage.representation(using: .png, properties: [:]) else {
        return ""
    }
    return pngData.base64EncodedString()
}
