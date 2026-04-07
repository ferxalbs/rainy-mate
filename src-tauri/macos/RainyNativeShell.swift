import AppKit
import Carbon
import Foundation

public typealias RainyNativeShellCallback = @convention(c) (_ action: UnsafePointer<CChar>?, _ payload: UnsafePointer<CChar>?) -> Void

private struct ShellRecentChat: Decodable {
    let chatId: String
    let workspaceId: String
    let title: String
    let updatedAt: String
    let messageCount: Int
    let isActive: Bool
}

private struct ShellSnapshot: Decodable {
    let available: Bool
    let workspaceName: String?
    let workspacePath: String?
    let pendingApprovalCount: Int
    let activeSessionCount: Int
    let quickDelegateBusy: Bool
    let recentChats: [ShellRecentChat]
}

private final class RainyNativeShellController: NSObject {
    static let shared = RainyNativeShellController()

    private var callback: RainyNativeShellCallback?
    private var statusItem: NSStatusItem?
    private var menu: NSMenu?
    private var panel: NSPanel?
    private var textView: NSTextView?
    private var summaryLabel: NSTextField?
    private var eventHandlerRef: EventHandlerRef?
    private var hotKeyRef: EventHotKeyRef?
    private var snapshot = ShellSnapshot(
        available: true,
        workspaceName: nil,
        workspacePath: nil,
        pendingApprovalCount: 0,
        activeSessionCount: 0,
        quickDelegateBusy: false,
        recentChats: []
    )

    func setCallback(_ callback: RainyNativeShellCallback?) {
        self.callback = callback
    }

    func initializeBridge() {
        guard Thread.isMainThread else {
            DispatchQueue.main.async { self.initializeBridge() }
            return
        }

        guard runtimeSupported() else {
            return
        }

        installStatusItemIfNeeded()
        _ = ensurePanel()
        registerHotkeyIfNeeded()
        rebuildMenu()
    }

    func runtimeSupported() -> Bool {
        if !Thread.isMainThread {
            return DispatchQueue.main.sync { self.runtimeSupported() }
        }

        return NSApp != nil
    }

    @discardableResult
    func showPalette() -> Bool {
        guard Thread.isMainThread else {
            return DispatchQueue.main.sync { self.showPalette() }
        }

        guard runtimeSupported(), let panel = ensurePanel() else {
            return false
        }

        summaryLabel?.stringValue = buildSummaryLine()
        NSApp.activate(ignoringOtherApps: true)
        panel.center()
        panel.makeKeyAndOrderFront(nil)
        panel.orderFrontRegardless()
        textView?.window?.makeFirstResponder(textView)
        return true
    }

    @discardableResult
    func updateSnapshot(json: String) -> Bool {
        guard Thread.isMainThread else {
            return DispatchQueue.main.sync { self.updateSnapshot(json: json) }
        }

        guard let data = json.data(using: .utf8),
              let decoded = try? JSONDecoder().decode(ShellSnapshot.self, from: data) else {
            return false
        }

        snapshot = decoded
        summaryLabel?.stringValue = buildSummaryLine()
        rebuildMenu()
        return true
    }

    private func installStatusItemIfNeeded() {
        guard statusItem == nil else { return }

        let statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        if let button = statusItem.button {
            button.title = "MaTE"
        }
        let menu = NSMenu()
        statusItem.menu = menu

        self.statusItem = statusItem
        self.menu = menu
    }

