//! This crate offers Rust bindings to [KaTeX](https://katex.org).
//! This allows you to render LaTeX equations to HTML.
//!
//! # Usage
//!
//! Add this to your `Cargo.toml`:
//! ```toml
//! [dependencies]
//! katex = "0.1"
//! ```
//!
//! # Examples
//!
//! ```
//! let html = katex::render("E = mc^2").unwrap();
//!
//! let opts = katex::Opts::builder().display_mode(true).build().unwrap();
//! let html_in_display_mode = katex::render_with_opts("E = mc^2", opts).unwrap();
//! ```

#[macro_use]
extern crate derive_builder;

use quick_js::{self, Context as JsContext, JsValue};
use std::collections::HashMap;

const KATEX_SRC: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/vendor/katex.min.js"));

thread_local! {
    static KATEX: Result<JsContext> = init_katex();
}

/// Error type for this crate.
#[non_exhaustive]
#[derive(thiserror::Error, Clone, Debug)]
pub enum Error {
    #[error("failed to initialize js environment (detail: {0})")]
    JsInitError(String),
    #[error("failed to execute js (detail: {0})")]
    JsExecError(String),
    #[error("js returns invalid result")]
    InvalidResult,
}

impl From<quick_js::ContextError> for Error {
    fn from(e: quick_js::ContextError) -> Self {
        Self::JsInitError(format!("{}", e))
    }
}

impl From<quick_js::ExecutionError> for Error {
    fn from(e: quick_js::ExecutionError) -> Self {
        Self::JsExecError(format!("{}", e))
    }
}

/// Alias to `core::result::Result<T, katex::Error>`
pub type Result<T> = core::result::Result<T, Error>;

/// Initialize KaTeX js environment.
fn init_katex() -> Result<JsContext> {
    let ctx = JsContext::new()?;
    let _ = ctx.eval(KATEX_SRC)?;
    let _ = ctx.eval("renderToString = katex.renderToString;")?;
    Ok(ctx)
}

/// Options to be passed to KaTeX.
///
/// Read <https://katex.org/docs/options.html> for more information.
#[non_exhaustive]
#[derive(Clone, Builder, Debug, Default)]
#[builder(default)]
#[builder(setter(into, strip_option))]
pub struct Opts {
    /// Whether to render the math in the display mode.
    pub display_mode: Option<bool>,
    /// KaTeX output type.
    pub output_type: Option<OutputType>,
    /// Whether to have `\tags` rendered on the left instead of the right.
    pub leqno: Option<bool>,
    /// Whether to make display math flush left.
    pub fleqn: Option<bool>,
    /// Whether to let KaTeX throw a ParseError for invalid LaTeX.
    pub throw_on_error: Option<bool>,
    /// Color used for invalid LaTeX.
    pub error_color: Option<String>,
    /// Collection of custom macros.
    pub macros: HashMap<String, String>,
    /// Specifies a minimum thickness, in ems.
    /// Read <https://katex.org/docs/options.html> for more information.
    pub min_rule_thickness: Option<f64>,
}

impl Opts {
    /// Return [`OptsBuilder`].
    pub fn builder() -> OptsBuilder {
        OptsBuilder::default()
    }
}

impl Into<JsValue> for Opts {
    fn into(self) -> JsValue {
        let mut opt: HashMap<String, JsValue> = HashMap::new();
        if let Some(display_mode) = self.display_mode {
            opt.insert("displayMode".to_owned(), display_mode.into());
        }
        if let Some(output_type) = self.output_type {
            opt.insert(
                "output".to_owned(),
                match output_type {
                    OutputType::Html => "html",
                    OutputType::Mathml => "mathml",
                    OutputType::HtmlAndMathml => "htmlAndMathml",
                }
                .into(),
            );
        }
        if let Some(leqno) = self.leqno {
            opt.insert("leqno".to_owned(), leqno.into());
        }
        if let Some(fleqn) = self.fleqn {
            opt.insert("fleqn".to_owned(), fleqn.into());
        }
        if let Some(throw_on_error) = self.throw_on_error {
            opt.insert("throwOnError".to_owned(), throw_on_error.into());
        }
        if let Some(error_color) = self.error_color {
            opt.insert("errorColor".to_owned(), error_color.into());
        }
        opt.insert("macros".to_owned(), self.macros.into());
        if let Some(min_rule_thickness) = self.min_rule_thickness {
            opt.insert("minRuleThickness".to_owned(), min_rule_thickness.into());
        }
        JsValue::Object(opt)
    }
}

