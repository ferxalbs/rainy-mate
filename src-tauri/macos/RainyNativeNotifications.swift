import AppKit
import Foundation
import UserNotifications

public typealias RainyNotificationCallback = @convention(c) (_ action: UnsafePointer<CChar>?, _ commandId: UnsafePointer<CChar>?) -> Void

private final class RainyNotificationDelegate: NSObject, UNUserNotificationCenterDelegate {
    static let shared = RainyNotificationDelegate()

    private var callback: RainyNotificationCallback?
    private var didRegisterLaunchObserver = false
    private var isConfigured = false
    private var isUnsupportedRuntime = false

    func setCallback(_ callback: RainyNotificationCallback?) {
        self.callback = callback
    }

    func initializeBridge() {
        if !Thread.isMainThread {
            DispatchQueue.main.async {
                self.initializeBridge()
            }
            return
        }

        if !didRegisterLaunchObserver {
            NotificationCenter.default.addObserver(
                self,
                selector: #selector(handleDidFinishLaunching),
                name: NSApplication.didFinishLaunchingNotification,
                object: nil
            )
            didRegisterLaunchObserver = true
        }

        configureIfPossible()
    }

    func runtimeSupported() -> Bool {
        if !Thread.isMainThread {
            return DispatchQueue.main.sync {
                self.runtimeSupported()
            }
        }

        if isUnsupportedRuntime {
            return false
        }

        let bundle = Bundle.main
        guard
            let identifier = bundle.bundleIdentifier,
            !identifier.isEmpty,
            bundle.bundleURL.pathExtension == "app"
        else {
            isUnsupportedRuntime = true
            return false
        }

        return true
    }

    func currentNotificationCenter() -> UNUserNotificationCenter? {
        if !Thread.isMainThread {
            return nil
        }

        guard runtimeSupported() else {
            return nil
        }

        configureIfPossible()
        guard isConfigured else {
            return nil
        }

        return UNUserNotificationCenter.current()
    }

    @objc
    private func handleDidFinishLaunching() {
        configureIfPossible()
    }

    private func configureIfPossible() {
        guard Thread.isMainThread else {
            DispatchQueue.main.async {
                self.configureIfPossible()
            }
            return
        }

        guard !isConfigured else {
            return
        }

        guard runtimeSupported() else {
            return
        }

        guard let app = NSApp, app.isRunning else {
            return
        }

        let center = UNUserNotificationCenter.current()
        center.delegate = self
        registerCategories()
        isConfigured = true
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        willPresent notification: UNNotification,
        withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
    ) {
        completionHandler([.banner, .sound, .list])
    }

    func userNotificationCenter(
        _ center: UNUserNotificationCenter,
        didReceive response: UNNotificationResponse,
        withCompletionHandler completionHandler: @escaping () -> Void
    ) {
        let actionIdentifier = response.actionIdentifier
        let commandId = response.notification.request.content.userInfo["commandId"] as? String

        switch actionIdentifier {
        case "RAINY_AIRLOCK_APPROVE":
            sendCallback(action: "approve", commandId: commandId)
        case "RAINY_AIRLOCK_REJECT":
            sendCallback(action: "reject", commandId: commandId)
        case "RAINY_AGENT_OPEN":
            sendCallback(action: "open", commandId: commandId)
        case UNNotificationDefaultActionIdentifier:
            sendCallback(action: "open", commandId: commandId)
        default:
            sendCallback(action: actionIdentifier, commandId: commandId)
        }

        DispatchQueue.main.async {
            NSApp.activate(ignoringOtherApps: true)
        }
        completionHandler()
    }

    private func sendCallback(action: String, commandId: String?) {
        guard let callback else { return }
        action.withCString { actionPtr in
            if let commandId {
                commandId.withCString { commandPtr in
                    callback(actionPtr, commandPtr)
                }
            } else {
                callback(actionPtr, nil)
            }
        }
    }
}

