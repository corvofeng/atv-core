import Foundation

/// The app speaks Chinese when the Mac's preferred language is Chinese, and English otherwise.
enum L10n {
    static let isChinese = Locale.preferredLanguages.first?.lowercased().hasPrefix("zh") ?? false

    /// The Chinese or the English version of a piece of interface text.
    static func t(_ zh: String, _ en: String) -> String {
        isChinese ? zh : en
    }
}
