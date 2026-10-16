import Foundation

struct JSONParser {
    static func parse(_ jsonString: String) -> Result<JSONNode, JSONParseError> {
        guard let data = jsonString.data(using: .utf8) else {
            return .failure(.invalidUTF8)
        }

        do {
            let json = try JSONSerialization.jsonObject(with: data)
            let root = buildNode(key: "root", value: json, path: "root")
            return .success(root)
        } catch let error as NSError {
            // Extract line and column from error
            let line = extractLine(from: error)
            let column = extractColumn(from: error)
            return .failure(.parseError(line: line, column: column, message: error.localizedDescription))
        }
    }

    private static func buildNode(key: String, value: Any, path: String) -> JSONNode {
        if let dict = value as? [String: Any] {
            let children = dict.map { k, v in
                buildNode(key: k, value: v, path: "\(path).\(k)")
            }.sorted { $0.key < $1.key }
            return JSONNode(id: path, key: key, value: nil, type: .object, children: children, arrayCount: nil)
        }

        if let array = value as? [Any] {
            let children = array.enumerated().map { index, v in
                buildNode(key: "[\(index)]", value: v, path: "\(path)[\(index)]")
            }
            return JSONNode(id: path, key: key, value: nil, type: .array, children: children, arrayCount: array.count)
        }

        if let string = value as? String {
            return JSONNode(id: path, key: key, value: "\"\(string)\"", type: .string, children: nil, arrayCount: nil)
        }

        if let number = value as? NSNumber {
            if number.isBool {
                return JSONNode(id: path, key: key, value: number.boolValue ? "true" : "false", type: .boolean, children: nil, arrayCount: nil)
            }
            return JSONNode(id: path, key: key, value: "\(number)", type: .number, children: nil, arrayCount: nil)
        }

        if value is NSNull {
            return JSONNode(id: path, key: key, value: "null", type: .null, children: nil, arrayCount: nil)
        }

        return JSONNode(id: path, key: key, value: "\(value)", type: .string, children: nil, arrayCount: nil)
    }

    private static func extractLine(from error: NSError) -> Int {
        // NSJSONSerialization doesn't provide line numbers, return 1
        return 1
    }

    private static func extractColumn(from error: NSError) -> Int {
        return 1
    }
}

extension NSNumber {
    var isBool: Bool {
        String(cString: objCType) == "c"
    }
}

struct JSONParseError: Error, LocalizedError {
    let line: Int?
    let column: Int?
    let message: String

    static let invalidUTF8 = JSONParseError(line: nil, column: nil, message: "Invalid UTF-8 encoding")

    static func parseError(line: Int, column: Int, message: String) -> JSONParseError {
        JSONParseError(line: line, column: column, message: message)
    }

    var errorDescription: String? {
        if let line = line, let column = column {
            return "Error at line \(line), column \(column): \(message)"
        }
        return message
    }
}
