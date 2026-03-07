use crate::antipattern::types::{AntiPattern, AntiPatternCategory, Confidence, WarningSeverity};

struct PatternDef {
    id: &'static str,
    name: &'static str,
    category: AntiPatternCategory,
    severity: WarningSeverity,
    confidence: Confidence,
    regex: &'static str,
    title: &'static str,
    explanation: &'static str,
    suggestion: &'static str,
    nudge: Option<&'static str>,
    file_extensions: Option<&'static [&'static str]>,
    all_file_types: bool,
    allowlist: &'static [&'static str],
    enabled: bool,
    opt_in: bool,
}

const AP003_ALLOWLIST: &[&str] = &[
    "*.d.ts",
    "**/__mocks__/**",
    "**/test/**/*.ts",
    "**/__tests__/**",
];
const AP004_ALLOWLIST: &[&str] = &["**/*.test.ts", "**/*.spec.ts", "**/__tests__/**"];
const AP007_ALLOWLIST: &[&str] = &["**/*.test.ts", "**/*.spec.ts", "**/scripts/**", "**/cli/**"];
const EMAIL_ALLOWLIST: &[&str] = &["**/email/**"];
const AP012_ALLOWLIST: &[&str] = &["**/reset.css", "**/normalize.css"];
const NO_ALLOWLIST: &[&str] = &[];

const HTML_EXTENSIONS: &[&str] = &[".html", ".htm"];
const CSS_EXTENSIONS: &[&str] = &[".css", ".scss", ".less"];