impl OptsBuilder {
    /// Add an entry to [`Opts::macros`].
    pub fn add_macro(mut self, entry_name: String, entry_data: String) -> Self {
        match self.macros.as_mut() {
            Some(macros) => {
                macros.insert(entry_name, entry_data);
            }
            None => {
                let mut macros = HashMap::new();
                macros.insert(entry_name, entry_data);
                self.macros = Some(macros);
            }
        }
        self
    }
}

/// Output type from KaTeX.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum OutputType {
    /// Outputs KaTeX in HTML only.
    Html,
    /// Outputs KaTeX in MathML only.
    Mathml,
    /// Outputs HTML for visual rendering and includes MathML for accessibility.
    HtmlAndMathml,
}

/// Render LaTeX equation to HTML with additional [options](`Opts`).
pub fn render_with_opts(input: &str, opts: Opts) -> Result<String> {
    KATEX.with(|ctx| {
        let ctx = match ctx.as_ref() {
            Ok(ctx) => ctx,
            Err(e) => return Err(e.clone()),
        };
        let args: Vec<JsValue> = vec![input.into(), opts.into()];
        let result = ctx.call_function("renderToString", args)?;
        result.into_string().ok_or_else(|| Error::InvalidResult)
    })
}

/// Render LaTeX equation to HTML.
#[inline]
pub fn render(input: &str) -> Result<String> {
    render_with_opts(input, Default::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render() {
        let html = render("a = b + c").unwrap();
        assert!(!html.contains(r#"span class="katex-display""#));
        assert!(html.contains(r#"span class="katex""#));
        assert!(html.contains(r#"span class="katex-mathml""#));
        assert!(html.contains(r#"span class="katex-html""#));
    }

    #[test]
    fn test_render_in_display_mode() {
        let opts = Opts::builder().display_mode(true).build().unwrap();
        let html = render_with_opts("a = b + c", opts).unwrap();
        assert!(html.contains(r#"span class="katex-display""#));
    }

    #[test]
    fn test_output_html_only() {
        let opts = Opts::builder()
            .output_type(OutputType::Html)
            .build()
            .unwrap();
        let html = render_with_opts("a = b + c", opts).unwrap();
        assert!(!html.contains(r#"span class="katex-mathml""#));
        assert!(html.contains(r#"span class="katex-html""#));
    }

    #[test]
    fn test_output_mathml_only() {
        let opts = Opts::builder()
            .output_type(OutputType::Mathml)
            .build()
            .unwrap();
        let html = render_with_opts("a = b + c", opts).unwrap();
        assert!(html.contains(r#"MathML"#));
        assert!(!html.contains(r#"span class="katex-html""#));
    }

    #[test]
    fn test_leqno() {
        let opts = Opts::builder()
            .display_mode(true)
            .leqno(true)
            .build()
            .unwrap();
        let html = render_with_opts("a = b + c", opts).unwrap();
        assert!(html.contains(r#"span class="katex-display leqno""#));
    }

    #[test]
    fn test_fleqn() {
        let opts = Opts::builder()
            .display_mode(true)
            .fleqn(true)
            .build()
            .unwrap();
        let html = render_with_opts("a = b + c", opts).unwrap();
        assert!(html.contains(r#"span class="katex-display fleqn""#));
    }

    #[test]
    fn test_throw_on_error() {
        let err_msg = match render(r#"\"#) {
            Ok(_) => unreachable!(),
            Err(e) => match e {
                Error::JsExecError(msg) => msg,
                _ => unreachable!(),
            },
        };
        assert!(err_msg.contains("ParseError"));
    }

    #[test]
    fn test_error_color() {
        let opts = Opts::builder()
            .throw_on_error(false)
            .error_color("#ff0000")
            .build()
            .unwrap();
        let html = render_with_opts(r#"\"#, opts).unwrap();
        assert!(html.contains(r#"span class="katex-error""#));
        assert!(html.contains("#ff0000"));
    }

    #[test]
    fn test_macros() {
        let opts = Opts::builder()
            .add_macro(r#"\RR"#.to_owned(), r#"\mathbb{R}"#.to_owned())
            .build()
            .unwrap();
        let html = render_with_opts(r#"\RR"#, opts).unwrap();
        assert!(html.contains("mathbb"));
    }
}
