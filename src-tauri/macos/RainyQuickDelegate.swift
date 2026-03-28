import AppKit
import Carbon
import Foundation

public typealias RainyQuickDelegateCallback = @convention(c) (_ action: UnsafePointer<CChar>?, _ payload: UnsafePointer<CChar>?) -> Void

private final class RainyQuickDelegateController: NSObject {
    static let shared = RainyQuickDelegateController()

    private var callback: RainyQuickDelegateCallback?
    private var statusItem: NSStatusItem?
    private var statusMenu: NSMenu?
    private var panel: NSPanel?
    private var textView: NSTextView?
    private var statusLabel: NSTextField?
    private var submitButton: NSButton?
    private var cancelButton: NSButton?
    private var eventHandlerRef: EventHandlerRef?
    private var hotKeyRef: EventHotKeyRef?

    func setCallback(_ callback: RainyQuickDelegateCallback?) {
        self.callback = callback
    }

    func initializeBridge() {
        if !Thread.isMainThread {
            DispatchQueue.main.async {
                self.initializeBridge()
            }
            return
        }

        guard runtimeSupported() else {
            return
        }

        installStatusItemIfNeeded()
        _ = ensurePanel()
        registerHotkeyIfNeeded()
    }

    func runtimeSupported() -> Bool {
        if !Thread.isMainThread {
            return DispatchQueue.main.sync {
                self.runtimeSupported()
            }
        }

        return NSApp != nil
    }

    @discardableResult
    func showPanel(state: String, message: String) -> Bool {
        guard Thread.isMainThread else {
            return DispatchQueue.main.sync {
                self.showPanel(state: state, message: message)
            }
        }

        guard runtimeSupported(), let panel = ensurePanel() else {
            return false
        }

        // Clear previous input and force white typingAttributes every open.
        // This is the critical reset: NSTextView can lose typingAttributes
        // between sessions, causing black (invisible) typed text.
        if let tv = textView {
            tv.string = ""
            let font = tv.font ?? NSFont.systemFont(ofSize: 15, weight: .regular)
            tv.typingAttributes = [
                .foregroundColor: NSColor.white,
                .font: font,
            ]
        }

        updateState(state: state, message: message)
        NSApp.activate(ignoringOtherApps: true)
        panel.center()
        panel.makeKeyAndOrderFront(nil)
        panel.orderFrontRegardless()
        textView?.window?.makeFirstResponder(textView)
        return true
    }

    @discardableResult
    func hidePanel() -> Bool {
        guard Thread.isMainThread else {
            return DispatchQueue.main.sync {
                self.hidePanel()
            }
        }

        panel?.orderOut(nil)
        return true
    }

    @discardableResult
    func updateState(state: String, message: String) -> Bool {
        guard Thread.isMainThread else {
            return DispatchQueue.main.sync {
                self.updateState(state: state, message: message)
            }
        }

        let normalized = state.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch normalized {
        case "running":
            statusLabel?.stringValue = message.isEmpty ? "Delegating to Rainy..." : message
            statusLabel?.textColor = NSColor.systemBlue.withAlphaComponent(0.90)
            submitButton?.isEnabled = false
            cancelButton?.isEnabled = true
        case "error":
            statusLabel?.stringValue = message.isEmpty ? "The request could not be submitted." : message
            statusLabel?.textColor = NSColor.systemRed.withAlphaComponent(0.90)
            submitButton?.isEnabled = true
            cancelButton?.isEnabled = true
        default:
            statusLabel?.stringValue = message.isEmpty ? "Ask Rainy to delegate a task into a new chat session." : message
            statusLabel?.textColor = NSColor.white.withAlphaComponent(0.60)
            submitButton?.isEnabled = true
            cancelButton?.isEnabled = true
        }

        return true
    }

