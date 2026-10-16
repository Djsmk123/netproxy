import Foundation
import SwiftUI

struct JSONNode: Identifiable {
    let id: String          // Path like "root.user.login"
    let key: String
    let value: String?      // Leaf nodes only
    let type: JSONType
    let children: [JSONNode]?
    let arrayCount: Int?

    var hasChildren: Bool { children != nil && !children!.isEmpty }
    var isArray: Bool { type == .array }
    var isObject: Bool { type == .object }

    var typeLabel: String {
        switch type {
        case .object:
            if let count = children?.count {
                return "object (\(count))"
            }
            return "object"
        case .array:
            if let count = arrayCount {
                return "array [\(count)]"
            }
            return "array"
        default:
            return type.rawValue
        }
    }

    var color: Color {
        switch type {
        case .string: return DesignSystem.Colors.jsonString
        case .number: return DesignSystem.Colors.jsonNumber
        case .boolean: return DesignSystem.Colors.jsonBoolean
        case .null: return DesignSystem.Colors.jsonNull
        default: return .primary
        }
    }
}

enum JSONType: String {
    case object, array, string, number, boolean, null
}