    private func rebuildMenu() {
        guard let menu, let button = statusItem?.button else { return }

        let suffix = buildStatusSuffix()
        button.title = suffix.isEmpty ? "MaTE" : "MaTE \(suffix)"

        menu.removeAllItems()

        let workspaceTitle = snapshot.workspaceName ?? "No workspace selected"
        let workspaceItem = NSMenuItem(title: workspaceTitle, action: nil, keyEquivalent: "")
        workspaceItem.isEnabled = false
        menu.addItem(workspaceItem)

        let summaryItem = NSMenuItem(title: buildSummaryLine(), action: nil, keyEquivalent: "")
        summaryItem.isEnabled = false
        menu.addItem(summaryItem)
        menu.addItem(NSMenuItem.separator())

        menu.addItem(makeMenuItem("Quick Palette", action: #selector(openPaletteFromMenu)))
        menu.addItem(makeMenuItem("Quick Ask", action: #selector(openQuickDelegate)))
        menu.addItem(makeMenuItem("Open Rainy MaTE", action: #selector(openMainWindow)))

        let approvalsTitle = snapshot.pendingApprovalCount > 0
            ? "Review Approvals (\(snapshot.pendingApprovalCount))"
            : "Review Approvals"
        let approvalsItem = makeMenuItem(approvalsTitle, action: #selector(reviewApprovals))
        approvalsItem.isEnabled = snapshot.pendingApprovalCount > 0
        menu.addItem(approvalsItem)

        if !snapshot.recentChats.isEmpty {
            menu.addItem(NSMenuItem.separator())
            let recentsHeader = NSMenuItem(title: "Recent Chats", action: nil, keyEquivalent: "")
            recentsHeader.isEnabled = false
            menu.addItem(recentsHeader)

            for chat in snapshot.recentChats {
                let item = NSMenuItem(
                    title: chat.isActive ? "● \(chat.title)" : chat.title,
                    action: #selector(openRecentChat(_:)),
                    keyEquivalent: ""
                )
                item.representedObject = "{\"workspaceId\":\"\(chat.workspaceId)\",\"chatId\":\"\(chat.chatId)\"}"
                menu.addItem(item)
            }
        }
    }

    private func makeMenuItem(_ title: String, action: Selector) -> NSMenuItem {
        let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
        item.target = self
        return item
    }

    private func buildStatusSuffix() -> String {
        var parts: [String] = []
        if snapshot.pendingApprovalCount > 0 {
            parts.append("A\(snapshot.pendingApprovalCount)")
        }
        if snapshot.activeSessionCount > 0 {
            parts.append("R\(snapshot.activeSessionCount)")
        }
        return parts.joined(separator: " ")
    }

    private func buildSummaryLine() -> String {
        let approvals = snapshot.pendingApprovalCount == 1
            ? "1 approval"
            : "\(snapshot.pendingApprovalCount) approvals"
        let runs = snapshot.activeSessionCount == 1
            ? "1 active run"
            : "\(snapshot.activeSessionCount) active runs"
        let busy = snapshot.quickDelegateBusy ? " · quick ask busy" : ""
        return "\(approvals) · \(runs)\(busy)"
    }

    private func ensurePanel() -> NSPanel? {
        if let panel {
            return panel
        }

        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 620, height: 320),
            styleMask: [.titled, .fullSizeContentView, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.titleVisibility = .hidden
        panel.titlebarAppearsTransparent = true
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.hidesOnDeactivate = false
        panel.collectionBehavior = [.moveToActiveSpace, .fullScreenAuxiliary]

        let effectView = NSVisualEffectView(frame: panel.contentView?.bounds ?? .zero)
        effectView.autoresizingMask = [.width, .height]
        effectView.material = .hudWindow
        effectView.blendingMode = .behindWindow
        effectView.state = .active

        let titleLabel = NSTextField(labelWithString: "Quick Palette")
        titleLabel.font = NSFont.systemFont(ofSize: 22, weight: .semibold)
        titleLabel.textColor = .white
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        let summaryLabel = NSTextField(labelWithString: buildSummaryLine())
        summaryLabel.font = NSFont.systemFont(ofSize: 12, weight: .regular)
        summaryLabel.textColor = NSColor.white.withAlphaComponent(0.65)
        summaryLabel.translatesAutoresizingMaskIntoConstraints = false

        let textView = NSTextView(frame: NSRect(x: 0, y: 0, width: 560, height: 120))
        textView.isRichText = false
        textView.drawsBackground = false
        textView.backgroundColor = .clear
        textView.textColor = .white
        textView.insertionPointColor = .white
        textView.font = NSFont.systemFont(ofSize: 15, weight: .regular)
        textView.textContainerInset = NSSize(width: 12, height: 12)

        let scrollView = NSScrollView()
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.hasVerticalScroller = true
        scrollView.drawsBackground = false
        scrollView.documentView = textView

        let askButton = NSButton(title: "Ask MaTE", target: self, action: #selector(submitPrompt))
        askButton.bezelStyle = .rounded
        askButton.translatesAutoresizingMaskIntoConstraints = false

        let openButton = NSButton(title: "Open App", target: self, action: #selector(openMainWindow))
        openButton.bezelStyle = .rounded
        openButton.translatesAutoresizingMaskIntoConstraints = false

        let approvalsButton = NSButton(title: "Review Approvals", target: self, action: #selector(reviewApprovals))
        approvalsButton.bezelStyle = .rounded
        approvalsButton.translatesAutoresizingMaskIntoConstraints = false

        let content = NSView(frame: effectView.bounds)
        content.translatesAutoresizingMaskIntoConstraints = false
        effectView.addSubview(content)
        panel.contentView = effectView

        [titleLabel, summaryLabel, scrollView, askButton, openButton, approvalsButton].forEach {
            content.addSubview($0)
        }

        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: effectView.leadingAnchor),
            content.trailingAnchor.constraint(equalTo: effectView.trailingAnchor),
            content.topAnchor.constraint(equalTo: effectView.topAnchor),
            content.bottomAnchor.constraint(equalTo: effectView.bottomAnchor),

            titleLabel.topAnchor.constraint(equalTo: content.topAnchor, constant: 28),
            titleLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor, constant: 28),

            summaryLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 8),
            summaryLabel.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),

            scrollView.topAnchor.constraint(equalTo: summaryLabel.bottomAnchor, constant: 20),
            scrollView.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: content.trailingAnchor, constant: -28),
            scrollView.heightAnchor.constraint(equalToConstant: 130),

            askButton.topAnchor.constraint(equalTo: scrollView.bottomAnchor, constant: 18),
            askButton.leadingAnchor.constraint(equalTo: titleLabel.leadingAnchor),

            openButton.centerYAnchor.constraint(equalTo: askButton.centerYAnchor),
            openButton.leadingAnchor.constraint(equalTo: askButton.trailingAnchor, constant: 10),

            approvalsButton.centerYAnchor.constraint(equalTo: askButton.centerYAnchor),
            approvalsButton.leadingAnchor.constraint(equalTo: openButton.trailingAnchor, constant: 10),
        ])

        self.panel = panel
        self.textView = textView
        self.summaryLabel = summaryLabel
        return panel
    }

    private func registerHotkeyIfNeeded() {
        guard hotKeyRef == nil, eventHandlerRef == nil else { return }

        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let status = InstallEventHandler(
            GetApplicationEventTarget(),
            { _, eventRef, _ in
                var hotKeyID = EventHotKeyID()
                let err = GetEventParameter(
                    eventRef,
                    EventParamName(kEventParamDirectObject),
                    EventParamType(typeEventHotKeyID),
                    nil,
                    MemoryLayout<EventHotKeyID>.size,
                    nil,
                    &hotKeyID
                )
                guard err == noErr else { return noErr }
                RainyNativeShellController.shared.sendCallback(action: "hotkey", payload: nil)
                _ = RainyNativeShellController.shared.showPalette()
                return noErr
            },
            1,
            &eventType,
            nil,
            &eventHandlerRef
        )
        guard status == noErr else { return }

        var hotKeyID = EventHotKeyID(signature: OSType(0x524D5445), id: UInt32(2))
        RegisterEventHotKey(UInt32(kVK_Space), UInt32(cmdKey | optionKey), hotKeyID, GetApplicationEventTarget(), 0, &hotKeyRef)
    }

    private func sendCallback(action: String, payload: String?) {
        guard let callback else { return }
        action.withCString { actionPtr in
            if let payload {
                payload.withCString { payloadPtr in
                    callback(actionPtr, payloadPtr)
                }
            } else {
                callback(actionPtr, nil)
            }
        }
    }

    @objc
    private func openPaletteFromMenu() {
        sendCallback(action: "show_palette", payload: nil)
        _ = showPalette()
    }

    @objc
    private func openQuickDelegate() {
        sendCallback(action: "open_quick_delegate", payload: nil)
    }

    @objc
    private func openMainWindow() {
        sendCallback(action: "open_main", payload: nil)
    }

    @objc
    private func reviewApprovals() {
        sendCallback(action: "review_approvals", payload: nil)
    }

    @objc
    private func submitPrompt() {
        let prompt = textView?.string.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !prompt.isEmpty else { return }
        sendCallback(action: "submit_prompt", payload: prompt)
        textView?.string = ""
        panel?.orderOut(nil)
    }

    @objc
    private func openRecentChat(_ sender: NSMenuItem) {
        guard let payload = sender.representedObject as? String else { return }
        sendCallback(action: "resume_chat", payload: payload)
    }
}

@_cdecl("rainy_native_shell_bridge_initialize")
public func rainy_native_shell_bridge_initialize(_ callback: RainyNativeShellCallback?) {
    DispatchQueue.main.async {
        RainyNativeShellController.shared.setCallback(callback)
        RainyNativeShellController.shared.initializeBridge()
    }
}

@_cdecl("rainy_native_shell_bridge_runtime_supported")
public func rainy_native_shell_bridge_runtime_supported() -> Int32 {
    RainyNativeShellController.shared.runtimeSupported() ? 1 : 0
}

@_cdecl("rainy_native_shell_bridge_show_palette")
public func rainy_native_shell_bridge_show_palette() -> Int32 {
    RainyNativeShellController.shared.showPalette() ? 1 : 0
}

@_cdecl("rainy_native_shell_bridge_update_snapshot")
public func rainy_native_shell_bridge_update_snapshot(_ json: UnsafePointer<CChar>?) -> Int32 {
    guard let json else { return 0 }
    return RainyNativeShellController.shared.updateSnapshot(json: String(cString: json)) ? 1 : 0
}
