use super::*;

#[test]
fn auth_shell_uses_standalone_assets_and_preserves_flash_precedence() {
    let asset = |path: &str| campfire_assets::asset_path(path);
    let signer = |_parts: &[&str]| String::new();
    let mut ctx = context(None, &asset, &signer, "");
    ctx.custom_styles = Some(".auth { --example: 1; }".into());
    ctx.flash_notice = Some("Connected <&> successfully.".into());
    ctx.flash_alert = Some("An alert that loses to the notice".into());
    let page = layouts::Auth {
        ctx: &ctx,
        page_title: Some("Sign in <&>".into()),
        head: h::empty(),
        content: h::raw("<p>Content</p>"),
    };
    let html = page.render().unwrap();
    assert!(html.contains("<html lang=\"en\""));
    assert!(html.contains("<meta charset=\"utf-8\">"));
    assert!(html.contains("<meta name=\"viewport\""));
    assert!(html.contains("<meta name=\"color-scheme\""));
    assert!(html.contains("<title>Sign in &lt;&amp;&gt;</title>"));
    assert!(html.contains("rel=\"icon\""));
    assert!(html.contains("rel=\"apple-touch-icon\""));
    assert!(html.contains(".auth { --example: 1; }"));
    assert!(html.contains("role=\"status\""));
    form_contracts::assert_text(&html, "Connected <&> successfully.");
    assert!(html.contains("Connected &lt;&amp;&gt; successfully."));
    assert!(!html.contains("An alert that loses"));
    let css = format!(
        "<link rel=\"stylesheet\" href=\"{}\">",
        campfire_assets::asset_path("auth.css")
    );
    let script = format!(
        "<script src=\"{}\"></script>",
        campfire_assets::asset_path("auth.js")
    );
    assert_eq!(h::auth_stylesheet_tag().0, css);
    assert_eq!(h::auth_script_tag().0, script);
    assert!(html.contains(&css));
    assert!(html.contains(&script));
    assert!(html.find(&script).unwrap() < html.find("</head>").unwrap());
    assert!(!html.contains("importmap"));
    assert!(!html.contains("element-removal"));
    ctx.flash_notice = None;
    ctx.flash_alert = Some("Failed <&>. Try again.".into());
    let html = layouts::Auth {
        ctx: &ctx,
        page_title: None,
        head: h::empty(),
        content: h::empty(),
    }
    .render()
    .unwrap();
    assert!(html.contains("role=\"alert\""));
    form_contracts::assert_text(&html, "Failed <&>. Try again.");
    assert!(html.contains("Failed &lt;&amp;&gt;. Try again."));
}
