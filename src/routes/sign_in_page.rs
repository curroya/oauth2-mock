use crate::users::User;

const STYLE: &str = r#"
:root { color-scheme: light dark; --border: #8884; --accent: #2563eb; }
body { font: 16px/1.5 system-ui, sans-serif; margin: 0; display: grid; place-items: center; min-height: 100vh; }
main { width: min(28rem, calc(100vw - 2rem)); }
h1 { font-size: 1.25rem; margin: 0 0 1rem; }
h2 { font-size: .8rem; text-transform: uppercase; letter-spacing: .05em; opacity: .7; margin: 1.5rem 0 .5rem; }
.users { display: grid; gap: .5rem; }
.user { display: grid; text-align: left; padding: .75rem 1rem; border: 1px solid var(--border); border-radius: .5rem; background: none; color: inherit; font: inherit; cursor: pointer; }
.user:hover, .user:focus-visible { border-color: var(--accent); }
.user small { opacity: .6; font-family: ui-monospace, monospace; }
.new-user { display: flex; gap: .5rem; }
.new-user input { flex: 1; padding: .5rem .75rem; border: 1px solid var(--border); border-radius: .5rem; font: inherit; }
.new-user button { padding: .5rem 1rem; border: 0; border-radius: .5rem; background: var(--accent); color: white; font: inherit; cursor: pointer; }
"#;

pub fn render_sign_in_page(forwarded_fields: &[(&str, &str)], users: &[User]) -> String {
    let hidden_inputs = hidden_inputs(forwarded_fields);
    let known_users = known_users_section(&hidden_inputs, users);
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>oauth2-mock sign in</title>
<style>{STYLE}</style>
</head>
<body>
<main>
<h1>Sign in</h1>
{known_users}
<h2>Another email</h2>
<form method="post" class="new-user">
{hidden_inputs}
<input type="email" name="login_hint" aria-label="Email" placeholder="email@example.com" required autofocus>
<button type="submit">Sign in</button>
</form>
</main>
</body>
</html>"#
    )
}

fn known_users_section(hidden_inputs: &str, users: &[User]) -> String {
    if users.is_empty() {
        return String::new();
    }
    let user_buttons: Vec<String> = users.iter().map(user_button).collect();
    let buttons = user_buttons.join("\n");
    format!(
        r#"<h2>Users</h2>
<form method="post" class="users">
{hidden_inputs}
{buttons}
</form>"#
    )
}

fn user_button(user: &User) -> String {
    format!(
        r#"<button type="submit" class="user" name="login_hint" value="{email}"><strong>{name}</strong><span>{email}</span><small>{id}</small></button>"#,
        email = escape_html(&user.email),
        name = escape_html(&user.name),
        id = escape_html(&user.id)
    )
}

fn hidden_inputs(fields: &[(&str, &str)]) -> String {
    let inputs: Vec<String> = fields
        .iter()
        .map(|(name, value)| hidden_input(name, value))
        .collect();
    inputs.join("\n")
}

fn hidden_input(name: &str, value: &str) -> String {
    format!(
        r#"<input type="hidden" name="{name}" value="{}">"#,
        escape_html(value)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
