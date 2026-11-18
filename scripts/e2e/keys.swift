import Carbon
import CoreGraphics
import Foundation

let src = CGEventSource(stateID: .hidSystemState)

func fail(_ message: String) -> Never {
    FileHandle.standardError.write("keys: \(message)\n".data(using: .utf8)!)
    exit(2)
}

func key(_ code: CGKeyCode, mask: CGEventFlags) {
    for down in [true, false] {
        guard let event = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down) else {
            fail("cannot create a key event")
        }
        event.flags = mask
        event.post(tap: .cghidEventTap)
    }
}

func modifier(_ raw: UInt64, device: UInt64) -> CGEventFlags {
    CGEventFlags(rawValue: raw | device)
}

let modKeys: [String: (CGKeyCode, CGEventFlags)] = [
    "ropt": (61, modifier(CGEventFlags.maskAlternate.rawValue, device: 0x40)),
    "lopt": (58, modifier(CGEventFlags.maskAlternate.rawValue, device: 0x20)),
    "rcmd": (54, modifier(CGEventFlags.maskCommand.rawValue, device: 0x10)),
    "rctl": (62, modifier(CGEventFlags.maskControl.rawValue, device: 0x2000)),
    "ctl": (59, modifier(CGEventFlags.maskControl.rawValue, device: 0x1)),
    "shift": (56, modifier(CGEventFlags.maskShift.rawValue, device: 0x2)),
]

func flagsChanged(_ code: CGKeyCode, down: Bool, flags: CGEventFlags) {
    guard let event = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: down) else {
        fail("cannot create a flagsChanged event")
    }
    event.type = .flagsChanged
    event.flags = flags
    event.post(tap: .cghidEventTap)
    usleep(20_000)
}

func modifierNamed(_ name: String) -> (CGKeyCode, CGEventFlags) {
    guard let entry = modKeys[name] else { fail("unknown modifier \(name)") }
    return entry
}

func currentSource() -> String {
    let source = TISCopyCurrentKeyboardInputSource().takeRetainedValue()
    guard let raw = TISGetInputSourceProperty(source, kTISPropertyInputSourceID) else {
        fail("cannot read the input source")
    }
    return Unmanaged<CFString>.fromOpaque(raw).takeUnretainedValue() as String
}

func selectSource(_ id: String) {
    let filter = [kTISPropertyInputSourceID as String: id] as CFDictionary
    guard let list = TISCreateInputSourceList(filter, false)?.takeRetainedValue() as? [TISInputSource],
        let target = list.first
    else { fail("no input source \(id)") }
    TISSelectInputSource(target)
}

var held: CGEventFlags = []
for arg in CommandLine.arguments.dropFirst() {
    let parts = arg.split(separator: ":", maxSplits: 1).map(String.init)
    let value = parts.count > 1 ? parts[1] : ""
    switch parts[0] {
    case "down":
        let (code, mask) = modifierNamed(value)
        held.insert(mask)
        flagsChanged(code, down: true, flags: held)
    case "up":
        let (code, mask) = modifierNamed(value)
        held.remove(mask)
        flagsChanged(code, down: false, flags: held)
    case "releaseall":
        for (code, _) in modKeys.values { flagsChanged(code, down: false, flags: []) }
        held = []
    case "sleep":
        guard let seconds = Double(value) else { fail("bad sleep \(arg)") }
        usleep(UInt32(seconds * 1_000_000))
    case "esc": key(53, mask: held)
    case "v": key(9, mask: held)
    case "e": key(14, mask: held)
    case "key":
        guard let code = Int(value) else { fail("bad key \(arg)") }
        key(CGKeyCode(code), mask: held)
    case "source-get": print(currentSource())
    case "source-set": selectSource(value)
    default: fail("unknown command \(arg)")
    }
}
