import AppKit
import FinderSync

@objc(MToolFinderSync)
final class FinderSync: FIFinderSync {
  override init() {
    super.init()
    FIFinderSyncController.default().directoryURLs = [URL(fileURLWithPath: "/")]
  }

  override func menu(for menuKind: FIMenuKind) -> NSMenu? {
    let controller = FIFinderSyncController.default()
    guard let target = controller.targetedURL(), target.isFileURL else { return nil }
    let menu = NSMenu()
    switch menuKind {
    case .contextualMenuForContainer:
      addItem(
        to: menu, title: "新增文件", action: #selector(newFileInContainer(_:)), icon: "doc.badge.plus")
      addItem(
        to: menu, title: "使用 VS Code 打开", action: #selector(openContainerInVSCode(_:)),
        icon: "chevron.left.forwardslash.chevron.right")
    case .contextualMenuForItems:
      let selected = (controller.selectedItemURLs() ?? [target]).filter(\.isFileURL)
      if selected.count == 1, selected[0].hasDirectoryPath {
        addItem(
          to: menu, title: "新增文件", action: #selector(newFileInSelection(_:)),
          icon: "doc.badge.plus")
      }
      if !selected.isEmpty, selected.count <= 32 {
        addItem(
          to: menu, title: "使用 VS Code 打开", action: #selector(openSelectionInVSCode(_:)),
          icon: "chevron.left.forwardslash.chevron.right")
      }
    default:
      return nil
    }
    return menu.items.isEmpty ? nil : menu
  }

  private func addItem(
    to menu: NSMenu, title: String, action: Selector, icon: String
  ) {
    let item = NSMenuItem(title: title, action: action, keyEquivalent: "")
    item.target = self
    item.image = NSImage(systemSymbolName: icon, accessibilityDescription: nil)
    menu.addItem(item)
  }

  @objc private func newFileInContainer(_ sender: NSMenuItem) {
    dispatch(action: "new-file", selection: false)
  }

  @objc private func newFileInSelection(_ sender: NSMenuItem) {
    dispatch(action: "new-file", selection: true)
  }

  @objc private func openContainerInVSCode(_ sender: NSMenuItem) {
    dispatch(action: "vscode", selection: false)
  }

  @objc private func openSelectionInVSCode(_ sender: NSMenuItem) {
    dispatch(action: "vscode", selection: true)
  }

  private func dispatch(action: String, selection: Bool) {
    let controller = FIFinderSyncController.default()
    let urls: [URL]
    if selection {
      let selected = controller.selectedItemURLs() ?? []
      urls = selected.isEmpty ? controller.targetedURL().map { [$0] } ?? [] : selected
    } else {
      urls = controller.targetedURL().map { [$0] } ?? []
    }
    guard !urls.isEmpty, urls.count <= 32, urls.allSatisfy(\.isFileURL) else {
      NSLog("MTool: Finder action has no valid file context")
      return
    }
    if action == "new-file", urls.count != 1 {
      NSLog("MTool: new file action requires one directory")
      return
    }
    var components = URLComponents()
    components.scheme = "mtool"
    components.host = "context"
    components.path = "/\(action)"
    components.queryItems = urls.map { URLQueryItem(name: "path", value: $0.path) }
    components.percentEncodedQuery = components.percentEncodedQuery?.replacingOccurrences(
      of: "+", with: "%2B")
    guard let url = components.url, url.absoluteString.utf8.count <= 64 * 1024 else {
      NSLog("MTool: Finder action URL exceeds the allowed size")
      return
    }
    let configuration = NSWorkspace.OpenConfiguration()
    configuration.activates = false
    configuration.addsToRecentItems = false
    configuration.environment = ["MTOOL_CONTEXT_LAUNCH": "1"]
    NSWorkspace.shared.open(url, configuration: configuration) { _, error in
      if let error {
        NSLog(
          "MTool: could not open context action (%@ %ld)", (error as NSError).domain,
          (error as NSError).code)
      }
    }
  }
}