    private func ensurePanel() -> NSPanel? {
        if let panel {
            return panel
        }

        let panel = NSPanel(
            contentRect: NSRect(x: 0, y: 0, width: 660, height: 395),
            styleMask: [.titled, .fullSizeContentView, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.titleVisibility = .hidden
        panel.titlebarAppearsTransparent = true
        panel.isFloatingPanel = true
        panel.level = .floating
        panel.hidesOnDeactivate = false
        panel.isReleasedWhenClosed = false
        panel.collectionBehavior = [.moveToActiveSpace, .fullScreenAuxiliary]
        panel.backgroundColor = .clear

        let effectView = NSVisualEffectView(frame: panel.contentView?.bounds ?? .zero)
        effectView.autoresizingMask = [.width, .height]
        effectView.material = .hudWindow
        effectView.blendingMode = .behindWindow
        effectView.state = .active
        effectView.wantsLayer = true
        effectView.layer?.cornerRadius = 18
        effectView.layer?.masksToBounds = true

        let content = NSView()
        content.translatesAutoresizingMaskIntoConstraints = false

        let titleLabel = NSTextField(labelWithString: "Quick Delegate")
        titleLabel.font = NSFont.systemFont(ofSize: 22, weight: .semibold)
        // Force white text so it's always legible on the dark HUD material
        titleLabel.textColor = NSColor.white
        titleLabel.translatesAutoresizingMaskIntoConstraints = false

        let statusLabel = NSTextField(labelWithString: "Ask Rainy to delegate a task into a new chat session.")
        statusLabel.font = NSFont.systemFont(ofSize: 13, weight: .regular)
        // Use a high-alpha white so text is always visible on the dark glass panel
        statusLabel.textColor = NSColor.white.withAlphaComponent(0.60)
        statusLabel.lineBreakMode = .byWordWrapping
        statusLabel.maximumNumberOfLines = 2
        statusLabel.translatesAutoresizingMaskIntoConstraints = false

        // Static Airlock notice — always visible, never changes with state transitions.
        let noticeLabel = NSTextField(labelWithString: "MaTE will notify you for sensitive actions and ask your approval for dangerous ones.")
        noticeLabel.font = NSFont.systemFont(ofSize: 11, weight: .regular)
        noticeLabel.textColor = NSColor.white.withAlphaComponent(0.45)
        noticeLabel.lineBreakMode = .byWordWrapping
        noticeLabel.maximumNumberOfLines = 3
        noticeLabel.translatesAutoresizingMaskIntoConstraints = false

        // Outer container: provides the background tint + border + rounded corners.
        // Background is set here on the CALayer so alpha compositing works correctly
        // over the NSVisualEffectView behind it (drawsBackground on NSScrollView
        // uses an opaque renderer path that ignores alpha, causing the black void).
        let textContainer = NSView()
        textContainer.translatesAutoresizingMaskIntoConstraints = false
        textContainer.wantsLayer = true
        textContainer.layer?.cornerRadius = 14
        textContainer.layer?.borderWidth = 1
        textContainer.layer?.borderColor = NSColor.white.withAlphaComponent(0.14).cgColor
        // Subtle white tint that composites correctly over the blurred background
        textContainer.layer?.backgroundColor = NSColor.white.withAlphaComponent(0.08).cgColor
        textContainer.layer?.masksToBounds = true

        let scrollView = NSScrollView()
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.hasVerticalScroller = true
        scrollView.borderType = .noBorder
        // MUST be false — drawsBackground = true uses an opaque rendering path
        // that ignores alpha, painting a solid dark rectangle over the glass effect.
        scrollView.drawsBackground = false
        scrollView.wantsLayer = true
        scrollView.layer?.backgroundColor = NSColor.clear.cgColor

        let textFont = NSFont.systemFont(ofSize: 15, weight: .regular)

        let textView = NSTextView(frame: NSRect(x: 0, y: 0, width: 612, height: 160))
        textView.isRichText = false
        textView.font = textFont
        textView.drawsBackground = false
        textView.backgroundColor = .clear
        textView.textColor = NSColor.white
        textView.insertionPointColor = NSColor.white
        // CRITICAL: typingAttributes controls the color of text AS IT IS TYPED.
        // Without this, NSTextView falls back to system defaults (black on dark bg = invisible).
        textView.typingAttributes = [
            .foregroundColor: NSColor.white,
            .font: textFont,
        ]
        textView.selectedTextAttributes = [
            .backgroundColor: NSColor.systemBlue.withAlphaComponent(0.40),
            .foregroundColor: NSColor.white,
        ]
        textView.isAutomaticTextCompletionEnabled = false
        textView.isAutomaticQuoteSubstitutionEnabled = false
        textView.isAutomaticDashSubstitutionEnabled = false
        textView.isAutomaticSpellingCorrectionEnabled = false
        textView.string = ""
        textView.isVerticallyResizable = true
        textView.isHorizontallyResizable = false
        textView.autoresizingMask = [.width]
        textView.textContainerInset = NSSize(width: 14, height: 12)
        textView.minSize = NSSize(width: 0, height: 0)
        textView.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        textView.textContainer?.widthTracksTextView = true
        textView.textContainer?.containerSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        scrollView.documentView = textView

        let cancelButton = NSButton(title: "Cancel", target: self, action: #selector(cancelPressed))
        cancelButton.bezelStyle = .rounded
        cancelButton.translatesAutoresizingMaskIntoConstraints = false

        let submitButton = NSButton(title: "Delegate", target: self, action: #selector(submitPressed))
        submitButton.bezelStyle = .rounded
        submitButton.keyEquivalent = "\r"
        submitButton.translatesAutoresizingMaskIntoConstraints = false

        textContainer.addSubview(scrollView)
        NSLayoutConstraint.activate([
            scrollView.leadingAnchor.constraint(equalTo: textContainer.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: textContainer.trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: textContainer.topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: textContainer.bottomAnchor),
        ])

        content.addSubview(titleLabel)
        content.addSubview(statusLabel)
        content.addSubview(noticeLabel)
        content.addSubview(textContainer)
        content.addSubview(cancelButton)
        content.addSubview(submitButton)
        effectView.addSubview(content)
        panel.contentView = effectView

        NSLayoutConstraint.activate([
            content.leadingAnchor.constraint(equalTo: effectView.leadingAnchor, constant: 24),
            content.trailingAnchor.constraint(equalTo: effectView.trailingAnchor, constant: -24),
            content.topAnchor.constraint(equalTo: effectView.topAnchor, constant: 24),
            content.bottomAnchor.constraint(equalTo: effectView.bottomAnchor, constant: -20),

            titleLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            titleLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            titleLabel.topAnchor.constraint(equalTo: content.topAnchor),

            statusLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            statusLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            statusLabel.topAnchor.constraint(equalTo: titleLabel.bottomAnchor, constant: 6),

            noticeLabel.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            noticeLabel.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            noticeLabel.topAnchor.constraint(equalTo: statusLabel.bottomAnchor, constant: 6),

            textContainer.leadingAnchor.constraint(equalTo: content.leadingAnchor),
            textContainer.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            textContainer.topAnchor.constraint(equalTo: noticeLabel.bottomAnchor, constant: 8),
            textContainer.bottomAnchor.constraint(equalTo: submitButton.topAnchor, constant: -16),

            cancelButton.trailingAnchor.constraint(equalTo: submitButton.leadingAnchor, constant: -10),
            cancelButton.bottomAnchor.constraint(equalTo: content.bottomAnchor),

            submitButton.trailingAnchor.constraint(equalTo: content.trailingAnchor),
            submitButton.bottomAnchor.constraint(equalTo: content.bottomAnchor),
        ])

        self.panel = panel
        self.textView = textView
        self.statusLabel = statusLabel
        self.submitButton = submitButton
        self.cancelButton = cancelButton
        return panel
    }

    private func installStatusItemIfNeeded() {
        guard statusItem == nil else {
            return
        }

        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.squareLength)
        guard let button = item.button else {
            return
        }

        button.toolTip = "Quick Delegate"
        button.imagePosition = .imageOnly
        button.imageScaling = .scaleProportionallyDown

        if let whale = loadWhaleImage() {
            whale.size = NSSize(width: 18, height: 18)
            whale.isTemplate = false
            button.image = whale
        } else {
            button.title = "RM"
        }

        item.menu = buildStatusMenu()
        statusItem = item
    }

    private func buildStatusMenu() -> NSMenu {
        if let statusMenu {
            return statusMenu
        }

        let menu = NSMenu()

        let titleItem = NSMenuItem(title: "Rainy MaTE", action: nil, keyEquivalent: "")
        titleItem.isEnabled = false
        menu.addItem(titleItem)

        // Removed duplicate 'Quick Delegate' subtitle item — it mirrored the action item below.

        menu.addItem(.separator())

        let quickDelegateItem = NSMenuItem(
            title: "Quick Delegate",
            action: #selector(statusMenuQuickDelegate),
            keyEquivalent: ""
        )
        quickDelegateItem.target = self
        menu.addItem(quickDelegateItem)

        let openMainItem = NSMenuItem(
            title: "Open Rainy MaTE",
            action: #selector(statusMenuOpenMain),
            keyEquivalent: ""
        )
        openMainItem.target = self
        menu.addItem(openMainItem)

        menu.addItem(.separator())

        let quitItem = NSMenuItem(
            title: "Quit",
            action: #selector(statusMenuQuit),
            keyEquivalent: "q"
        )
        quitItem.target = self
        menu.addItem(quitItem)

        self.statusMenu = menu
        return menu
    }

    private func loadWhaleImage() -> NSImage? {
        let candidates = candidateWhaleURLs()
        for url in candidates {
            if let image = NSImage(contentsOf: url) {
                return image
            }
        }
        return nil
    }

    private func candidateWhaleURLs() -> [URL] {
        let bundle = Bundle.main
        var urls: [URL] = []

        if let bundled = bundle.url(forResource: "whale-dnf", withExtension: "png") {
            urls.append(bundled)
        }

        if let resourceURL = bundle.resourceURL {
            urls.append(resourceURL.appendingPathComponent("whale-dnf.png"))
            urls.append(resourceURL.appendingPathComponent("public/whale-dnf.png"))
            urls.append(resourceURL.appendingPathComponent("dist/whale-dnf.png"))
            urls.append(resourceURL.appendingPathComponent("assets/whale-dnf.png"))
            urls.append(resourceURL.appendingPathComponent("../Resources/whale-dnf.png").standardizedFileURL)
        }

        let cwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath)
        urls.append(cwd.appendingPathComponent("public/whale-dnf.png"))
        urls.append(cwd.appendingPathComponent("dist/whale-dnf.png"))
        urls.append(cwd.appendingPathComponent("whale-dnf.png"))

        return urls
    }

    private func registerHotkeyIfNeeded() {
        guard eventHandlerRef == nil, hotKeyRef == nil else {
            return
        }

        var eventType = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))

        let hotKeyCallback: EventHandlerUPP = { _, eventRef, _ in
            guard let eventRef else { return noErr }
            var hotKeyID = EventHotKeyID()
            let status = GetEventParameter(
                eventRef,
                EventParamName(kEventParamDirectObject),
                EventParamType(typeEventHotKeyID),
                nil,
                MemoryLayout<EventHotKeyID>.size,
                nil,
                &hotKeyID
            )

            if status == noErr, hotKeyID.id == 1 {
                RainyQuickDelegateController.shared.sendCallback(action: "hotkey", payload: nil)
                _ = RainyQuickDelegateController.shared.showPanel(state: "idle", message: "")
            }

            return noErr
        }

        InstallEventHandler(
            GetApplicationEventTarget(),
            hotKeyCallback,
            1,
            &eventType,
            nil,
            &eventHandlerRef
        )

        var hotKeyID = EventHotKeyID(signature: OSType(0x524D5445), id: 1)
        RegisterEventHotKey(
            UInt32(kVK_Space),
            UInt32(cmdKey | shiftKey),
            hotKeyID,
            GetApplicationEventTarget(),
            0,
            &hotKeyRef
        )
    }

