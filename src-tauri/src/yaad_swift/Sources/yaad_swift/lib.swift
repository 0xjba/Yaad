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
            let myPID = ProcessInfo.processInfo.processIdentifier
            
            // 1. Fetch Content
            let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
            
            guard let mainDisplay = content.displays.first else {
                result = "ERROR: No display found"
                semaphore.signal()
                return
            }
            
            // 2. Identify "Yaad" (Us) so we can hide ourselves
            let myApp = content.applications.first(where: { $0.processID == myPID })
            let appsToExclude = myApp != nil ? [myApp!] : []
            
            // 3. Define the Filter
            // FIX: Changed 'excludingWindows' to 'exceptingWindows' to match macOS API
            let filter = SCContentFilter(display: mainDisplay,
                                         excludingApplications: appsToExclude,
                                         exceptingWindows: [])
            
            // 4. Configure
            let config = SCStreamConfiguration()
            config.width = mainDisplay.width
            config.height = mainDisplay.height
            config.showsCursor = false
            config.captureResolution = .best
            
            // 5. METADATA LOOKUP (App Name + URL)
            var activeAppName = "Unknown"
            var activeAppTitle = ""
            var activeAppURL = ""
            
            // Find the frontmost window that IS NOT Yaad
            if let frontWindow = content.windows.first(where: { window in
                guard let app = window.owningApplication,
                      app.processID != myPID,
                      window.isOnScreen,
                      window.windowLayer == 0 else { return false }
                
                // Must be a standard app (ignore notification center, dock, etc)
                if let runningApp = NSRunningApplication(processIdentifier: app.processID),
                   runningApp.activationPolicy == .regular {
                    return true
                }
                return false
            }) {
                activeAppName = frontWindow.owningApplication?.applicationName ?? "Unknown"
                activeAppTitle = frontWindow.title ?? ""
                
                // 6. FETCH URL
                activeAppURL = getBrowserURL(appName: activeAppName)
            }
            
            // 7. EXECUTE CAPTURE
            if let image = try? await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config) {
                let base64 = convertImageToBase64(image)
                let ocrText = await performOCR(image: image)
                
                let json: [String: String] = [
                    "image": base64,
                    "ocr": ocrText,
                    "app_name": activeAppName,
                    "title": activeAppTitle,
                    "url": activeAppURL // <--- Include in JSON
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
            
        } catch {
            result = "ERROR: \(error.localizedDescription)"
        }
        semaphore.signal()
    }
    
    semaphore.wait()
    return SRString(result)
}

// --- HELPER: APPLESCRIPT URL FETCHER ---
func getBrowserURL(appName: String) -> String {
    let script: String
    
    // Customize script based on the browser
    switch appName {
    case "Google Chrome", "Brave Browser", "Microsoft Edge", "Arc":
        script = "tell application \"\(appName)\" to return URL of active tab of front window"
    case "Safari":
        script = "tell application \"Safari\" to return URL of current tab of front window"
    default:
        return "" // Not a supported browser
    }
    
    var error: NSDictionary?
    if let appleScript = NSAppleScript(source: script) {
        let output = appleScript.executeAndReturnError(&error)
        if error == nil {
            return output.stringValue ?? ""
        }
    }
    return ""
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
        request.usesLanguageCorrection = true
        let handler = VNImageRequestHandler(cgImage: image, options: [:])
        do { try handler.perform([request]) } catch { continuation.resume(returning: "") }
    }
}

func convertImageToBase64(_ image: CGImage) -> String {
    let uiImage = NSImage(cgImage: image, size: NSSize(width: image.width, height: image.height))
    guard let tiffData = uiImage.tiffRepresentation,
          let bitmapImage = NSBitmapImageRep(data: tiffData),
          let pngData = bitmapImage.representation(using: .png, properties: [:]) else { return "" }
    return pngData.base64EncodedString()
}