const PATTERN_DEFS: &[PatternDef] = &[
    PatternDef {
        id: "AP-001",
        name: "Broad eslint-disable",
        category: AntiPatternCategory::EscapeHatch,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r"/\*\s*eslint-disable\s*\*/|//\s*eslint-disable(?!-next-line|-line)\s*$",
        title: "Broad eslint-disable added",
        explanation: "Disabling all ESLint rules hides legitimate issues and makes code harder to maintain. This pattern indicates technical debt that should be addressed.",
        suggestion: "Disable specific rules with /* eslint-disable rule-name */ or fix the underlying issues.",
        nudge: Some(
            "Don't disable all linting rules. Identify which specific rule is failing and either fix the underlying issue or disable only that one rule with `/* eslint-disable specific-rule */`. Blanket disables hide real problems.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: NO_ALLOWLIST,
        enabled: true,
        opt_in: false,
    },
    PatternDef {
        id: "AP-002",
        name: "Rule-specific eslint-disable",
        category: AntiPatternCategory::EscapeHatch,
        severity: WarningSeverity::Info,
        confidence: Confidence::High,
        regex: r"eslint-disable(?:-next-line|-line)?\s+[\w@/-]+",
        title: "Rule-specific eslint-disable",
        explanation: "While better than disabling all rules, targeted disables still indicate code that violates linting standards. Consider if the disable is necessary or if the code can be improved.",
        suggestion: "Add a comment explaining why this rule needs to be disabled here.",
        nudge: Some(
            "Before disabling this rule, try to fix the code so it passes. If the disable is genuinely necessary, add a comment explaining why this specific case can't follow the rule.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: NO_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-003",
        name: "Explicit any type",
        category: AntiPatternCategory::TypeSafety,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r":\s*any\b|as\s+any\b|<any>",
        title: "Explicit any type usage",
        explanation: "Using `any` defeats the purpose of TypeScript by disabling type checking. This can hide bugs and makes refactoring harder.",
        suggestion: "Use `unknown` for truly unknown types, or define a proper interface/type. For third-party libraries, consider using or creating type definitions.",
        nudge: Some(
            "Don't use `any` here. Think about what type this value actually holds and declare it explicitly. If it comes from an API, define an interface for the response shape. If the type is truly unknown, use `unknown` and narrow it with type guards before use.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: AP003_ALLOWLIST,
        enabled: true,
        opt_in: false,
    },
    PatternDef {
        id: "AP-004",
        name: "@ts-ignore directive",
        category: AntiPatternCategory::TypeSafety,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r"@ts-ignore",
        title: "@ts-ignore suppresses all errors",
        explanation: "@ts-ignore suppresses ALL TypeScript errors on the next line, including legitimate issues. This can hide bugs introduced by code changes.",
        suggestion: "Use @ts-expect-error with a description instead, which fails if the expected error disappears. Better yet, fix the underlying type issue.",
        nudge: Some(
            "Don't suppress this TypeScript error - fix it. If you must suppress, use `@ts-expect-error` instead so it fails when the underlying issue is resolved. But first, read the actual error message and address the type mismatch directly.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: AP004_ALLOWLIST,
        enabled: true,
        opt_in: false,
    },
    PatternDef {
        id: "AP-005",
        name: "@ts-expect-error directive",
        category: AntiPatternCategory::TypeSafety,
        severity: WarningSeverity::Info,
        confidence: Confidence::High,
        regex: r"@ts-expect-error",
        title: "@ts-expect-error used",
        explanation: "@ts-expect-error is safer than @ts-ignore as it fails when the error disappears. However, it still indicates intentional type system workarounds.",
        suggestion: "Consider if the underlying type issue can be fixed. If not, ensure the @ts-expect-error comment explains why.",
        nudge: Some(
            "This type error is being suppressed rather than fixed. Read the error message and resolve the type mismatch. If it is a genuine limitation of the type system, keep the `@ts-expect-error` but ensure the comment explains exactly why.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: AP004_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-006",
        name: "Empty catch block",
        category: AntiPatternCategory::ErrorHandling,
        severity: WarningSeverity::Warning,
        confidence: Confidence::Medium,
        regex: r"catch\s*\([^)]*\)\s*\{\s*(?://[^\n]*\s*)?\}",
        title: "Empty catch block swallows errors",
        explanation: "Empty catch blocks silently swallow errors, making debugging difficult. Errors should be logged, re-thrown, or explicitly handled.",
        suggestion: "At minimum, log the error for debugging. Consider if the error should be re-thrown or if specific recovery logic is needed.",
        nudge: Some(
            "Don't swallow this error silently. At minimum, log it so failures are visible. Better: decide whether this error is recoverable (handle it) or not (re-throw it). Silent catch blocks make debugging impossible.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: NO_ALLOWLIST,
        enabled: true,
        opt_in: false,
    },
    PatternDef {
        id: "AP-007",
        name: "Console in production code",
        category: AntiPatternCategory::CodeQuality,
        severity: WarningSeverity::Info,
        confidence: Confidence::Medium,
        regex: r"console\.(log|warn|info|debug)\s*\(",
        title: "Console statement in production code",
        explanation: "Console statements should not appear in production code. They can leak sensitive information, clutter the console, and indicate incomplete debugging.",
        suggestion: "Use a proper logging library with log levels, or remove the console statement. console.error is acceptable for actual error conditions.",
        nudge: Some(
            "Remove this console statement or replace it with a proper logger that supports log levels. Console output in production leaks information and clutters output. If this is intentional debugging, wrap it in a development-only check.",
        ),
        file_extensions: None,
        all_file_types: false,
        allowlist: AP007_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-008",
        name: "Inline style attribute",
        category: AntiPatternCategory::Html,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r#"style\s*=\s*["']"#,
        title: "Inline style attribute found",
        explanation: "Inline styles mix presentation with structure, making CSS harder to maintain, override, and cache. They also increase HTML file size.",
        suggestion: "Move styles to an external CSS file or use CSS classes. For dynamic styles, use CSS custom properties or a CSS-in-JS solution.",
        nudge: Some(
            "Move this inline style to a CSS class. Inline styles can't be overridden by stylesheets, break consistency, and make maintenance harder. Define a class in your stylesheet and apply it instead.",
        ),
        file_extensions: Some(HTML_EXTENSIONS),
        all_file_types: false,
        allowlist: EMAIL_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-009",
        name: "Inline script block",
        category: AntiPatternCategory::Html,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r"<script(?:\s[^>]*)?>(?!\s*<\/script>)",
        title: "Inline script block found",
        explanation: "Inline scripts bypass Content Security Policy (CSP), prevent browser caching, and make code harder to test and maintain.",
        suggestion: "Move JavaScript to external .js files referenced with <script src=\"...\">. This enables caching, CSP compliance, and better separation of concerns.",
        nudge: Some(
            "Move this script to an external `.js` file and reference it with `<script src=\"...\">`. Inline scripts cannot be cached, violate CSP policies, and make code harder to test.",
        ),
        file_extensions: Some(HTML_EXTENSIONS),
        all_file_types: false,
        allowlist: EMAIL_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-010",
        name: "Inline event handler",
        category: AntiPatternCategory::Html,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r#"\bon\w+\s*=\s*["']"#,
        title: "Inline event handler found",
        explanation: "Inline event handlers (onclick, onload, etc.) mix behaviour with HTML structure, bypass CSP, and make code harder to debug and maintain.",
        suggestion: "Use addEventListener() in external JavaScript files instead. For frameworks, use the framework event binding syntax.",
        nudge: Some(
            "Remove this inline event handler and use `addEventListener()` in an external script instead. Inline handlers mix behaviour with markup and are blocked by strict Content Security Policies.",
        ),
        file_extensions: Some(HTML_EXTENSIONS),
        all_file_types: false,
        allowlist: EMAIL_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-011",
        name: "Deprecated HTML tag",
        category: AntiPatternCategory::Html,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r"<(?:font|center|marquee|blink|big|strike)\b",
        title: "Deprecated HTML tag used",
        explanation: "Deprecated HTML tags like <font>, <center>, and <marquee> are obsolete. They may not render correctly in modern browsers and indicate outdated practices.",
        suggestion: "Replace deprecated tags with semantic HTML and CSS. For example, use CSS text-align instead of <center>, and CSS font properties instead of <font>.",
        nudge: Some(
            "Replace this deprecated HTML tag with its modern CSS equivalent. Use CSS for visual presentation instead of presentational HTML elements.",
        ),
        file_extensions: Some(HTML_EXTENSIONS),
        all_file_types: false,
        allowlist: EMAIL_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-012",
        name: "!important in CSS",
        category: AntiPatternCategory::Css,
        severity: WarningSeverity::Warning,
        confidence: Confidence::High,
        regex: r"!\s*important",
        title: "!important used in CSS",
        explanation: "Using !important overrides all other specificity rules, creating maintenance headaches. It often indicates specificity wars or architectural issues in CSS.",
        suggestion: "Increase selector specificity naturally, restructure CSS to avoid conflicts, or use CSS layers (@layer) for better cascade control.",
        nudge: Some(
            "Don't use `!important` - it breaks the cascade and makes styles nearly impossible to override. Instead, increase the specificity of your selector or restructure your CSS to avoid the conflict.",
        ),
        file_extensions: Some(CSS_EXTENSIONS),
        all_file_types: false,
        allowlist: AP012_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
    PatternDef {
        id: "AP-013",
        name: "CSS @import",
        category: AntiPatternCategory::Css,
        severity: WarningSeverity::Info,
        confidence: Confidence::High,
        regex: r#"@import\s+(?:url\()?["']"#,
        title: "CSS @import causes sequential loading",
        explanation: "CSS @import causes browsers to load stylesheets sequentially rather than in parallel, which increases page load time. Each @import blocks rendering until the imported file loads.",
        suggestion: "Use <link> tags in HTML for parallel loading, or use a CSS bundler (PostCSS, Sass, etc.) to inline imports at build time.",
        nudge: Some(
            "Replace this CSS `@import` with a `<link>` tag in your HTML. `@import` blocks parallel downloads and slows page load. Each `@import` creates a sequential request.",
        ),
        file_extensions: Some(CSS_EXTENSIONS),
        all_file_types: false,
        allowlist: NO_ALLOWLIST,
        enabled: true,
        opt_in: true,
    },
];

fn to_antipattern(def: &PatternDef) -> AntiPattern {
    AntiPattern {
        id: def.id.to_string(),
        name: def.name.to_string(),
        category: def.category,
        severity: def.severity,
        confidence: def.confidence,
        regex: def.regex.to_string(),
        title: def.title.to_string(),
        explanation: def.explanation.to_string(),
        suggestion: def.suggestion.to_string(),
        nudge: def.nudge.map(ToString::to_string),
        file_extensions: def
            .file_extensions
            .map(|extensions| extensions.iter().map(ToString::to_string).collect()),
        all_file_types: def.all_file_types,
        allowlist: def.allowlist.iter().map(ToString::to_string).collect(),
        threshold: None,
        enabled: def.enabled,
        opt_in: def.opt_in,
    }
}

#[must_use]
pub fn all_patterns() -> Vec<AntiPattern> {
    PATTERN_DEFS.iter().map(to_antipattern).collect()
}

#[must_use]
pub fn get_pattern(id: &str) -> Option<AntiPattern> {
    PATTERN_DEFS
        .iter()
        .find(|pattern| pattern.id == id)
        .map(to_antipattern)
}

#[must_use]
pub fn get_enabled_patterns() -> Vec<AntiPattern> {
    PATTERN_DEFS
        .iter()
        .filter(|pattern| pattern.enabled)
        .map(to_antipattern)
        .collect()
}

#[must_use]
pub fn get_default_patterns() -> Vec<AntiPattern> {
    PATTERN_DEFS
        .iter()
        .filter(|pattern| pattern.enabled && !pattern.opt_in)
        .map(to_antipattern)
        .collect()
}

#[must_use]
pub fn get_pattern_ids() -> Vec<String> {
    PATTERN_DEFS
        .iter()
        .map(|pattern| pattern.id.to_string())
        .collect()
}

#[must_use]
pub fn is_valid_pattern_id(id: &str) -> bool {
    PATTERN_DEFS.iter().any(|pattern| pattern.id == id)
}

pub const PATTERNS: usize = 13;

#[cfg(test)]
mod tests {
    use crate::antipattern::patterns::{
        PATTERNS, all_patterns, get_default_patterns, get_enabled_patterns, get_pattern,
        get_pattern_ids, is_valid_pattern_id,
    };

    #[test]
    fn exposes_all_thirteen_patterns() {
        let patterns = all_patterns();
        assert_eq!(patterns.len(), 13);
        assert_eq!(patterns.len(), PATTERNS);
        assert_eq!(get_pattern_ids().len(), 13);
    }

    #[test]
    fn filters_default_and_opt_in_patterns() {
        let default_patterns = get_default_patterns();
        let enabled_patterns = get_enabled_patterns();

        assert_eq!(default_patterns.len(), 4);
        assert_eq!(enabled_patterns.len(), 13);
        assert!(default_patterns.iter().all(|pattern| !pattern.opt_in));
    }

    #[test]
    fn returns_expected_exact_regex_for_lookahead_patterns() {
        let ap001 = get_pattern("AP-001");
        let ap009 = get_pattern("AP-009");

        if let Some(pattern) = ap001 {
            assert_eq!(
                pattern.regex,
                r"/\*\s*eslint-disable\s*\*/|//\s*eslint-disable(?!-next-line|-line)\s*$"
            );
        } else {
            panic!("AP-001 pattern missing");
        }
        if let Some(pattern) = ap009 {
            assert_eq!(pattern.regex, r"<script(?:\s[^>]*)?>(?!\s*<\/script>)");
        } else {
            panic!("AP-009 pattern missing");
        }
    }

    #[test]
    fn keeps_required_allowlists() {
        let ap003 = get_pattern("AP-003");
        let ap012 = get_pattern("AP-012");

        if let Some(pattern) = ap003 {
            assert!(pattern.allowlist.iter().any(|item| item == "*.d.ts"));
            assert!(
                pattern
                    .allowlist
                    .iter()
                    .any(|item| item == "**/__tests__/**")
            );
        } else {
            panic!("AP-003 pattern missing");
        }
        if let Some(pattern) = ap012 {
            assert_eq!(pattern.allowlist, vec!["**/reset.css", "**/normalize.css"]);
        } else {
            panic!("AP-012 pattern missing");
        }
    }

    #[test]
    fn validates_pattern_ids() {
        assert!(is_valid_pattern_id("AP-013"));
        assert!(!is_valid_pattern_id("AP-999"));
    }
}
