import Foundation

// Mock rule models matching the Rust backend (np-mock crate)

public struct MockRulesResponse: Codable, Sendable {
    public let rules: [MockRule]
}

public struct MockRule: Codable, Identifiable, Hashable, Sendable {
    public var id: String
    public var priority: Int
    public var enabled: Bool
    public var description: String?
    public var matchSpec: MockMatchSpec
    public var action: MockAction

    public init(id: String, priority: Int, enabled: Bool, description: String?, matchSpec: MockMatchSpec, action: MockAction) {
        self.id = id
        self.priority = priority
        self.enabled = enabled
        self.description = description
        self.matchSpec = matchSpec
        self.action = action
    }
}

public struct MockMatchSpec: Codable, Hashable, Sendable {
    public var method: String?
    public var host: String?
    public var path: String?
    public var pathRegex: String?
    public var query: [String: String]?
    public var headers: [String: String]?

    public init(method: String? = nil, host: String? = nil, path: String? = nil, pathRegex: String? = nil, query: [String: String]? = nil, headers: [String: String]? = nil) {
        self.method = method
        self.host = host
        self.path = path
        self.pathRegex = pathRegex
        self.query = query
        self.headers = headers
    }
}

public enum MockAction: Codable, Hashable, Sendable {
    case respond(MockRespondAction)
    case pass
    case fail(String)

    public var type: String {
        switch self {
        case .respond: return "respond"
        case .pass: return "pass"
        case .fail: return "fail"
        }
    }

    enum CodingKeys: String, CodingKey {
        case type
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        let type = try container.decode(String.self, forKey: .type)

        switch type {
        case "respond":
            self = .respond(try MockRespondAction(from: decoder))
        case "pass":
            self = .pass
        case "fail":
            let failContainer = try decoder.singleValueContainer()
            let error = try failContainer.decode(String.self)
            self = .fail(error)
        default:
            throw DecodingError.dataCorruptedError(forKey: .type, in: container, debugDescription: "Unknown action type: \(type)")
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        switch self {
        case .respond(let action):
            try container.encode("respond", forKey: .type)
            try action.encode(to: encoder)
        case .pass:
            try container.encode("pass", forKey: .type)
        case .fail(let error):
            try container.encode("fail", forKey: .type)
            var singleContainer = encoder.singleValueContainer()
            try singleContainer.encode(error)
        }
    }
}

public struct MockRespondAction: Codable, Hashable, Sendable {
    public var status: Int
    public var headers: [String: String]
    public var body: MockBodySource
    public var delayMs: Int?

    public init(status: Int, headers: [String: String], body: MockBodySource, delayMs: Int?) {
        self.status = status
        self.headers = headers
        self.body = body
        self.delayMs = delayMs
    }
}

public enum MockBodySource: Codable, Hashable, Sendable {
    case file(String)
    case inline(String)

    enum CodingKeys: String, CodingKey {
        case file
        case inline
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)

        if let file = try? container.decode(String.self, forKey: .file) {
            self = .file(file)
        } else if let inline = try? container.decode(String.self, forKey: .inline) {
            self = .inline(inline)
        } else {
            throw DecodingError.dataCorrupted(.init(codingPath: decoder.codingPath, debugDescription: "Invalid body source"))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)

        switch self {
        case .file(let path):
            try container.encode(path, forKey: .file)
        case .inline(let content):
            try container.encode(content, forKey: .inline)
        }
    }
}
