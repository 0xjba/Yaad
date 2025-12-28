import SwiftRs
import ScreenCaptureKit
import Vision
import AppKit
import ApplicationServices

// 1. Define a lightweight struct for JSON serialization
struct MetadataResult: Encodable {
    let app_name: String
    let title: String
    let url: String
    let error: String?
}

@_cdecl("fetch_metadata_only")
public func fetch_metadata_only() -> SRString {
    var appName = "Unknown"
    var windowTitle = ""
    var url = ""
    
    // A. Get Frontmost Application
    if let frontApp = NSWorkspace.shared.frontmostApplication {
        appName = frontApp.localizedName ?? "Unknown"
        
        // B. Get Window Title via Accessibility API (AXUIElement)
        let appRef = AXUIElementCreateApplication(frontApp.processIdentifier)
        var focusedWindow: AnyObject?
        
        // Get the focused window of the app
        let result = AXUIElementCopyAttributeValue(appRef, kAXFocusedWindowAttribute as CFString, &focusedWindow)
        
        if result == .success, let window = focusedWindow {
            let windowRef = window as! AXUIElement
            var titleRef: AnyObject?
            // Get the title of that window
            let titleResult = AXUIElementCopyAttributeValue(windowRef, kAXTitleAttribute as CFString, &titleRef)
            if titleResult == .success, let titleStr = titleRef as? String {
                windowTitle = titleStr
            }
        }
    }
    
    // C. Get Browser URL (Reuse your existing logic)
    // Only fetch if it's a known browser to save resources
    let browsers = ["Google Chrome", "Safari", "Arc", "Brave Browser", "Microsoft Edge", "Orion"]
    if browsers.contains(appName) {
        url = getBrowserURL(appName: appName)
    }
    
    // D. Serialize
    let metadata = MetadataResult(
        app_name: appName,
        title: windowTitle,
        url: url,
        error: nil
    )
    
    do {
        let jsonData = try JSONEncoder().encode(metadata)
        if let jsonString = String(data: jsonData, encoding: .utf8) {
            return SRString(jsonString)
        }
    } catch {
        return SRString("{\"error\": \"JSON Encoding Failed\"}")
    }
    
    return SRString("{\"error\": \"Unknown Failure\"}")
}

// NEW: Permission Check
@_cdecl("check_accessibility_permissions")
public func check_accessibility_permissions() -> Bool {
    return AXIsProcessTrusted()
}

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