private func registerCategories() {
    let approve = UNNotificationAction(
        identifier: "RAINY_AIRLOCK_APPROVE",
        title: "Approve",
        options: []
    )
    let reject = UNNotificationAction(
        identifier: "RAINY_AIRLOCK_REJECT",
        title: "Reject",
        options: [.destructive]
    )
    let category = UNNotificationCategory(
        identifier: "RAINY_AIRLOCK_CATEGORY",
        actions: [approve, reject],
        intentIdentifiers: [],
        options: [.customDismissAction]
    )
    let openSession = UNNotificationAction(
        identifier: "RAINY_AGENT_OPEN",
        title: "Open Session",
        options: [.foreground]
    )
    let agentCategory = UNNotificationCategory(
        identifier: "RAINY_AGENT_CATEGORY",
        actions: [openSession],
        intentIdentifiers: [],
        options: [.customDismissAction]
    )

    UNUserNotificationCenter.current().setNotificationCategories([category, agentCategory])
}

@_cdecl("rainy_notification_bridge_initialize")
public func rainy_notification_bridge_initialize(_ callback: RainyNotificationCallback?) {
    DispatchQueue.main.async {
        RainyNotificationDelegate.shared.setCallback(callback)
        RainyNotificationDelegate.shared.initializeBridge()
    }
}

@_cdecl("rainy_notification_bridge_runtime_supported")
public func rainy_notification_bridge_runtime_supported() -> Int32 {
    RainyNotificationDelegate.shared.runtimeSupported() ? 1 : 0
}

@_cdecl("rainy_notification_bridge_request_authorization")
public func rainy_notification_bridge_request_authorization() -> Int32 {
    guard RainyNotificationDelegate.shared.runtimeSupported() else {
        return -2
    }

    let semaphore = DispatchSemaphore(value: 0)
    var result: Int32 = 0

    DispatchQueue.main.async {
        guard let center = RainyNotificationDelegate.shared.currentNotificationCenter() else {
            semaphore.signal()
            return
        }

        center.requestAuthorization(options: [.alert, .badge, .sound]) { granted, _ in
            result = granted ? 1 : 0
            semaphore.signal()
        }
    }

    semaphore.wait()
    return result
}

@_cdecl("rainy_notification_bridge_authorization_status")
public func rainy_notification_bridge_authorization_status() -> Int32 {
    guard RainyNotificationDelegate.shared.runtimeSupported() else {
        return -2
    }

    let semaphore = DispatchSemaphore(value: 0)
    var statusValue: Int32 = 0

    DispatchQueue.main.async {
        guard let center = RainyNotificationDelegate.shared.currentNotificationCenter() else {
            semaphore.signal()
            return
        }

        center.getNotificationSettings { settings in
            switch settings.authorizationStatus {
            case .authorized, .provisional, .ephemeral:
                statusValue = 1
            case .denied:
                statusValue = -1
            case .notDetermined:
                statusValue = 0
            @unknown default:
                statusValue = 0
            }
            semaphore.signal()
        }
    }

    semaphore.wait()
    return statusValue
}

@_cdecl("rainy_notification_bridge_send")
public func rainy_notification_bridge_send(
    _ title: UnsafePointer<CChar>?,
    _ body: UnsafePointer<CChar>?,
    _ commandId: UnsafePointer<CChar>?,
    _ categoryId: UnsafePointer<CChar>?
) -> Int32 {
    guard let title, let body else { return 0 }
    guard RainyNotificationDelegate.shared.runtimeSupported() else { return -2 }

    let titleString = String(cString: title)
    let bodyString = String(cString: body)
    let commandIdString = commandId.map { String(cString: $0) }
    let categoryString = categoryId.map { String(cString: $0) }

    let content = UNMutableNotificationContent()
    content.title = titleString
    content.body = bodyString
    content.sound = .default

    if let categoryString {
        content.categoryIdentifier = categoryString
    }
    if let commandIdString {
        content.userInfo["commandId"] = commandIdString
        content.threadIdentifier = commandIdString
    }

    let request = UNNotificationRequest(
        identifier: commandIdString ?? UUID().uuidString,
        content: content,
        trigger: nil
    )

    let semaphore = DispatchSemaphore(value: 0)
    var result: Int32 = 0

    DispatchQueue.main.async {
        guard let center = RainyNotificationDelegate.shared.currentNotificationCenter() else {
            semaphore.signal()
            return
        }

        center.add(request) { error in
            result = error == nil ? 1 : 0
            semaphore.signal()
        }
    }

    semaphore.wait()
    return result
}

@_cdecl("rainy_notification_bridge_activate_app")
public func rainy_notification_bridge_activate_app() {
    DispatchQueue.main.async {
        NSApp.activate(ignoringOtherApps: true)
    }
}
