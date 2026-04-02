import AppKit
import Foundation
import ServiceManagement

private final class RainyAutoLaunchController {
    static let shared = RainyAutoLaunchController()

    func runtimeSupported() -> Bool {
        let bundle = Bundle.main
        guard
            let identifier = bundle.bundleIdentifier,
            !identifier.isEmpty,
            bundle.bundleURL.pathExtension == "app"
        else {
            return false
        }

        if #available(macOS 13.0, *) {
            return true
        }

        return false
    }

    func statusCode() -> Int32 {
        guard runtimeSupported() else {
            return -2
        }

        if #available(macOS 13.0, *) {
            switch SMAppService.mainApp.status {
            case .enabled:
                return 1
            case .requiresApproval:
                return 2
            case .notFound:
                return -1
            case .notRegistered:
                return 0
            @unknown default:
                return -1
            }
        }

        return -2
    }

    func setEnabled(_ enabled: Bool) -> Int32 {
        guard runtimeSupported() else {
            return -2
        }

        guard #available(macOS 13.0, *) else {
            return -2
        }

        do {
            if enabled {
                try SMAppService.mainApp.register()
            } else {
                try SMAppService.mainApp.unregister()
            }
            return statusCode()
        } catch {
            NSLog("RainyAutoLaunch register/unregister failed: %@", error.localizedDescription)
            return -1
        }
    }

    @discardableResult
    func openSystemSettings() -> Bool {
        guard runtimeSupported() else {
            return false
        }

        guard
            let url = URL(string: "x-apple.systempreferences:com.apple.LoginItems-Settings.extension")
        else {
            return false
        }

        return NSWorkspace.shared.open(url)
    }
}

@_cdecl("rainy_auto_launch_runtime_supported")
public func rainy_auto_launch_runtime_supported() -> Int32 {
    RainyAutoLaunchController.shared.runtimeSupported() ? 1 : 0
}

@_cdecl("rainy_auto_launch_status")
public func rainy_auto_launch_status() -> Int32 {
    RainyAutoLaunchController.shared.statusCode()
}

@_cdecl("rainy_auto_launch_set_enabled")
public func rainy_auto_launch_set_enabled(_ enabled: Int32) -> Int32 {
    RainyAutoLaunchController.shared.setEnabled(enabled != 0)
}

@_cdecl("rainy_auto_launch_open_system_settings")
public func rainy_auto_launch_open_system_settings() -> Int32 {
    RainyAutoLaunchController.shared.openSystemSettings() ? 1 : 0
}
