// Gibt die Fenster-Kennung einer laufenden Anwendung aus.
//
// Gebraucht von scripts/screenshot.mjs: Mit der Kennung nimmt `screencapture`
// genau ein Fenster auf - so kann nichts anderes vom Bildschirm mit ins Bild
// geraten, auch kein privates Fenster im Hintergrund.

import CoreGraphics
import Foundation

let wanted = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "Gesture TimeTrack"
let options: CGWindowListOption = [.optionOnScreenOnly, .excludeDesktopElements]

guard let windows = CGWindowListCopyWindowInfo(options, kCGNullWindowID) as? [[String: Any]] else {
    FileHandle.standardError.write("Fensterliste nicht lesbar.\n".data(using: .utf8)!)
    exit(1)
}

for window in windows {
    let owner = window[kCGWindowOwnerName as String] as? String ?? ""
    let name = window[kCGWindowName as String] as? String ?? ""
    let bounds = window[kCGWindowBounds as String] as? [String: Any] ?? [:]
    let width = bounds["Width"] as? Double ?? 0
    let height = bounds["Height"] as? Double ?? 0

    // Winzige Hilfsfenster überspringen.
    guard owner.contains(wanted) || name.contains(wanted), width > 120, height > 120 else {
        continue
    }
    print(window[kCGWindowNumber as String] as? Int ?? 0)
    exit(0)
}

FileHandle.standardError.write("Kein Fenster gefunden für: \(wanted)\n".data(using: .utf8)!)
exit(2)
