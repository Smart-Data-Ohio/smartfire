//! Owned GitHub sections embedded in the WS8b-r2 profile and WS11-ui bot page.
//! Public display data only; request-local tokens come from the shared form helpers.
use crate::helpers as h;
fn disconnect(path: &str, confirmation: &str) -> String {
    h::button_to_form(
        path,
        h::attrs().method("delete").class("btn btn--negative"),
        h::attrs().data("turbo-confirm", confirmation),
        "Disconnect GitHub",
    )
    .0
}
fn connect_form(path: &str, linked: bool, indent: &str) -> String {
    let form = h::form_with(path).class("flex align-center gap");
    let field = form.password_field(
        "access_token",
        h::attrs()
            .class("input flex-item-grow")
            .attr("placeholder", "github_pat_…")
            .attr("autocomplete", "off")
            .attr("required", true)
            .attr("aria-label", "GitHub personal access token"),
    );
    let label = if linked {
        "Reconnect GitHub"
    } else {
        "Connect GitHub"
    };
    let submit = h::legacy_tag(
        "input",
        h::attrs()
            .type_("submit")
            .name("commit")
            .value(label)
            .class("btn")
            .data("disable-with", label),
    );
    form.wrap(&format!("\n{indent}{field}\n{indent}{submit}\n"))
        .0
}
/// `users/profiles/_github_connection.html.erb`, including its trailing newline.
pub fn profile(data: &Connection) -> String {
    let mut html = String::from(
        "<section class=\"margin-block pad-inline pad-block fill-shade border-radius\" aria-labelledby=\"github-connection-title\">\n  <h2 id=\"github-connection-title\" class=\"txt-large\">GitHub</h2>\n\n\n",
    );
    if data.usable {
        html.push_str(&format!("    <p>Connected as {}{}</p>\n    <p>Comments and reviews you post from PR threads act as <strong>@{}</strong> on GitHub. Your linked account also unlocks cards for private repositories it can read.</p>\n    {}\n",h::escape(&data.login),if data.app_token {" through the GitHub App"} else {""},h::escape(&data.login),disconnect("/github/connection","Disconnect GitHub? You will no longer be able to comment or review from PR threads.")));
    } else {
        if data.linked {
            html.push_str(&format!(
                "      <p>GitHub rejected the connection{}. Reconnect below.</p>\n",
                reason(data)
            ));
        } else {
            html.push_str("      <p>Connect GitHub to comment and review from PR threads as yourself. Linking also shows you cards for private repositories your account can read.</p>\n");
        }
        if data.app_configured {
            html.push_str(&format!("      <p><a class=\"btn btn--primary\" href=\"/github/app/connect\">{} with GitHub</a></p>\n      <details>\n        <summary>Or paste a personal access token instead</summary>\n",if data.linked {"Reconnect"} else {"Connect"}));
        }
        html.push_str(&format!("    {}    <p class=\"txt-small\">The token needs <strong>Pull requests: Read and write</strong>, <strong>Issues: Read and write</strong>, and <strong>Metadata: Read</strong>. It is validated with GitHub before it is stored and is never shown again.</p>\n",connect_form("/github/connection",data.linked,"      ")));
        if data.app_configured {
            html.push_str("      </details>\n");
        }
    }
    html.push_str("</section>\n");
    html
}
/// Inline GitHub section from `accounts/bots/edit.html.erb`. Caller supplies viewer policy.
pub fn bot(data: &Connection, bot_id: i64, administrator: bool) -> String {
    let mut html = String::from(
        "  <section aria-labelledby=\"github-connection-title\">\n    <h2 id=\"github-connection-title\" class=\"txt-large margin-block-end\">GitHub account</h2>\n\n\n",
    );
    let path = format!("/account/bots/{bot_id}/github_connection");
    if data.usable {
        html.push_str(&format!("      <p>Connected as {}</p>\n      <p>Approved write actions post from PR threads as <strong>@{}</strong> on GitHub.</p>\n",h::escape(&data.login),h::escape(&data.login)));
        if administrator {
            html.push_str(&format!("      {}\n",disconnect(&path,"Disconnect GitHub? This agent will no longer be able to request PR write actions.")));
        }
    } else if !administrator {
        html.push_str(
            "      <p>No GitHub account is connected. Only an administrator can connect one.</p>\n",
        );
    } else {
        if data.linked {
            html.push_str(&format!("        <p>GitHub rejected the connection{}. Paste a new token to reconnect.</p>\n",reason(data)));
        } else {
            html.push_str("        <p>Paste a fine-grained personal access token for a machine user dedicated to this agent, with <strong>Pull requests: Read and write</strong>, <strong>Issues: Read and write</strong>, and <strong>Metadata: Read</strong>. Approved write actions post as that user. The token is validated with GitHub before it is stored and is never shown again.</p>\n");
        }
        html.push_str(&format!(
            "      {}",
            connect_form(&path, data.linked, "        ")
        ));
    }
    html.push_str("  </section>\n");
    html
}
pub use campfire_presentation::github::connections::*;