    @objc
    private func submitPressed() {
        let text = textView?.string.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        textView?.string = ""
        updateState(state: "running", message: "Delegating to Rainy...")
        sendCallback(action: "submit", payload: text)
    }

    @objc
    private func cancelPressed() {
        textView?.string = ""
        updateState(state: "idle", message: "")
        sendCallback(action: "cancel", payload: nil)
        _ = hidePanel()
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
    private func statusMenuQuickDelegate() {
        updateState(state: "idle", message: "")
        _ = showPanel(state: "idle", message: "")
    }

    @objc
    private func statusMenuOpenMain() {
        sendCallback(action: "open_main", payload: nil)
    }

    @objc
    private func statusMenuQuit() {
        NSApp.terminate(nil)
    }
}

@_cdecl("rainy_quick_delegate_bridge_initialize")
public func rainy_quick_delegate_bridge_initialize(_ callback: RainyQuickDelegateCallback?) {
    DispatchQueue.main.async {
        RainyQuickDelegateController.shared.setCallback(callback)
        RainyQuickDelegateController.shared.initializeBridge()
    }
}

@_cdecl("rainy_quick_delegate_bridge_runtime_supported")
public func rainy_quick_delegate_bridge_runtime_supported() -> Int32 {
    RainyQuickDelegateController.shared.runtimeSupported() ? 1 : 0
}

@_cdecl("rainy_quick_delegate_bridge_show")
public func rainy_quick_delegate_bridge_show(
    _ state: UnsafePointer<CChar>?,
    _ message: UnsafePointer<CChar>?
) -> Int32 {
    let stateValue = state.map { String(cString: $0) } ?? "idle"
    let messageValue = message.map { String(cString: $0) } ?? ""
    return RainyQuickDelegateController.shared.showPanel(state: stateValue, message: messageValue) ? 1 : 0
}

@_cdecl("rainy_quick_delegate_bridge_hide")
public func rainy_quick_delegate_bridge_hide() -> Int32 {
    RainyQuickDelegateController.shared.hidePanel() ? 1 : 0
}

@_cdecl("rainy_quick_delegate_bridge_set_state")
public func rainy_quick_delegate_bridge_set_state(
    _ state: UnsafePointer<CChar>?,
    _ message: UnsafePointer<CChar>?
) -> Int32 {
    let stateValue = state.map { String(cString: $0) } ?? "idle"
    let messageValue = message.map { String(cString: $0) } ?? ""
    return RainyQuickDelegateController.shared.updateState(state: stateValue, message: messageValue) ? 1 : 0
}
